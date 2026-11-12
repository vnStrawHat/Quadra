//! What selects: a click, everything, lines, the word at an offset, a cursor more above or
//! below, the next place the selected text is; and the bracket that matches one.

use std::ops::Range;

use super::motion::{Class, class_of};
use super::{Cursor, Doc};

/// How far a bracket's match is looked for, in bytes.
// ponytail: strings and comments are not skipped, and a match further away is not found.
const BRACKET_REACH: usize = 20_000;
const BRACKETS: [(u8, u8); 3] = [(b'(', b')'), (b'[', b']'), (b'{', b'}')];

impl Doc {
    /// Puts one cursor in place of all (a click), or `add`s it to them.
    pub fn set_cursor(&mut self, anchor: usize, head: usize, add: bool) {
        let len = self.buffer.len();
        if !add {
            self.cursors.clear();
        }
        self.cursors.push(Cursor { anchor: anchor.min(len), head: head.min(len), goal: None });
        self.normalize();
        self.break_group();
    }

    pub fn select_all(&mut self) {
        self.cursors = vec![Cursor { anchor: 0, head: self.buffer.len(), goal: None }];
    }

    /// Makes every cursor select the whole of its lines, with their line breaks.
    pub fn select_line(&mut self) {
        for i in 0..self.cursors.len() {
            let c = self.cursors[i];
            let (first, last) = (self.buffer.line_of(c.start()), self.buffer.line_of(c.end()));
            let end = if last + 1 < self.buffer.line_count() {
                self.buffer.line_start(last + 1)
            } else {
                self.buffer.len()
            };
            self.cursors[i] = Cursor { anchor: self.buffer.line_start(first), head: end, goal: None };
        }
        self.normalize();
    }

    /// The word about an offset: empty when the offset is in none.
    pub fn word_at(&self, offset: usize) -> Range<usize> {
        let line = self.buffer.line_of(offset);
        let (start, text) = (self.buffer.line_start(line), self.buffer.line(line));
        let (bytes, at) = (text.as_bytes(), offset - start);
        let word = |byte: &&u8| class_of(**byte) == Class::Word;
        let before = bytes[..at].iter().rev().take_while(word).count();
        offset - before..offset + bytes[at..].iter().take_while(word).count()
    }

    /// Adds a cursor on the line above the first or below the last, in the same columns: a
    /// selection in a line is one in the next line too, which selects a column.
    pub fn add_cursor_vertical(&mut self, down: bool) {
        let edge = if down { self.primary() } else { self.cursors[0] };
        let line = self.buffer.line_of(edge.head);
        if (down && line + 1 >= self.buffer.line_count()) || (!down && line == 0) {
            return;
        }
        let target = if down { line + 1 } else { line - 1 };
        let goal = edge.goal.unwrap_or_else(|| self.col_of(edge.head));
        let head = self.offset_at_col(target, goal);
        let same_line = self.buffer.line_of(edge.anchor) == line;
        let anchor = if same_line { self.offset_at_col(target, self.col_of(edge.anchor)) } else { head };
        self.cursors.push(Cursor { anchor, head, goal: Some(goal) });
        self.normalize();
    }

    /// Selects the word at the primary cursor; when it selects something, adds a cursor at
    /// the next place the same text is, from the start again after the end.
    pub fn select_next(&mut self) {
        let primary = self.primary();
        if primary.is_empty() {
            let word = self.word_at(primary.head);
            if !word.is_empty() {
                let last = self.cursors.len() - 1;
                self.cursors[last] = Cursor { anchor: word.start, head: word.end, goal: None };
                // Another cursor may be in the word.
                self.normalize();
            }
            return;
        }
        let needle = self.buffer.slice(primary.range()).into_owned();
        let taken = |at: &usize| self.cursors.iter().any(|c| c.start() == *at);
        // After the last cursor to the end, then from the start: the first not selected yet.
        let mut from = primary.end();
        let mut wrapped = false;
        let found = loop {
            match self.buffer.find(&needle, from) {
                Some(at) if !taken(&at) => break Some(at),
                Some(at) => from = at + needle.len(),
                None if wrapped => break None,
                None => (from, wrapped) = (0, true),
            }
            if wrapped && from >= primary.start() {
                break None;
            }
        };
        if let Some(at) = found {
            self.cursors.push(Cursor { anchor: at, head: at + needle.len(), goal: None });
            self.normalize();
        }
    }

    /// The bracket at the primary cursor, or before it, and the bracket that matches it.
    pub fn bracket_pair(&self) -> Option<(usize, usize)> {
        let head = self.primary().head;
        [Some(head), head.checked_sub(1)].into_iter().flatten().find_map(|at| {
            let byte = self.buffer.bytes(at).next()?;
            let open = BRACKETS.iter().find(|pair| pair.0 == byte);
            let close = BRACKETS.iter().find(|pair| pair.1 == byte);
            // How deep in brackets of its kind each byte is, from the bracket on: its match
            // is where that is nothing again.
            let depth = |(same, other): (u8, u8)| {
                let mut depth = 0isize;
                move |byte: u8| {
                    depth += isize::from(byte == same) - isize::from(byte == other);
                    depth == 0
                }
            };
            let other = match (open, close) {
                (Some(pair), _) => at + self.buffer.bytes(at).take(BRACKET_REACH).position(depth(*pair))?,
                (None, Some(pair)) => {
                    at - self
                        .buffer
                        .bytes_back(at + 1)
                        .take(BRACKET_REACH)
                        .position(depth((pair.1, pair.0)))?
                }
                (None, None) => return None,
            };
            Some((at, other))
        })
    }
}
