//! The tokenizer of YAML. States: 0, or n > 0 in a block scalar whose key is indented
//! n - 1: it lasts while lines are blank or indented deeper.

use crate::lex::Lex;
use crate::token::{Token, TokenKind};

const CONSTANTS: &str = "true false True False TRUE FALSE yes no on off null Null NULL ~";

pub fn tokenize(line: &str, state: u32, out: &mut Vec<Token>) -> u32 {
    let mut l = Lex::new(line, out);
    l.skip_space();
    let indent = l.i;
    if state > 0 && (l.done() || indent >= state as usize) {
        l.i = line.len();
        l.add(indent, TokenKind::Str);
        return state;
    }
    if line.starts_with('%') {
        l.i = line.len();
        l.add(0, TokenKind::Keyword);
        return 0;
    }
    if (line.starts_with("---") || line.starts_with("...")) && matches!(l.byte(3), 0 | b' ') {
        l.i = 3;
        l.add(0, TokenKind::Keyword);
        l.skip_space();
    }
    while l.at(0) == b'-' && matches!(l.at(1), 0 | b' ') {
        l.i += 1;
        l.add(l.i - 1, TokenKind::Punctuation);
        l.skip_space();
    }
    l.yaml_rest()
}

impl Lex<'_> {
    /// Keys and values to the end of the line; gives the state. A block scalar is as deep
    /// as its key, which is where this starts.
    fn yaml_rest(&mut self) -> u32 {
        let (indent, mut flow) = (self.i, 0_i64);
        while !self.done() {
            let (c, a) = (self.s[self.i], self.i);
            let ends_key = matches!(self.at(1), 0 | b' ') || (flow > 0 && matches!(self.at(1), b',' | b'}'));
            match c {
                b' ' | b'\t' => self.i += 1,
                b'#' if matches!(self.before(a), 0 | b' ' | b'\t') => {
                    self.i = self.s.len();
                    self.add(a, TokenKind::Comment);
                }
                b'"' | b'\'' => {
                    self.yaml_quoted(c);
                    self.key_or(a, TokenKind::Str);
                }
                b'&' | b'*' | b'!' => {
                    self.i += 1;
                    self.skip_while(|c| !matches!(c, b' ' | b',' | b']' | b'}'));
                    self.add(a, if c == b'!' { TokenKind::Tag } else { TokenKind::Attribute });
                }
                b'|' | b'>' if flow == 0 && self.block_header() => {
                    self.add(a, TokenKind::Punctuation);
                    return u32::try_from(indent + 1).unwrap_or(u32::MAX);
                }
                b'{' | b'[' | b'}' | b']' | b',' => {
                    flow += match c {
                        b'{' | b'[' => 1,
                        b',' => 0,
                        _ => -1,
                    };
                    self.i += 1;
                    self.add(a, TokenKind::Punctuation);
                }
                b':' if ends_key => {
                    self.i += 1;
                    self.add(a, TokenKind::Punctuation);
                }
                _ => self.yaml_plain(a, flow > 0),
            }
        }
        0
    }

    /// `a..i` is a property when a colon follows it, else of the kind.
    fn key_or(&mut self, a: usize, kind: TokenKind) {
        let k = self.i + self.s[self.i..].iter().take_while(|c| **c == b' ').count();
        let key = self.byte(k) == b':' && matches!(self.byte(k + 1), 0 | b' ');
        self.add(a, if key { TokenKind::Property } else { kind });
    }

    fn yaml_quoted(&mut self, quote: u8) {
        self.i += 1;
        while !self.done() {
            let c = self.s[self.i];
            if quote == b'"' && c == b'\\' {
                self.escape();
                continue;
            }
            self.i += 1;
            if c == quote {
                if quote == b'\'' && self.at(0) == b'\'' {
                    self.i += 1; // '' is a quote in the string
                    continue;
                }
                break;
            }
        }
    }

    /// A plain scalar from `a`: to `: `, ` #`, the end, or in a flow to what ends a value
    /// there. It is a constant, a number or a string, or a key.
    fn yaml_plain(&mut self, a: usize, in_flow: bool) {
        self.i += 1; // what starts it is in it, whatever it is
        while !self.done() {
            let (c, n) = (self.s[self.i], self.at(1));
            let ends = match c {
                b':' => matches!(n, 0 | b' ') || (in_flow && matches!(n, b',' | b'}')),
                b'#' => self.before(self.i) == b' ',
                b',' | b'[' | b']' | b'{' | b'}' => in_flow,
                _ => false,
            };
            if ends {
                break;
            }
            self.i += 1;
        }
        let end = self.i;
        let spaces = self.s[a..end].iter().rev().take_while(|c| **c == b' ').count();
        let e = end - spaces;
        let kind = if self.word_in(CONSTANTS, a, e, false) {
            TokenKind::Constant
        } else if is_number(&self.s[a..e]) {
            TokenKind::Number
        } else {
            TokenKind::Str
        };
        self.i = e;
        self.key_or(a, kind);
        self.i = end;
    }

    /// `|` or `>` with its indicators and nothing after them but a comment: goes past it.
    fn block_header(&mut self) -> bool {
        let mut k = self.i + 1;
        while matches!(self.byte(k), b'+' | b'-') || self.byte(k).is_ascii_digit() {
            k += 1;
        }
        if self.s.get(self.next_nonspace(k)).is_some_and(|c| *c != b'#') {
            return false;
        }
        self.i = k;
        true
    }
}

fn is_number(w: &[u8]) -> bool {
    if [".inf", "-.inf", "+.inf", ".nan", ".NaN", ".Inf"].iter().any(|n| n.as_bytes() == w) {
        return true;
    }
    if w.len() > 2 && w[0] == b'0' && matches!(w[1], b'x' | b'o') {
        return w[2..].iter().all(u8::is_ascii_hexdigit);
    }
    let mut digits = 0;
    for (i, c) in w.iter().enumerate() {
        let sign = matches!(c, b'-' | b'+') && (i == 0 || matches!(w[i - 1], b'e' | b'E'));
        if c.is_ascii_digit() {
            digits += 1;
        } else if !(matches!(c, b'.' | b'_' | b'e' | b'E') || sign) {
            return false;
        }
    }
    digits > 0 && (w[0].is_ascii_digit() || matches!(w[0], b'-' | b'+' | b'.'))
}
