//! Quadra's code (specs/13, "Code"; behaviour/18): the buffer that holds the text of a
//! document, and the tokenizers that colour code a line at a time. They are plain
//! functions over text: a file that is not ours costs what its length costs.

mod buffer;
mod lang;
mod lex;
mod md;
mod syntax;
mod token;
mod v;
mod yaml;

pub use buffer::Buffer;
pub use lang::LangSpec;
pub use syntax::role;
pub use token::{Token, TokenKind, Tokenizer, tokenizer_for};
