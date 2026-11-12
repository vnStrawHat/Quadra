//! Markup and diffs. The states of markup: 0 text, 1 in a `<!-- -->`, 2 in a tag (its
//! attributes may go on over lines).

use crate::lex::{Lex, is_ident};
use crate::token::{Token, TokenKind};

/// What a name of a tag or of an attribute is made of, after its first letter.
fn in_name(c: u8) -> bool {
    is_ident(c) || matches!(c, b'-' | b':' | b'.')
}

impl Lex<'_> {
    // ponytail: what is in <script> and <style> stays plain text; the rows of those
    // languages when code in markup shows in real documents.
    pub(super) fn markup(&mut self, state: u32) -> u32 {
        let mut state = state;
        while !self.done() {
            let (a, c) = (self.i, self.s[self.i]);
            if state == 1 {
                state = self.markup_comment(a);
            } else if state == 2 {
                state = self.tag_attrs();
            } else if self.starts_at("<!--", false) {
                self.i += 4;
                state = self.markup_comment(a);
            } else if c == b'<'
                && (self.at(1).is_ascii_alphabetic() || matches!(self.at(1), b'/' | b'!' | b'?'))
            {
                let named = self.at(1).is_ascii_alphabetic();
                self.i += if named { 1 } else { 2 };
                self.add(a, TokenKind::Punctuation);
                let b = self.i;
                self.skip_while(in_name);
                let declares = matches!(self.byte(a + 1), b'!' | b'?');
                self.add(b, if declares { TokenKind::Keyword } else { TokenKind::Tag });
                state = 2;
            } else if c == b'&' && (is_ident(self.at(1)) || self.at(1) == b'#') {
                // An entity is a dozen bytes at most, and ends with a `;`.
                self.i += 1;
                while self.i - a < 12 && (is_ident(self.at(0)) || self.at(0) == b'#') {
                    self.i += 1;
                }
                if self.at(0) == b';' {
                    self.i += 1;
                    self.add(a, TokenKind::Escape);
                }
            } else {
                self.i += 1;
            }
        }
        state
    }

    fn markup_comment(&mut self, a: usize) -> u32 {
        while !self.done() {
            if self.starts_at("-->", false) {
                self.i += 3;
                self.add(a, TokenKind::Comment);
                return 0;
            }
            self.i += 1;
        }
        self.add(a, TokenKind::Comment);
        1
    }

    /// Reads attributes up to the end of the tag: 0 when it closed, 2 when it goes on.
    fn tag_attrs(&mut self) -> u32 {
        while !self.done() {
            let (a, c) = (self.i, self.s[self.i]);
            if c == b'>' || ((c == b'/' || c == b'?') && self.at(1) == b'>') {
                self.i += if c == b'>' { 1 } else { 2 };
                self.add(a, TokenKind::Punctuation);
                return 0;
            }
            if c == b'"' || c == b'\'' {
                self.through(a, c, TokenKind::Str);
            } else if is_ident(c) {
                self.skip_while(in_name);
                self.add(a, TokenKind::Attribute);
            } else {
                self.i += 1;
                if c == b'=' {
                    self.add(a, TokenKind::Operator);
                }
            }
        }
        2
    }
}

/// A whole line of a diff: headers are keywords, hunks escapes, what was added a string,
/// what was removed a variable.
pub fn diff_line(line: &str, out: &mut Vec<Token>) {
    let starts = |marks: &[&str]| marks.iter().any(|mark| line.starts_with(mark));
    let kind = if starts(&["diff ", "index ", "--- ", "+++ "]) {
        TokenKind::Keyword
    } else if starts(&["@@"]) {
        TokenKind::Escape
    } else if starts(&["+", ">"]) {
        TokenKind::Str
    } else if starts(&["-", "<"]) {
        TokenKind::Variable
    } else {
        return;
    };
    out.push(Token { start: 0, end: line.len(), kind });
}
