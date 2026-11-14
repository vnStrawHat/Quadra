//! Finding in a document (behaviour/18, "Find"): a query with its switches, the places it
//! matches, and replacing them.

use std::ops::Range;

use regex_lite::Regex;

use crate::doc::{Doc, Edit};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Query {
    pub text: String,
    pub case: bool,
    pub word: bool,
    pub regex: bool,
}

impl Query {
    /// The pattern of the query, or what is wrong with it. A letter matches its other case
    /// unless `case` (as a class of both, so the offsets are those of the text).
    fn pattern(&self) -> Result<Regex, String> {
        let mut source = String::new();
        if self.regex {
            if !self.case {
                source.push_str("(?i)");
            }
            source.push_str(&self.text);
        } else {
            for c in self.text.chars() {
                let (lower, upper) = (one(c.to_lowercase()), one(c.to_uppercase()));
                match (lower, upper) {
                    (Some(l), Some(u)) if !self.case && l != u => {
                        source.push('[');
                        source.push_str(&regex_lite::escape(&l.to_string()));
                        source.push_str(&regex_lite::escape(&u.to_string()));
                        source.push(']');
                    }
                    _ => source.push_str(&regex_lite::escape(&c.to_string())),
                }
            }
        }
        Regex::new(&source).map_err(|e| e.to_string())
    }

    /// Every place the query matches in the document, in order; none for an empty query.
    /// A whole word has no letter, digit or `_` on either side.
    pub fn matches(&self, doc: &Doc) -> Result<Vec<Range<usize>>, String> {
        if self.text.is_empty() {
            return Ok(Vec::new());
        }
        let pattern = self.pattern()?;
        // ponytail: the whole text is copied a search; search piece by piece if files of
        // tens of megabytes make it slow.
        let text = doc.text();
        let wordy = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
        Ok(pattern
            .find_iter(&text)
            .map(|m| m.range())
            .filter(|r| !r.is_empty())
            .filter(|r| {
                !self.word
                    || (!wordy(text[..r.start].chars().next_back()) && !wordy(text[r.end..].chars().next()))
            })
            .collect())
    }
}

fn one(mut chars: impl Iterator<Item = char>) -> Option<char> {
    let c = chars.next();
    if chars.next().is_some() { None } else { c }
}

/// The match at or after `offset`, round to the first; `back` takes the one before it.
pub fn next_match(matches: &[Range<usize>], offset: usize, back: bool) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    let after = matches.partition_point(|m| m.start < offset);
    Some(if back { (after + matches.len() - 1) % matches.len() } else { after % matches.len() })
}

impl Doc {
    /// Puts `with` in place of the match `range`; the cursors are taken through it.
    pub fn replace(&mut self, range: Range<usize>, with: &str) {
        self.break_group();
        self.apply(vec![Edit { range, text: with.to_string(), caret: None }], false);
    }

    /// Puts `with` in place of every match, as one undo.
    pub fn replace_all(&mut self, matches: &[Range<usize>], with: &str) {
        let edits = matches.iter().map(|r| Edit { range: r.clone(), text: with.to_string(), caret: None });
        self.break_group();
        self.apply(edits.collect(), false);
    }
}
