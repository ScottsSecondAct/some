use anyhow::{Context, Result};
use memmap2::Mmap;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GitChange {
    Added,
    Modified,
    Deleted,
}

/// Holds file contents and provides efficient random line access.
pub struct Buffer {
    source: BufferSource,
    /// Byte offset of the start of each line
    line_offsets: Vec<usize>,
    /// Original file path (None for stdin)
    pub path: Option<PathBuf>,
    /// Display name for the status bar
    pub name: String,
    /// Git change indicators per line (0-indexed)
    pub git_changes: HashMap<usize, GitChange>,
    /// True when this buffer is a synthetic unified diff
    pub is_diff: bool,
}

enum BufferSource {
    Mmap(Mmap),
    Memory(Vec<u8>),
}

impl BufferSource {
    fn as_bytes(&self) -> &[u8] {
        match self {
            BufferSource::Mmap(m) => m.as_ref(),
            BufferSource::Memory(v) => v.as_slice(),
        }
    }
}

// ── Decompression helpers ───────────────────────────────────────────────────

fn decompress_gz(path: &Path) -> Result<Vec<u8>> {
    let file = File::open(path).with_context(|| format!("Cannot open '{}'", path.display()))?;
    let mut decoder = flate2::read::GzDecoder::new(file);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).with_context(|| format!("Failed to decompress '{}'", path.display()))?;
    Ok(out)
}

fn decompress_zst(path: &Path) -> Result<Vec<u8>> {
    let file = File::open(path).with_context(|| format!("Cannot open '{}'", path.display()))?;
    let mut decoder = zstd::stream::read::Decoder::new(file)
        .with_context(|| format!("Failed to init zstd decoder for '{}'", path.display()))?;
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).with_context(|| format!("Failed to decompress '{}'", path.display()))?;
    Ok(out)
}

fn decompress_bz2(path: &Path) -> Result<Vec<u8>> {
    let file = File::open(path).with_context(|| format!("Cannot open '{}'", path.display()))?;
    let mut decoder = bzip2::read::BzDecoder::new(file);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).with_context(|| format!("Failed to decompress '{}'", path.display()))?;
    Ok(out)
}

fn decompress_if_needed(path: &Path) -> Result<Option<Vec<u8>>> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("gz")              => Ok(Some(decompress_gz(path)?)),
        Some("zst") | Some("zstd") => Ok(Some(decompress_zst(path)?)),
        Some("bz2")             => Ok(Some(decompress_bz2(path)?)),
        _                       => Ok(None),
    }
}

// ── Git diff parsing ────────────────────────────────────────────────────────

fn parse_git_changes(stdout: &[u8]) -> HashMap<usize, GitChange> {
    let mut changes: HashMap<usize, GitChange> = HashMap::new();
    let text = match std::str::from_utf8(stdout) {
        Ok(t) => t,
        Err(_) => return changes,
    };

    for line in text.lines() {
        if !line.starts_with("@@") {
            continue;
        }
        // Parse @@ -old[,count] +new[,count] @@
        // Example: @@ -10,5 +10,7 @@
        let rest = &line[2..];
        let end = rest.find("@@").unwrap_or(rest.len());
        let hunk_header = rest[..end].trim();

        // Split into old/new parts
        let parts: Vec<&str> = hunk_header.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let new_part = parts.iter().find(|p| p.starts_with('+'));
        if let Some(new_range) = new_part {
            let range_str = &new_range[1..]; // strip leading '+'
            let (start, count) = parse_range(range_str);
            if count == 0 {
                // Deletion at line `start`
                if start > 0 {
                    changes.entry(start - 1).or_insert(GitChange::Deleted);
                }
            } else {
                let old_part = parts.iter().find(|p| p.starts_with('-'));
                let (_, old_count) = old_part
                    .map(|p| parse_range(&p[1..]))
                    .unwrap_or((0, 0));

                let tag = if old_count == 0 {
                    GitChange::Added
                } else {
                    GitChange::Modified
                };
                for line_idx in start..(start + count) {
                    if line_idx > 0 {
                        changes.entry(line_idx - 1).or_insert(tag);
                    }
                }
            }
        }
    }
    changes
}

