//! A statement of a diagram, read from left to right.

use super::draw::Head;

pub struct Scan<'a> {
    s: &'a str,
    i: usize,
    /// For each thing searched for: where the search started, and where it found it. A
    /// statement is not searched again for what it was already searched for.
    seen: [(usize, Option<usize>); 8],
}

/// What a name is made of: letters, digits, `_` and what is not ASCII.
pub fn is_id(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c >= 0x80
}

impl<'a> Scan<'a> {
    pub fn new(s: &'a str) -> Scan<'a> {
        Scan { s, i: 0, seen: [(usize::MAX, None); 8] }
    }

    pub fn rest(&self) -> &'a str {
        &self.s[self.i..]
    }

    /// What is left, for a fault to show: forty characters of it at most.
    pub fn rest_shown(&self) -> &'a str {
        let rest = self.rest();
        rest.char_indices().nth(40).map_or(rest, |(end, _)| &rest[..end])
    }

    pub fn done(&self) -> bool {
        self.i >= self.s.len()
    }

    fn byte(&self) -> Option<u8> {
        self.s.as_bytes().get(self.i).copied()
    }

    pub fn space(&mut self) {
        while matches!(self.byte(), Some(b' ' | b'\t')) {
            self.i += 1;
        }
    }

    /// Goes past `what` when it is next.
    pub fn take(&mut self, what: &str) -> bool {
        let found = self.rest().starts_with(what);
        self.i += if found { what.len() } else { 0 };
        found
    }

    /// Goes `bytes` on: they end between characters.
    pub fn skip(&mut self, bytes: usize) {
        self.i += bytes;
    }

    /// The name that is next; empty when none is.
    pub fn id(&mut self) -> &'a str {
        let start = self.i;
        while self.byte().is_some_and(is_id) {
            self.i += 1;
        }
        &self.s[start..self.i]
    }

    /// Goes past the name of a class: a name, with dashes.
    pub fn class(&mut self) {
        while self.byte().is_some_and(|c| is_id(c) || c == b'-') {
            self.i += 1;
        }
    }

    /// A run of `-`, `=` and `.`, and the head after it.
    pub fn link_body(&mut self) -> (&'a str, Head) {
        let start = self.i;
        while matches!(self.byte(), Some(b'-' | b'=' | b'.')) {
            self.i += 1;
        }
        let body = &self.s[start..self.i];
        let after = self.s.as_bytes().get(self.i + 1).copied().unwrap_or(b' ');
        let head = match self.byte() {
            Some(b'>') if !body.is_empty() => Head::Arrow,
            Some(b'o') if !body.is_empty() && !is_id(after) => Head::Circle,
            Some(b'x') if !body.is_empty() && !is_id(after) => Head::Cross,
            _ => return (body, Head::None),
        };
        self.i += 1;
        (body, head)
    }

    /// How far the nearest of `marks` is from where the scanner is; none when none is left.
    pub fn nearest(&mut self, marks: &[&str; 8]) -> Option<usize> {
        let mut best: Option<usize> = None;
        for (k, mark) in marks.iter().enumerate() {
            let (from, found) = self.seen[k];
            if from > self.i || found.is_some_and(|at| at < self.i) {
                self.seen[k] = (self.i, self.rest().find(mark).map(|at| at + self.i));
            }
            let at = self.seen[k].1;
            best = best.into_iter().chain(at).min();
        }
        best.map(|at| at - self.i)
    }
}
