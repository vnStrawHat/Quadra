//! The scanner of the table's rows. States: 0 code, 1 to 999 how deep in block comments,
//! 1000 and up an open string (its quote and what kind it is: `strings.rs`); markup has
//! its own (`markup.rs`).

use super::markup::diff_line;
use super::{LangSpec, Mode};
use crate::lex::{Lex, is_ident, mark};
use crate::token::{Token, TokenKind};

pub fn tokenize(spec: &LangSpec, line: &str, state: u32, out: &mut Vec<Token>) -> u32 {
    if spec.mode == Mode::Diff {
        diff_line(line, out);
        return 0;
    }
    let mut l = Lex::new(line, out);
    if spec.mode == Mode::Markup {
        return l.markup(state);
    }
    let mut state = match state {
        0 => 0,
        1..1000 => l.lang_block(spec, 0, state),
        _ => l.lang_string(spec, 0, state),
    };
    let selector = spec.selectors && is_selector_line(line);
    while state == 0 && !l.done() {
        state = l.lang_step(spec, selector);
    }
    state
}

/// A line of selectors: it has a `{`, or ends with a comma and has no `;`.
fn is_selector_line(line: &str) -> bool {
    let t = line.trim_end_matches([' ', '\t']);
    line.contains('{') || (t.ends_with(',') && !t.contains(';'))
}