fn parse_range(s: &str) -> (usize, usize) {
    if let Some(comma) = s.find(',') {
        let start = s[..comma].parse().unwrap_or(1);
        let count = s[comma + 1..].parse().unwrap_or(1);
        (start, count)
    } else {
        let start = s.parse().unwrap_or(1);
        (start, 1)
    }
}

// ── Buffer impl ─────────────────────────────────────────────────────────────

impl Buffer {
    /// Load a file into a buffer. Uses mmap for files above the threshold.
    /// Transparently decompresses .gz/.zst/.bz2 files.
    pub fn from_file(path: &Path, mmap_threshold: u64) -> Result<Self> {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());

        // Attempt transparent decompression
        if let Some(data) = decompress_if_needed(path)? {
            let line_offsets = Self::index_lines(&data);
            return Ok(Self {
                source: BufferSource::Memory(data),
                line_offsets,
                path: Some(path.to_path_buf()),
                name,
                git_changes: HashMap::new(),
                is_diff: false,
            });
        }

        let metadata = std::fs::metadata(path)
            .with_context(|| format!("Cannot stat '{}'", path.display()))?;
        let file_size = metadata.len();

        let source = if file_size >= mmap_threshold {
            let file = File::open(path)
                .with_context(|| format!("Cannot open '{}'", path.display()))?;
            let mmap = unsafe { Mmap::map(&file) }
                .with_context(|| format!("Cannot mmap '{}'", path.display()))?;
            BufferSource::Mmap(mmap)
        } else {
            let mut file = File::open(path)
                .with_context(|| format!("Cannot open '{}'", path.display()))?;
            let mut contents = Vec::with_capacity(file_size as usize);
            file.read_to_end(&mut contents)?;
            BufferSource::Memory(contents)
        };

        let line_offsets = Self::index_lines(source.as_bytes());

