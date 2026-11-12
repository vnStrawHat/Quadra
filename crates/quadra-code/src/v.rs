//! The tokenizer of V. States: 0 code, 1 to 999 how deep in block comments, 1000 and its
//! quote (and 256 when raw) an open string.

use crate::lex::{Lex, is_ident, mark};
use crate::token::{Token, TokenKind};

const KEYWORDS: &str = "fn struct enum interface type union pub mut const module import as in is sizeof typeof isreftype __global shared lock rlock atomic static volatile unsafe go spawn asm chan select assert __offsetof";
const CONTROL: &str = "if else for match return break continue goto defer or";
const CONSTANTS: &str = "true false none nil";
const TYPES: &str = "int i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 bool string rune byte voidptr byteptr charptr isize usize char any map thread";

pub fn tokenize(line: &str, state: u32, out: &mut Vec<Token>) -> u32 {
    let mut l = Lex::new(line, out);
    let mut state = match state {
        0 => 0,
        1..1000 => l.block_comment(0, state),
        _ => l.v_string(0, state),
    };
    while state == 0 && !l.done() {
        let (c, a) = (l.s[l.i], l.i);
        match (c, l.at(1)) {
            (b' ' | b'\t', _) => l.i += 1,
            (b'/', b'/') => {
                l.i = l.s.len();
                l.add(a, TokenKind::Comment);
            }
            (b'/', b'*') => {
                l.i += 2;
                state = l.block_comment(a, 1);
            }
            (b'\'' | b'"', _) => {
                l.i += 1;
                state = l.v_string(a, 1000 + u32::from(c));
            }
            (b'r', quote @ (b'\'' | b'"')) => {
                l.i += 2;
                state = l.v_string(a, 1000 + 256 + u32::from(quote));
            }
            (b'`', _) => l.v_rune(),
            (b'@', _) => l.v_at(),
            _ if c.is_ascii_digit() => {
                l.skip_while(is_ident);
                while l.at(0) == b'.' && l.at(1).is_ascii_digit() {
                    l.i += 1;
                    l.skip_while(is_ident);
                }
                l.add(a, TokenKind::Number);
            }
            _ if is_ident(c) => l.v_ident(),
            _ => {
                l.i += 1;
                l.add(a, mark(c));
            }
        }
    }
    state
}

impl Lex<'_> {
    /// Reads a `/* */` comment, which nests, from `a`; gives how deep it is still open.
    fn block_comment(&mut self, a: usize, depth: u32) -> u32 {
        let mut depth = depth;
        while !self.done() && depth > 0 {
            match (self.at(0), self.at(1)) {
                (b'/', b'*') => (depth, self.i) = ((depth + 1).min(999), self.i + 2),
                (b'*', b'/') => (depth, self.i) = (depth - 1, self.i + 2),
                _ => self.i += 1,
            }
        }
        self.add(a, TokenKind::Comment);
        depth
    }

    /// Reads a string from `a` to its quote; gives the state, 0 when it closed.
    fn v_string(&mut self, a: usize, state: u32) -> u32 {
        let (quote, raw) = (((state - 1000) & 255) as u8, state - 1000 >= 256);
        let mut from = a;
        while !self.done() {
            let c = self.s[self.i];
            if c == quote {
                self.i += 1;
                self.add(from, TokenKind::Str);
                return 0;
            }
            if !raw && c == b'\\' {
                self.add(from, TokenKind::Str);
                let e = self.i;
                self.escape();
                self.add(e, TokenKind::Escape);
                from = self.i;
            } else if !raw && c == b'$' && self.at(1) == b'{' {
                self.add(from, TokenKind::Str);
                self.interpolation();
                from = self.i;
            } else {
                self.i += 1;
            }
        }
        self.add(from, TokenKind::Str);
        state
    }

    /// `${` and `}` are escapes, the expression between them a variable.
    fn interpolation(&mut self) {
        let e = self.i;
        self.i += 2;
        self.add(e, TokenKind::Escape);
        let (inner, mut depth) = (self.i, 1);
        while !self.done() {
            let c = self.s[self.i];
            depth += i32::from(c == b'{') - i32::from(c == b'}');
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

    /// A character between backticks.
    fn v_rune(&mut self) {
        let a = self.i;
        self.i += 1;
        while !self.done() && self.s[self.i] != b'`' {
            if self.s[self.i] == b'\\' { self.escape() } else { self.i += 1 }
        }
        self.i = (self.i + 1).min(self.s.len());
        self.add(a, TokenKind::Str);
    }

    /// An attribute `@[..]`, or a name of the compiler: `@FILE`.
    fn v_at(&mut self) {
        let a = self.i;
        if self.at(1) == b'[' {
            self.through(a, b']', TokenKind::Attribute);
            return;
        }
        self.i += 1;
        self.skip_while(is_ident);
        self.add(a, TokenKind::Constant);
    }

    fn v_ident(&mut self) {
        let a = self.i;
        self.skip_while(is_ident);
        let b = self.i;
        let after = b + self.s[b..].iter().take_while(|c| **c == b' ').count();
        let (call, after_dot) = (self.byte(after) == b'(', self.before(a) == b'.');
        let is = |set: &str| self.word_in(set, a, b, false);
        let kind = if after_dot {
            if call { TokenKind::Function } else { TokenKind::Property }
        } else if is(KEYWORDS) {
            TokenKind::Keyword
        } else if is(CONTROL) {
            TokenKind::Control
        } else if is(CONSTANTS) {
            TokenKind::Constant
        } else if is(TYPES) || self.s[a].is_ascii_uppercase() {
            TokenKind::Type
        } else if call {
            TokenKind::Function
        } else {
            return;
        };
        self.add(a, kind);
    }
}
