//! Tokens (behaviour/18, "Layers"). A tokenizer works a line at a time and gives the state
//! the next line starts in: what is open at the end of a line (a comment, a string) is all
//! a line needs to know of the lines before it.

use crate::lang::{self, LangSpec};
use crate::{md, v, yaml};

/// What a token is: the kinds a theme gives colours to (its `syntax`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Keyword,
    Control,
    Str,
    Number,
    Constant,
    Type,
    Function,
    Variable,
    Property,
    Operator,
    Punctuation,
    Comment,
    Tag,
    Attribute,
    Regexp,
    Escape,
    Invalid,
}

/// The bytes `start..end` of a line, on character boundaries. What no token covers is
/// plain text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub start: usize,
    pub end: usize,
    pub kind: TokenKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tokenizer {
    /// No tokens: plain text.
    Plain,
    V,
    Yaml,
    Markdown,
    /// One of the languages of the table.
    Lang(&'static LangSpec),
}

impl Tokenizer {
    /// Adds the tokens of `line` to `out`, in order, and gives the state the next line
    /// starts in. The first line of a text starts in the state 0.
    pub fn tokenize(self, line: &str, state: u32, out: &mut Vec<Token>) -> u32 {
        match self {
            Tokenizer::Plain => 0,
            Tokenizer::V => v::tokenize(line, state, out),
            Tokenizer::Yaml => yaml::tokenize(line, state, out),
            Tokenizer::Markdown => md::tokenize(line, state, out),
            Tokenizer::Lang(spec) => lang::tokenize(spec, line, state, out),
        }
    }

    fn named(name: &str) -> Option<Tokenizer> {
        match name {
            "v" | "vsh" | "vlang" => Some(Tokenizer::V),
            "yaml" | "yml" => Some(Tokenizer::Yaml),
            "md" | "markdown" => Some(Tokenizer::Markdown),
            name => lang::named(name).map(Tokenizer::Lang),
        }
    }
}

/// The tokenizer for the name of a file (`src/a.rs`, `Dockerfile`) or for what follows a
/// fence (`python`, `js {.class}`, `rust title=main.rs`), whatever the case: by all of the
/// name, then by its first word, then by what is after the last dot.
pub fn tokenizer_for(path: &str) -> Tokenizer {
    let lower = path.to_lowercase();
    let name = lower.split('{').next().unwrap_or("").trim();
    let name = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let word = name.split(' ').next().unwrap_or(name);
    let after_dot = |name: &str| name.rsplit('.').next().map(str::to_string).unwrap_or_default();
    let tries = [name.to_string(), word.to_string(), after_dot(name), after_dot(word)];
    tries.iter().find_map(|name| Tokenizer::named(name)).unwrap_or(Tokenizer::Plain)
}
