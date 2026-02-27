use anyhow::Result;
use regex::{Regex, RegexBuilder};
use std::ops::Range;

/// Tracks search state: pattern, all matches, current position.
pub struct SearchState {
    pub pattern: Option<Regex>,
    pub query_string: String,
    pub matches: Vec<(usize, Range<usize>)>,
    pub current: usize,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            pattern: None,
            query_string: String::new(),
            matches: Vec::new(),
            current: 0,
        }
    }

    /// Compile a search pattern with smart case.
    pub fn set_pattern(&mut self, query: &str, smart_case: bool) -> Result<()> {
        self.query_string = query.to_string();
        if query.is_empty() {
            self.pattern = None;
            self.matches.clear();
            return Ok(());
        }
        let case_insensitive = smart_case && !query.chars().any(|c| c.is_uppercase());
        let regex = RegexBuilder::new(query)
            .case_insensitive(case_insensitive)
            .build()?;
        self.pattern = Some(regex);
        Ok(())
    }

    /// Run search across all lines.
    pub fn search_buffer(&mut self, buffer: &crate::buffer::Buffer) {
        self.matches.clear();
        self.current = 0;
        let regex = match &self.pattern {
            Some(r) => r,
            None => return,
        };
        for line_idx in 0..buffer.line_count() {
            if let Some(text) = buffer.get_line(line_idx) {
                for mat in regex.find_iter(text) {
                    self.matches.push((line_idx, mat.start()..mat.end()));
                }
            }
        }
    }

    pub fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.current = (self.current + 1) % self.matches.len();
        }
    }

    pub fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            self.current = if self.current == 0 {
                self.matches.len() - 1
            } else {
                self.current - 1
            };
        }
    }

    pub fn jump_to_line(&mut self, line: usize) {
        if let Some(idx) = self.matches.iter().position(|(l, _)| *l >= line) {
            self.current = idx;
        }
    }

    pub fn current_match_line(&self) -> Option<usize> {
        self.matches.get(self.current).map(|(line, _)| *line)
    }

    pub fn matches_on_line(&self, line: usize) -> Vec<Range<usize>> {
        self.matches
            .iter()
            .filter(|(l, _)| *l == line)
            .map(|(_, r)| r.clone())
            .collect()
    }

    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    pub fn has_pattern(&self) -> bool {
        self.pattern.is_some()
    }
}
