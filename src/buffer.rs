use anyhow::{Context, Result};
use memmap2::Mmap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Holds file contents and provides efficient random line access.
pub struct Buffer {
    source: BufferSource,
    /// Byte offset of the start of each line
    line_offsets: Vec<usize>,
    /// Original file path (None for stdin)
    pub path: Option<PathBuf>,
    /// Display name for the status bar
    pub name: String,
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

impl Buffer {
    /// Load a file into a buffer. Uses mmap for files above the threshold.
    pub fn from_file(path: &Path, mmap_threshold: u64) -> Result<Self> {
        let metadata = std::fs::metadata(path)
            .with_context(|| format!("Cannot stat '{}'", path.display()))?;

        let file_size = metadata.len();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());

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

    /// Check if the file appears to be binary.
    pub fn is_binary(&self) -> bool {
        let data = self.source.as_bytes();
        let check_len = std::cmp::min(data.len(), 8192);
        data[..check_len].contains(&0)
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
}
