//! The colour of each kind of token: a role of the theme (a theme file's `syntax`).

use quadra_kit::token;
use quadra_tree::Token as Role;

use crate::token::TokenKind;

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
