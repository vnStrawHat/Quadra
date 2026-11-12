//! Strings, comments, numbers and variables for the scanner of the table's rows.

use super::LangSpec;
use crate::lex::{Lex, has, is_ident};
use crate::token::TokenKind;

/// What an open string is: these are added to 1000 and its quote.
const RAW: u32 = 256; // no escapes
const TRIPLE: u32 = 512; // closed by three quotes
const HASH: u32 = 1024; // a raw string with one hash: r#"..."#

impl Lex<'_> {
    /// Goes past what opens a string where it is and gives the string's state; 0 when
    /// there is none.
    pub(super) fn string_open(&mut self, sp: &LangSpec) -> u32 {
        let (mut k, mut raw) = (self.i, false);
        while k - self.i < 2 && has(sp.prefixes, self.byte(k)) {
            raw |= matches!(self.s[k], b'r' | b'R' | b'@');
            k += 1;
        }
        let q = self.byte(k);
        if k > self.i && q == b'#' && self.byte(k + 1) == b'"' && raw {
            self.i = k + 2;
            return 1000 + u32::from(b'"') + RAW + HASH;
        }
        if !(has(sp.quotes, q) || has(sp.multi_quotes, q)) {
            if k == self.i && q == b'[' && sp.long_brackets && self.at(1) == b'[' {
                self.i += 2;
                return 1000 + u32::from(b'[') + RAW;
            }
            return 0;
        }
        if q == b'\'' && sp.lifetimes && self.byte(k + 1) != b'\\' && self.byte(k + 2) != b'\'' {
            if k == self.i && is_ident(self.at(1)) {
                // A lifetime, or a label.
                let a = self.i;
                self.i += 1;
                self.skip_while(is_ident);
                self.add(a, TokenKind::Attribute);
            }
            return 0;
        }
        let kind = u32::from(q) + if raw || has(sp.raw_quotes, q) { RAW } else { 0 };
        if sp.triple && self.byte(k + 1) == q && self.byte(k + 2) == q {
            self.i = k + 3;
            return 1000 + kind + TRIPLE;
        }
        self.i = k + 1;
        1000 + kind
    }

    /// How long what closes the string is, where it is; 0 when it is not there.
    fn close_len(&self, q: u8, kind: u32) -> usize {
        let here = |close: &[u8]| self.s[self.i..].starts_with(close);
        if kind & TRIPLE != 0 {
            3 * usize::from(here(&[q, q, q]))
        } else if kind & HASH != 0 {
            2 * usize::from(here(b"\"#"))
        } else if q == b'[' {
            2 * usize::from(here(b"]]"))
        } else {
            usize::from(self.at(0) == q)
        }
    }

    /// Reads a string from `a`; gives the state: 0 when it closed here, or at the end of
    /// the line for a quote that ends with its line.
    pub(super) fn lang_string(&mut self, sp: &LangSpec, a: usize, state: u32) -> u32 {
        let kind = state - 1000;
        let (q, raw) = ((kind & 255) as u8, kind & RAW != 0);
        let interp = !raw && has(sp.interp_in, q);
        let mut from = a;
        while !self.done() {
            let closes = self.close_len(q, kind);
            if closes > 0 {
                self.i += closes;
                self.add(from, TokenKind::Str);
                return 0;
            }
            let c = self.s[self.i];
            if interp && sp.interps.iter().any(|open| self.starts_at(open, false)) {
                self.add(from, TokenKind::Str);
                self.interpolate(sp);
                from = self.i;
            } else if interp && sp.dollar_var && c == b'$' && is_ident(self.at(1)) {
                self.add(from, TokenKind::Str);
                self.variable(sp);
                from = self.i;
            } else if !raw && c == b'\\' {
                self.add(from, TokenKind::Str);
                let e = self.i;
                self.escape();
                self.add(e, TokenKind::Escape);
                from = self.i;
            } else {
                self.i += 1;
            }
        }
        self.add(from, TokenKind::Str);
        let goes_on = kind & (TRIPLE | HASH) != 0 || q == b'[' || has(sp.multi_quotes, q);
        if goes_on { state } else { 0 }
    }

    /// The opener of an expression and its closer are escapes, the expression a variable.
    fn interpolate(&mut self, sp: &LangSpec) {
        let e = self.i;
        let opener = sp.interps.iter().find(|open| self.starts_at(open, false)).copied().unwrap_or("{");
        let open = opener.as_bytes()[opener.len() - 1];
        self.i += opener.len();
        let close = if open == b'(' { b')' } else { b'}' };
        self.add(e, TokenKind::Escape);
        let (inner, mut depth) = (self.i, 1);
        while !self.done() {
            let c = self.s[self.i];
            depth += i32::from(c == open) - i32::from(c == close);
            if depth == 0 {
                break;
            }
            self.i += 1;
        }
        self.add(inner, TokenKind::Variable);
        if !self.done() {
            self.i += 1;
            self.add(self.i - 1, TokenKind::Escape);
        }
    }

    /// `$name`, `${..}`, `$(..)`, `$1`, `$@`, or `%name%`, `%1`; false when none is here.
    pub(super) fn variable(&mut self, sp: &LangSpec) -> bool {
        let (a, c, n) = (self.i, self.s[self.i], self.at(1));
        if c == b'%' {
            if n == b'%' || n.is_ascii_digit() || n == b'~' {
                self.i += 2;
            } else if is_ident(n) {
                self.through(a, b'%', TokenKind::Variable);
                return true;
            } else {
                return false;
            }
            self.skip_while(is_ident);
        } else if n == b'{' || (n == b'(' && sp.label_colon) {
            self.through(a, if n == b'{' { b'}' } else { b')' }, TokenKind::Variable);
            return true;
        } else if is_ident(n) {
            self.i += 1;
            self.skip_while(is_ident);
        } else if has("@#?*!$-_", n) {
            self.i += 2;
        } else {
            return false;
        }
        self.add(a, TokenKind::Variable);
        true
    }

    /// `@name` (and `@a.b`): an attribute, a variable or a keyword, as the row says.
    pub(super) fn at_word(&mut self, sp: &LangSpec) {
        let a = self.i;
        self.i += 1 + usize::from(self.at(1) == b'@');
        self.skip_while(|c| is_ident(c) || (sp.at_attr && c == b'.') || (sp.at_kw && c == b'-'));
        let kind = match (sp.at_attr, sp.at_var) {
            (true, _) => TokenKind::Attribute,
            (false, true) => TokenKind::Variable,
            (false, false) => TokenKind::Keyword,
        };
        self.add(a, kind);
    }

    /// `0x1F`, `0b101`, `1_000`, `1.5e-3`, `10px`, `2u8`.
    pub(super) fn lang_number(&mut self) {
        let a = self.i;
        let hex = self.at(0) == b'0' && self.at(1) | 0x20 == b'x';
        while !self.done() {
            let c = self.s[self.i];
            let sign = (c == b'+' || c == b'-') && !hex && self.before(self.i) | 0x20 == b'e';
            if !(is_ident(c) || ((c == b'.' || sign) && self.at(1).is_ascii_digit())) {
                break;
            }
            self.i += 1;
        }
        self.add(a, TokenKind::Number);
    }

    /// Reads a block comment from `a`; gives how deep it is still open.
    pub(super) fn lang_block(&mut self, sp: &LangSpec, a: usize, depth: u32) -> u32 {
        let mut depth = depth;
        while !self.done() && depth > 0 {
            if sp.nest && self.starts_at(sp.block_open, false) {
                // No deeper than the states of comments go: the next are those of strings.
                depth = (depth + 1).min(999);
                self.i += sp.block_open.len();
            } else if self.starts_at(sp.block_close, false) {
                depth -= 1;
                self.i += sp.block_close.len();
            } else {
                self.i += 1;
            }
        }
        self.add(a, TokenKind::Comment);
        depth
    }
}
