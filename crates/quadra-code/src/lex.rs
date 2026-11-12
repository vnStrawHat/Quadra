//! What the tokenizers share: where they are in a line, and the tokens found so far.

use crate::token::{Token, TokenKind};

pub struct Lex<'a> {
    pub s: &'a [u8],
    pub i: usize,
    pub out: &'a mut Vec<Token>,
    /// Where the tokens of this line start in `out`.
    first: usize,
    /// Where the line starts after its spaces and tabs.
    lead: usize,
}

/// Letters, digits, `_` and what is not ASCII.
pub fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c >= 0x80
}

/// Whether `c` is one of the bytes of `set`; the byte past the end of a line (0) never is.
pub fn has(set: &str, c: u8) -> bool {
    c != 0 && set.as_bytes().contains(&c)
}

/// `(`, `)`, brackets, braces, `,`, `;` and `.` are punctuation; the rest operators.
pub fn mark(c: u8) -> TokenKind {
    if b"()[]{},;.".contains(&c) { TokenKind::Punctuation } else { TokenKind::Operator }
}

impl<'a> Lex<'a> {
    pub fn new(line: &'a str, out: &'a mut Vec<Token>) -> Lex<'a> {
        let (s, first) = (line.as_bytes(), out.len());
        let lead = s.iter().take_while(|c| matches!(c, b' ' | b'\t')).count();
        Lex { s, i: 0, out, first, lead }
    }

    /// The byte `k` after where it is; 0 past the end.
    pub fn at(&self, k: usize) -> u8 {
        self.byte(self.i + k)
    }

    /// The byte at `k`; 0 past the end.
    pub fn byte(&self, k: usize) -> u8 {
        self.s.get(k).copied().unwrap_or(0)
    }

    /// The byte before `a`; 0 at the start.
    pub fn before(&self, a: usize) -> u8 {
        a.checked_sub(1).map_or(0, |k| self.byte(k))
    }

    pub fn done(&self) -> bool {
        self.i >= self.s.len()
    }

    /// How many tokens this line has so far, and back to that many.
    pub fn count(&self) -> usize {
        self.out.len() - self.first
    }

    /// Back to `count` tokens, and none of them past `a`: a token that joined the one
    /// before it is taken out of that one.
    pub fn back_to(&mut self, count: usize, a: usize) {
        self.out.truncate(self.first + count);
        if let Some(last) = self.out[self.first..].last_mut() {
            last.end = last.end.min(a);
        }
    }

    /// `a..i` is a token of a kind; it joins the one before when that ends at `a` and is of
    /// the same kind.
    pub fn add(&mut self, a: usize, kind: TokenKind) {
        if self.i <= a {
            return;
        }
        match self.out[self.first..].last_mut() {
            Some(last) if last.kind == kind && last.end == a => last.end = self.i,
            _ => self.out.push(Token { start: a, end: self.i, kind }),
        }
    }

    pub fn skip_space(&mut self) {
        while matches!(self.at(0), b' ' | b'\t') {
            self.i += 1;
        }
    }

    /// Past the byte it is at and the rest of that character: a token never ends inside one.
    pub fn step(&mut self) {
        self.i += 1;
        while self.at(0) & 0xC0 == 0x80 {
            self.i += 1;
        }
    }

    /// Past a backslash and the character after it.
    pub fn escape(&mut self) {
        self.i += 1;
        if !self.done() {
            self.step();
        }
    }

    /// Past the bytes that `more` accepts.
    pub fn skip_while(&mut self, more: impl Fn(u8) -> bool) {
        while !self.done() && more(self.s[self.i]) {
            self.i += 1;
        }
    }

    /// Whether `s[a..b]` is one of the words of `set` (words with a space between them),
    /// compared in place; with `fold`, whatever its case (the set is in lower case).
    pub fn word_in(&self, set: &str, a: usize, b: usize, fold: bool) -> bool {
        let word = &self.s[a..b];
        let same = |w: &str| {
            w.len() == word.len()
                && w.bytes().zip(word).all(|(w, c)| w == if fold { c.to_ascii_lowercase() } else { *c })
        };
        set.split(' ').any(same)
    }

    /// Whether `m` is where it is; with `fold`, whatever the case of the line.
    pub fn starts_at(&self, m: &str, fold: bool) -> bool {
        let rest = &self.s[self.i.min(self.s.len())..];
        let same = |(m, c): (u8, &u8)| m == if fold { c.to_ascii_lowercase() } else { *c };
        !m.is_empty() && rest.len() >= m.len() && m.bytes().zip(rest).all(same)
    }

    pub fn only_space_before(&self, a: usize) -> bool {
        a <= self.lead
    }

    pub fn next_nonspace(&self, from: usize) -> usize {
        from + self.s[from.min(self.s.len())..].iter().take_while(|c| matches!(c, b' ' | b'\t')).count()
    }

    /// From `a` to the next `close`, which is in it, or to the end of the line: a token.
    pub fn through(&mut self, a: usize, close: u8, kind: TokenKind) {
        self.i = a + 1;
        self.skip_while(|c| c != close);
        self.i = (self.i + 1).min(self.s.len());
        self.add(a, kind);
    }
}
