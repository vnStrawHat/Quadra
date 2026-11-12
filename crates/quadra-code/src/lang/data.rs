//! Rows: the formats of data, of style and of markup, and diffs.

use super::{LangSpec, Mode, PLAIN};

pub const JSON: LangSpec = LangSpec {
    names: "json jsonc json5 geojson webmanifest jsonl ndjson",
    constants: "true false null",
    line_comments: &["//"],
    block_open: "/*",
    block_close: "*/",
    quotes: "\"",
    key_colon: true,
    ..PLAIN
};

pub const TOML: LangSpec = LangSpec {
    names: "toml",
    constants: "true false inf nan",
    line_comments: &["#"],
    quotes: "\"'",
    raw_quotes: "'",
    triple: true,
    key_equals: true,
    sections: true,
    ..PLAIN
};

pub const INI: LangSpec = LangSpec {
    names: "ini conf cfg properties editorconfig gitconfig desktop service env dotenv",
    constants: "true false yes no on off",
    fold: true,
    line_comments: &[";", "#"],
    quotes: "\"'",
    no_dot: true,
    ident_dash: true,
    key_equals: true,
    sections: true,
    ..PLAIN
};

pub const CSS: LangSpec = LangSpec {
    names: "css scss sass less pcss postcss",
    keywords: "important",
    line_comments: &["//"],
    comment_at_word: true,
    block_open: "/*",
    block_close: "*/",
    quotes: "\"'",
    dollar_var: true,
    at_kw: true,
    ident_dash: true,
    key_colon: true,
    selectors: true,
    ..PLAIN
};

pub const MARKUP: LangSpec = LangSpec {
    names: "html htm xhtml xml svg xsl xslt xsd plist rss atom csproj vcxproj props",
    mode: Mode::Markup,
    ..PLAIN
};

pub const DIFF: LangSpec = LangSpec { names: "diff patch", mode: Mode::Diff, ..PLAIN };