        Ok(Self {
            source,
            line_offsets,
            path: Some(path.to_path_buf()),
            name,
            git_changes: HashMap::new(),
            is_diff: false,
        })
    }

    /// Load from stdin into an in-memory buffer.
    pub fn from_stdin() -> Result<Self> {
        let mut contents = Vec::new();
        std::io::stdin()
            .read_to_end(&mut contents)
            .context("Failed to read from stdin")?;
        let line_offsets = Self::index_lines(&contents);
        Ok(Self {
            source: BufferSource::Memory(contents),
            line_offsets,
            path: None,
            name: "[stdin]".to_string(),
            git_changes: HashMap::new(),
            is_diff: false,
        })
    }

    /// Create a synthetic unified diff buffer comparing two files.
    pub fn from_diff(file_a: &Path, file_b: &Path) -> Result<Self> {
        let text_a = std::fs::read_to_string(file_a)
            .with_context(|| format!("Cannot read '{}'", file_a.display()))?;
        let text_b = std::fs::read_to_string(file_b)
            .with_context(|| format!("Cannot read '{}'", file_b.display()))?;

        let diff = similar::TextDiff::from_lines(&text_a, &text_b);

        let mut out = format!("--- {}\n+++ {}\n", file_a.display(), file_b.display());
        for group in diff.grouped_ops(3) {
            // Emit @@ header
            let first_op = &group[0];
            let last_op = &group[group.len() - 1];
            let old_start = first_op.old_range().start + 1;
            let old_len: usize = group.iter().map(|op| op.old_range().len()).sum();
            let new_start = first_op.new_range().start + 1;
            let new_len: usize = group.iter().map(|op| op.new_range().len()).sum();
            let _ = last_op; // suppress unused warning
            use std::fmt::Write as _;
            writeln!(out, "@@ -{},{} +{},{} @@", old_start, old_len, new_start, new_len).ok();
            for op in &group {
                for change in diff.iter_changes(op) {
                    let prefix = match change.tag() {
                        similar::ChangeTag::Delete => '-',
                        similar::ChangeTag::Insert => '+',
                        similar::ChangeTag::Equal  => ' ',
                    };
                    write!(out, "{}{}", prefix, change.value()).ok();
                }
            }
        }

        let data = out.into_bytes();
        let line_offsets = Self::index_lines(&data);
        let name = format!(
            "{} → {}",
            file_a.file_name().unwrap_or_default().to_string_lossy(),
            file_b.file_name().unwrap_or_default().to_string_lossy()
        );

        Ok(Self {
            source: BufferSource::Memory(data),
            line_offsets,
            path: None,
            name,
            git_changes: HashMap::new(),
            is_diff: true,
        })
    }

    /// Build an index of byte offsets for the start of each line.
    fn index_lines(data: &[u8]) -> Vec<usize> {
        if data.is_empty() {
            return vec![];
        }
        let mut offsets = vec![0usize];
        for (i, &byte) in data.iter().enumerate() {
            if byte == b'\n' && i + 1 < data.len() {
                offsets.push(i + 1);
            }
        }
        offsets
    }

    /// Total number of lines in the buffer.
    pub fn line_count(&self) -> usize {
        self.line_offsets.len()
    }

    /// Number of hex dump lines (16 bytes per row).
    pub fn hex_line_count(&self) -> usize {
        let len = self.source.as_bytes().len();
        len.div_ceil(16)
    }

    /// Line count used by the viewport (hex or text depending on content).
    pub fn display_line_count(&self) -> usize {
        if self.is_binary() {
            self.hex_line_count()
        } else {
            self.line_count()
        }
    }

    /// Render row `n` of a hex dump (16 bytes per row).
    pub fn hex_line(&self, n: usize) -> String {
        let data = self.source.as_bytes();
        let start = n * 16;
        if start >= data.len() {
            return String::new();
        }
        let end = (start + 16).min(data.len());
        let chunk = &data[start..end];

        let mut hex_parts = String::new();
        for (i, b) in chunk.iter().enumerate() {
            if i == 8 {
                hex_parts.push(' ');
            }
            if i > 0 && i != 8 {
                hex_parts.push(' ');
            }
            hex_parts.push_str(&format!("{:02x}", b));
        }
        // Pad to full width (16 bytes = 47 chars + 1 extra space for middle gap)
        let ascii: String = chunk.iter()
            .map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { '.' })
            .collect();

        format!("{:08x}  {:<48} |{}|", start, hex_parts, ascii)
    }

    /// Get the text content of line `n` (0-indexed), without trailing newline.
    pub fn get_line(&self, n: usize) -> Option<&str> {
        if n >= self.line_offsets.len() {
            return None;
        }
        let data = self.source.as_bytes();
        let start = self.line_offsets[n];
        let end = if n + 1 < self.line_offsets.len() {
            self.line_offsets[n + 1]
        } else {
            data.len()
        };

        let mut slice = &data[start..end];
        if slice.last() == Some(&b'\n') {
            slice = &slice[..slice.len() - 1];
        }
        if slice.last() == Some(&b'\r') {
            slice = &slice[..slice.len() - 1];
        }

        std::str::from_utf8(slice).ok()
    }

    /// Clone all lines into owned strings (for async search snapshot).
    pub fn text_snapshot(&self) -> Vec<String> {
        (0..self.line_count())
            .filter_map(|i| self.get_line(i).map(str::to_string))
            .collect()
    }

    /// Reload the buffer from disk (no-op for stdin). Re-decompresses if needed.
    pub fn reload(&mut self, mmap_threshold: u64) -> anyhow::Result<()> {
        let path = match &self.path {
            Some(p) => p.clone(),
            None => return Ok(()),
        };

        // Re-decompress if this is a compressed file
        if let Some(data) = decompress_if_needed(&path)? {
            self.line_offsets = Self::index_lines(&data);
            self.source = BufferSource::Memory(data);
            return Ok(());
        }

        let metadata = std::fs::metadata(&path)
            .with_context(|| format!("Cannot stat '{}'", path.display()))?;
        let file_size = metadata.len();
        let source = if file_size >= mmap_threshold {
            let file = File::open(&path)
                .with_context(|| format!("Cannot open '{}'", path.display()))?;
            let mmap = unsafe { Mmap::map(&file) }
                .with_context(|| format!("Cannot mmap '{}'", path.display()))?;
            BufferSource::Mmap(mmap)
        } else {
            let mut file = File::open(&path)
                .with_context(|| format!("Cannot open '{}'", path.display()))?;
            let mut contents = Vec::with_capacity(file_size as usize);
            file.read_to_end(&mut contents)?;
            BufferSource::Memory(contents)
        };
        self.line_offsets = Self::index_lines(source.as_bytes());
        self.source = source;
        Ok(())
    }

    /// Check if the file appears to be binary.
    pub fn is_binary(&self) -> bool {
        let data = self.source.as_bytes();
        let check_len = std::cmp::min(data.len(), 8192);
        data[..check_len].contains(&0)
    }

    /// Shell out to `git diff HEAD` and populate `git_changes`.
    pub fn load_git_changes(&mut self) {
        let path = match &self.path {
            Some(p) => p.clone(),
            None => return,
        };
        let parent = path.parent().unwrap_or(Path::new("."));
        let result = std::process::Command::new("git")
            .args(["diff", "HEAD", "--unified=0", "--"])
            .arg(&path)
            .current_dir(parent)
            .output();

        if let Ok(output) = result {
            if output.status.success() || !output.stdout.is_empty() {
                self.git_changes = parse_git_changes(&output.stdout);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_buffer(data: &[u8]) -> Buffer {
        let data = data.to_vec();
        let offsets = Buffer::index_lines(&data);
        Buffer {
            source: BufferSource::Memory(data),
            line_offsets: offsets,
            path: None,
            name: "test".to_string(),
            git_changes: HashMap::new(),
            is_diff: false,
        }
    }

    #[test]
    fn test_empty() {
        let buf = make_buffer(b"");
        assert_eq!(buf.line_count(), 0);
        assert_eq!(buf.get_line(0), None);
    }

    #[test]
    fn test_single_line() {
        let buf = make_buffer(b"hello world");
        assert_eq!(buf.line_count(), 1);
        assert_eq!(buf.get_line(0), Some("hello world"));
    }

    #[test]
    fn test_multiple_lines() {
        let buf = make_buffer(b"one\ntwo\nthree\n");
        assert_eq!(buf.line_count(), 3);
        assert_eq!(buf.get_line(0), Some("one"));
        assert_eq!(buf.get_line(1), Some("two"));
        assert_eq!(buf.get_line(2), Some("three"));
    }

    #[test]
    fn test_crlf() {
        let buf = make_buffer(b"first\r\nsecond\r\n");
        assert_eq!(buf.get_line(0), Some("first"));
        assert_eq!(buf.get_line(1), Some("second"));
    }

    #[test]
    fn test_binary() {
        let buf = make_buffer(b"hello\x00world");
        assert!(buf.is_binary());
    }

    #[test]
    fn test_hex_line() {
        let buf = make_buffer(b"ABCDEFGHIJKLMNOP");
        let line = buf.hex_line(0);
        assert!(line.starts_with("00000000"));
        assert!(line.contains("41 42 43 44"));
        assert!(line.contains("|ABCDEFGHIJKLMNOP|"));
    }

    #[test]
    fn test_hex_line_count() {
        let buf = make_buffer(&[0u8; 32]);
        assert_eq!(buf.hex_line_count(), 2);
        let buf2 = make_buffer(&[0u8; 17]);
        assert_eq!(buf2.hex_line_count(), 2);
    }
}
