//! Which lines of a log a filter lists: those that say a text or match a pattern, of the
//! levels that are lit.

use std::ops::Range;

use regex_lite::{Regex, RegexBuilder};

use super::Level;
use crate::ansi::plain;

pub(super) struct Listing {
    /// What a line must say: a text (escaped) or a pattern, whatever the case; none: any.
    pub(super) matcher: Option<Regex>,
    /// The lines that do not match are listed too, faint.
    pub(super) dim: bool,
    /// The levels a line may be of, a bit each; 0: any.
    pub(super) levels: u8,
    /// The lines that match, when only those are listed.
    pub(super) index: Vec<u32>,
}

impl Listing {
    pub(super) fn new() -> Listing {
        Listing { matcher: None, dim: false, levels: 0, index: Vec::new() }
    }

    pub(super) fn indexed(&self) -> bool {
        !self.dim && (self.matcher.is_some() || self.levels != 0)
    }

    pub(super) fn accepts(&self, line: &str, level: Level) -> bool {
        let leveled = self.levels == 0 || self.levels & bit(level) != 0;
        leveled && self.matcher.as_ref().is_none_or(|matcher| matcher.is_match(&plain(line)))
    }

    /// Where the filter's text or pattern is in what a line says (without its escapes).
    pub(super) fn marks(&self, line: &str) -> Vec<Range<usize>> {
        let Some(matcher) = &self.matcher else { return Vec::new() };
        matcher.find_iter(&plain(line)).filter(|m| !m.is_empty()).map(|m| m.range()).collect()
    }
}

/// The bit of a level in a set; fatal is counted as error.
pub(super) fn bit(level: Level) -> u8 {
    let level = if level == Level::Fatal { Level::Error } else { level };
    1 << level as u8
}

/// A query as a matcher: a text found whatever its case, or with `pattern` a regular
/// expression; none for an empty query. An error: the pattern is not one.
pub(super) fn matcher(query: &str, pattern: bool) -> Result<Option<Regex>, regex_lite::Error> {
    if query.is_empty() {
        return Ok(None);
    }
    let source = if pattern { query.to_string() } else { regex_lite::escape(query) };
    RegexBuilder::new(&source).case_insensitive(true).build().map(Some)
}