impl Lex<'_> {
    /// Reads one token where it is; gives the state after it.
    fn lang_step(&mut self, sp: &LangSpec, selector: bool) -> u32 {
        let (c, a) = (self.s[self.i], self.i);
        if c == b' ' || c == b'\t' {
            self.i += 1;
            return 0;
        }
        if self.starts_at(sp.block_open, false) {
            self.i += sp.block_open.len();
            return self.lang_block(sp, a, 1);
        }
        if self.line_comment(sp) || (sp.colon_label && c == b':' && self.only_space_before(a)) {
            let label = c == b':' && self.at(1) != b':';
            self.i = self.s.len();
            self.add(a, if label { TokenKind::Function } else { TokenKind::Comment });
            return 0;
        }
        if c == b'#' && sp.preproc && self.only_space_before(a) {
            self.preproc();
            return 0;
        }
        if c == b'#' && sp.hash_attr && (self.at(1) == b'[' || (self.at(1) == b'!' && self.at(2) == b'[')) {
            self.through(a, b']', TokenKind::Attribute);
            return 0;
        }
        if c == b'[' && sp.sections && self.only_space_before(a) {
            let end = self.s.iter().rposition(|c| *c == b']').filter(|end| *end > a);
            self.i = end.map_or(self.s.len(), |end| end + 1);
            self.add(a, TokenKind::Tag);
            return 0;
        }
        let opened = self.string_open(sp);
        if opened > 0 {
            let before = self.count();
            let end = self.lang_string(sp, a, opened);
            if end == 0 && sp.key_colon && self.byte(self.next_nonspace(self.i)) == b':' {
                self.back_to(before, a);
                self.add(a, TokenKind::Property);
            }
            return end;
        }
        if self.i > a {
            return 0; // a lifetime, which is no string
        }
        if ((c == b'$' && sp.dollar_var) || (c == b'%' && sp.percent_var)) && self.variable(sp) {
            return 0;
        }
        if c == b'@' && is_ident(self.at(1)) && (sp.at_attr || sp.at_var || sp.at_kw) {
            self.at_word(sp);
            return 0;
        }
        let marks = (c == b'#' || c == b':') && (is_ident(self.at(1)) || (c == b':' && self.at(1) == b':'));
        if sp.selectors && marks {
            self.css_mark(selector);
            return 0;
        }
        let digit_after_dot = c == b'.' && self.at(1).is_ascii_digit() && !is_ident(self.before(a));
        if c.is_ascii_digit() || digit_after_dot {
            self.lang_number();
            return 0;
        }
        if self.ident_start(sp) {
            self.lang_ident(sp, selector);
            return 0;
        }
        self.i += 1;
        self.add(a, mark(c));
        0
    }

    fn line_comment(&self, sp: &LangSpec) -> bool {
        sp.line_comments.iter().any(|m| {
            let word = m.as_bytes()[0].is_ascii_alphabetic();
            if !self.starts_at(m, word) {
                return false;
            }
            if word {
                // A marker of letters is one first on its line, with a space after it.
                self.only_space_before(self.i) && matches!(self.at(m.len()), 0 | b' ' | b'\t')
            } else {
                !sp.comment_at_word || matches!(self.before(self.i), 0 | b' ' | b'\t')
            }
        })
    }

    fn ident_start(&self, sp: &LangSpec) -> bool {
        let c = self.s[self.i];
        if is_ident(c) || (c == b'$' && !sp.dollar_var) {
            return true;
        }
        let dash = sp.ident_dash && c == b'-' && self.at(1).is_ascii_alphabetic();
        dash && matches!(self.before(self.i), 0 | b' ' | b'(')
    }

    /// Reads a word and colours it by the sets, then by what follows it.
    fn lang_ident(&mut self, sp: &LangSpec, selector: bool) {
        let a = self.i;
        self.i += 1;
        while !self.done() {
            let c = self.s[self.i];
            let joins = c == b'-' && sp.ident_dash && is_ident(self.at(1));
            if !(is_ident(c) || (c == b'$' && !sp.dollar_var) || joins) {
                break;
            }
            self.i += 1;
        }
        let b = self.i;
        let k = self.next_nonspace(b);
        let (nc, n2) = (self.byte(k), self.byte(k + 1));
        let (call, dot, first) = (nc == b'(', self.before(a) == b'.', self.s[a]);
        let lead = if dot { a - 1 } else { a };
        let is = |set: &str| self.word_in(set, a, b, sp.fold);
        let assigns = (nc == b'=' && n2 != b'=') || (n2 == b'=' && matches!(nc, b':' | b'?' | b'+'));
        let label = sp.label_colon && nc == b':' && n2 != b'=' && self.only_space_before(lead);
        let key = (sp.key_colon && nc == b':' && n2 != b':')
            || (sp.key_equals && self.only_space_before(a) && assigns);
        let kind = if selector {
            if dot { TokenKind::Type } else { TokenKind::Tag }
        } else if dot && !sp.no_dot {
            if call { TokenKind::Function } else { TokenKind::Property }
        } else if is(sp.keywords) {
            TokenKind::Keyword
        } else if is(sp.control) {
            TokenKind::Control
        } else if is(sp.constants) {
            TokenKind::Constant
        } else if is(sp.types) {
            TokenKind::Type
        } else if is(sp.builtins) || label {
            TokenKind::Function
        } else if key {
            TokenKind::Property
        } else if sp.macro_bang && k == b && nc == b'!' && n2 != b'=' {
            self.i += 1;
            TokenKind::Function
        } else if call || (sp.ident_dash && first.is_ascii_uppercase() && self.s[a..b].contains(&b'-')) {
            TokenKind::Function // a call, or a command of two words with a dash
        } else if first == b'-' && sp.ident_dash {
            TokenKind::Attribute
        } else if sp.cap_type && first.is_ascii_uppercase() {
            let caps = b - a >= 2 && !self.s[a..b].iter().any(u8::is_ascii_lowercase);
            if caps { TokenKind::Constant } else { TokenKind::Type }
        } else {
            return;
        };
        self.add(a, kind);
    }

    /// A `#` directive; the `<path>` of an include is a string.
    fn preproc(&mut self) {
        let a = self.i;
        self.i += 1;
        self.skip_space();
        self.skip_while(is_ident);
        self.add(a, TokenKind::Keyword);
        self.skip_space();
        if self.at(0) == b'<' {
            self.through(self.i, b'>', TokenKind::Str);
        }
    }

    /// `#id` and `:pseudo` in selectors, and `#hex` colours in declarations.
    fn css_mark(&mut self, selector: bool) {
        let a = self.i;
        self.i += 1;
        if self.at(0) == b':' {
            self.i += 1;
        }
        self.skip_while(|c| is_ident(c) || c == b'-');
        let kind = match (selector, self.s[a] == b'#') {
            (false, true) => TokenKind::Number,
            (true, true) => TokenKind::Constant,
            (true, false) => TokenKind::Keyword,
            (false, false) => {
                // A value after a colon: the colon alone.
                self.i = a + 1;
                TokenKind::Operator
            }
        };
        self.add(a, kind);
    }
}
