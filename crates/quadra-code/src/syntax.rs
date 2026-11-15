//! The colour of each kind of token: a role of the theme (a theme file's `syntax`).

use std::ops::Range;

use quadra_design::token;
use quadra_tree::Token as Role;

use crate::token::{TokenKind, tokenizer_for};

/// The colours of each line of a text in the language of `path`, read in order: for the
/// kit's diff, `diff(old, new).colours(line_colours("main.rs"))`.
pub fn line_colours(path: &str) -> impl Fn(&str) -> Vec<Vec<(Range<usize>, Role)>> + use<> {
    let tokenizer = tokenizer_for(path);
    move |text| {
        let (mut state, mut tokens) = (0, Vec::new());
        let line = |line: &str| {
            let line = line.strip_suffix('\r').unwrap_or(line);
            tokens.clear();
            state = tokenizer.tokenize(line, state, &mut tokens);
            tokens.iter().map(|t| (t.start..t.end, role(t.kind))).collect()
        };
        text.split('\n').map(line).collect()
    }
}

/// Every kind, in the order of their numbers.
pub(crate) const KINDS: [TokenKind; 17] = [
    TokenKind::Keyword,
    TokenKind::Control,
    TokenKind::Str,
    TokenKind::Number,
    TokenKind::Constant,
    TokenKind::Type,
    TokenKind::Function,
    TokenKind::Variable,
    TokenKind::Property,
    TokenKind::Operator,
    TokenKind::Punctuation,
    TokenKind::Comment,
    TokenKind::Tag,
    TokenKind::Attribute,
    TokenKind::Regexp,
    TokenKind::Escape,
    TokenKind::Invalid,
];

pub fn role(kind: TokenKind) -> Role {
    match kind {
        TokenKind::Keyword => token::SYNTAX_KEYWORD,
        TokenKind::Control => token::SYNTAX_CONTROL,
        TokenKind::Str => token::SYNTAX_STRING,
        TokenKind::Number => token::SYNTAX_NUMBER,
        TokenKind::Constant => token::SYNTAX_CONSTANT,
        TokenKind::Type => token::SYNTAX_TYPE,
        TokenKind::Function => token::SYNTAX_FUNCTION,
        TokenKind::Variable => token::SYNTAX_VARIABLE,
        TokenKind::Property => token::SYNTAX_PROPERTY,
        TokenKind::Operator => token::SYNTAX_OPERATOR,
        TokenKind::Punctuation => token::SYNTAX_PUNCTUATION,
        TokenKind::Comment => token::SYNTAX_COMMENT,
        TokenKind::Tag => token::SYNTAX_TAG,
        TokenKind::Attribute => token::SYNTAX_ATTRIBUTE,
        TokenKind::Regexp => token::SYNTAX_REGEXP,
        TokenKind::Escape => token::SYNTAX_ESCAPE,
        TokenKind::Invalid => token::SYNTAX_INVALID,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_colours_of_lines_are_read_in_order() {
        let lines = line_colours("main.rs")("fn a() {}\r\n/* one\ntwo */ x");
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains(&(0..2, role(TokenKind::Keyword))));
        // The comment goes on into the next line: its start is the line's.
        assert_eq!(lines[2].first(), Some(&(0..6, role(TokenKind::Comment))));
    }
}
