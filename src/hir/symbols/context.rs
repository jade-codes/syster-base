//! Extraction context — tracks scope, file, and line index during extraction.

use crate::base::FileId;

use super::types::SpanInfo;

/// Extraction state passed through all extraction functions.
pub(super) struct ExtractionContext {
    pub file: FileId,
    pub prefix: String,
    /// Counter for generating unique anonymous scope names
    pub anon_counter: u32,
    /// Stack of scope segments for proper push/pop
    pub scope_stack: Vec<String>,
    /// Line index for converting byte offsets to line/column
    pub line_index: crate::base::LineIndex,
}

impl ExtractionContext {
    pub fn qualified_name(&self, name: &str) -> String {
        if self.prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", self.prefix, name)
        }
    }

    /// Get the current scope name (the prefix without trailing ::)
    pub fn current_scope_name(&self) -> String {
        self.prefix.clone()
    }

    pub fn push_scope(&mut self, name: &str) {
        self.scope_stack.push(name.to_string());
        if self.prefix.is_empty() {
            self.prefix = name.to_string();
        } else {
            self.prefix = format!("{}::{}", self.prefix, name);
        }
    }

    pub fn pop_scope(&mut self) {
        if let Some(popped) = self.scope_stack.pop() {
            // Remove the last segment (which may contain ::) plus the joining ::
            let suffix_len = if self.scope_stack.is_empty() {
                popped.len()
            } else {
                popped.len() + 2 // +2 for the "::" separator
            };
            self.prefix
                .truncate(self.prefix.len().saturating_sub(suffix_len));
        }
    }

    /// Generate a unique anonymous scope name
    pub fn next_anon_scope(&mut self, rel_prefix: &str, target: &str, line: u32) -> String {
        self.anon_counter += 1;
        format!("<{}{}#{}@L{}>", rel_prefix, target, self.anon_counter, line)
    }

    /// Convert a TextRange to SpanInfo using the line index
    pub fn range_to_info(&self, range: Option<rowan::TextRange>) -> SpanInfo {
        match range {
            Some(r) => {
                let start = self.line_index.line_col(r.start());
                let end = self.line_index.line_col(r.end());
                SpanInfo {
                    start_line: start.line,
                    start_col: start.col,
                    end_line: end.line,
                    end_col: end.col,
                }
            }
            None => SpanInfo::default(),
        }
    }

    /// Convert a TextRange to optional line/col values (for short_name fields)
    pub fn range_to_optional(
        &self,
        range: Option<rowan::TextRange>,
    ) -> (Option<u32>, Option<u32>, Option<u32>, Option<u32>) {
        match range {
            Some(r) => {
                let start = self.line_index.line_col(r.start());
                let end = self.line_index.line_col(r.end());
                (
                    Some(start.line),
                    Some(start.col),
                    Some(end.line),
                    Some(end.col),
                )
            }
            None => (None, None, None, None),
        }
    }
}

/// Strip single quotes from a string.
pub(super) fn strip_quotes(s: &str) -> String {
    if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2 {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}
