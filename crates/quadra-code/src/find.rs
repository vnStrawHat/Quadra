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
        // tens of megabytes make it slow (a change searches only its lines, `rematch`).
        let text = doc.text();
        Ok(self.find_in(&pattern, &text, 0).collect())
    }

    /// The matches in `text`, which starts at `base` in the document and at the start of a
    /// line, and ends at the end of one: what is past either end is no letter.
    fn find_in<'a>(
        &self,
        pattern: &'a Regex,
        text: &'a str,
        base: usize,
    ) -> impl Iterator<Item = Range<usize>> + 'a {
        let wordy = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
        let word = self.word;
        pattern
            .find_iter(text)
            .map(|m| m.range())
            .filter(|r| !r.is_empty())
            .filter(move |r| {
                !word || (!wordy(text[..r.start].chars().next_back()) && !wordy(text[r.end..].chars().next()))
            })
            .map(move |r| base + r.start..base + r.end)
    }

    /// The matches after the changes since `version`, from `matches`, those of then: the
    /// lines a change was are searched again, and the matches after it move. `None` for a
    /// pattern (it may match across lines), or when the changes are no longer all known.
    pub fn rematch(
        &self,
        doc: &Doc,
        version: u64,
        mut matches: Vec<Range<usize>>,
    ) -> Option<Vec<Range<usize>>> {
        if self.regex || self.text.is_empty() {
            return None;
        }
        let pattern = self.pattern().ok()?;
        // What was changed, in bytes of the text as it is after the changes so far.
        let mut changed: Vec<Range<usize>> = Vec::new();
        for c in doc.line_changes_since(version)? {
            let (from, to, by) = (c.at, c.at + c.removed, c.inserted as isize - c.removed as isize);
            let moved = |r: &Range<usize>| r.start.wrapping_add_signed(by)..r.end.wrapping_add_signed(by);
            // A match before the change stays, one after it moves, one over it goes.
            matches.retain_mut(|m| {
                if m.end <= from {
                    return true;
                }
                if m.start >= to {
                    *m = moved(m);
                    return true;
                }
                false
            });
            let mut fresh = from..to;
            changed.retain_mut(|r| {
                if r.end < from || r.start > to {
                    if r.start > to {
                        *r = moved(r);
                    }
                    return true;
                }
                fresh = fresh.start.min(r.start)..fresh.end.max(r.end);
                false
            });
            changed.push(fresh.start..fresh.end.wrapping_add_signed(by));
        }
        // Each changed place as the whole lines it is in, and those searched again.
        let buffer = &doc.buffer;
        let mut spans: Vec<Range<usize>> = changed
            .iter()
            .map(|r| buffer.line_start(buffer.line_of(r.start))..buffer.line_end(buffer.line_of(r.end)))
            .collect();
        spans.sort_by_key(|span| span.start);
        spans.dedup_by(|next, kept| {
            let joins = next.start <= kept.end;
            if joins {
                kept.end = kept.end.max(next.end);
            }
            joins
        });
        for span in spans {
            let from = matches.partition_point(|m| m.start < span.start);
            let to = matches.partition_point(|m| m.start <= span.end);
            let text = buffer.slice(span.clone());
            let found: Vec<_> = self.find_in(&pattern, &text, span.start).collect();
            matches.splice(from..to, found);
        }
        Some(matches)
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
