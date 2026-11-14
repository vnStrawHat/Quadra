//! The languages of the table (behaviour/18, "Tokenizers"): one row a language, one scanner
//! for all of them. A row says what the words of a language are and how its comments,
//! strings and marks are written.

mod c;
mod data;
mod markup;
mod scan;
mod script;
mod strings;

pub use scan::tokenize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Code,
    /// Tags, attributes, entities and comments: HTML, XML, SVG.
    Markup,
    /// Whole lines, by what they start with.
    Diff,
}

/// A row. Sets of words are words with a space between them, in lower case when `fold`;
/// sets of quotes and prefixes are the bytes of a text.
#[derive(Debug, PartialEq)]
pub struct LangSpec {
    /// The names after a fence, other names and file extensions, in lower case.
    pub names: &'static str,
    pub mode: Mode,
    pub keywords: &'static str,
    pub control: &'static str,
    pub types: &'static str,
    pub constants: &'static str,
    /// Coloured as functions.
    pub builtins: &'static str,
    /// Words are what they are whatever their case.
    pub fold: bool,
    /// A marker of letters (`rem`) counts only first on a line.
    pub line_comments: &'static [&'static str],
    /// A comment marker counts only where a word starts (the `#` of shells).
    pub comment_at_word: bool,
    pub block_open: &'static str,
    pub block_close: &'static str,
    /// Block comments nest.
    pub nest: bool,
    /// The quotes of strings that end with their line at the latest.
    pub quotes: &'static str,
    /// The quotes of strings that go on over lines.
    pub multi_quotes: &'static str,
    /// The quotes of strings without escapes.
    pub raw_quotes: &'static str,
    /// Three quotes open a string that goes on over lines.
    pub triple: bool,
    /// Letters before a quote: `r b f u` (`r` and `@` make the string raw).
    pub prefixes: &'static str,
    /// What opens an expression in a string: `${`, `#{`, `\(`, `$(`.
    pub interps: &'static [&'static str],
    /// The quotes of the strings that have them.
    pub interp_in: &'static str,
    /// `[[ ]]` strings.
    pub long_brackets: bool,
    /// `$name`, `${name}` and `$(name)` are variables.
    pub dollar_var: bool,
    /// `%name%` and `%1` are variables.
    pub percent_var: bool,
    /// `@name` is an attribute, a variable, or a keyword.
    pub at_attr: bool,
    pub at_var: bool,
    pub at_kw: bool,
    /// `#` directives at the start of a line.
    pub preproc: bool,
    /// `#[..]` attributes.
    pub hash_attr: bool,
    /// `'a` is a lifetime unless it is a character.
    pub lifetimes: bool,
    /// `name!` is a call.
    pub macro_bang: bool,
    /// Words that start with a capital are types, and constants when all of them is.
    pub cap_type: bool,
    /// A word after a dot is not a property (the paths of shells).
    pub no_dot: bool,
    /// `-` joins words; a word that starts with one is an attribute.
    pub ident_dash: bool,
    /// A word or a string before `:` is a property.
    pub key_colon: bool,
    /// A first word before `=` is a property.
    pub key_equals: bool,
    /// `[section]` lines are tags.
    pub sections: bool,
    /// A first word before `:` is a function (the targets of a makefile).
    pub label_colon: bool,
    /// A line that starts with `:name` is a function (a label).
    pub colon_label: bool,
    /// The words of a line with `{` are selectors.
    pub selectors: bool,
    /// A `/` where a value starts opens a pattern (`/a+/g`); after a value it divides.
    pub slash_regex: bool,
}

/// A row that says nothing: rows are written as what they have otherwise.
pub const PLAIN: LangSpec = LangSpec {
    names: "",
    mode: Mode::Code,
    keywords: "",
    control: "",
    types: "",
    constants: "",
    builtins: "",
    fold: false,
    line_comments: &[],
    comment_at_word: false,
    block_open: "",
    block_close: "",
    nest: false,
    quotes: "",
    multi_quotes: "",
    raw_quotes: "",
    triple: false,
    prefixes: "",
    interps: &[],
    interp_in: "",
    long_brackets: false,
    dollar_var: false,
    percent_var: false,
    at_attr: false,
    at_var: false,
    at_kw: false,
    preproc: false,
    hash_attr: false,
    lifetimes: false,
    macro_bang: false,
    cap_type: false,
    no_dot: false,
    ident_dash: false,
    key_colon: false,
    key_equals: false,
    sections: false,
    label_colon: false,
    colon_label: false,
    selectors: false,
    slash_regex: false,
};

/// Every row.
static ALL: [&LangSpec; 27] = [
    &c::C,
    &c::CPP,
    &c::CS,
    &c::JAVA,
    &c::KOTLIN,
    &c::SWIFT,
    &c::GO,
    &c::RUST,
    &c::JS,
    &c::TS,
    &c::DART,
    &script::PYTHON,
    &script::RUBY,
    &script::PHP,
    &script::LUA,
    &script::SH,
    &script::PS,
    &script::BAT,
    &script::SQL,
    &script::MAKE,
    &script::DOCKER,
    &data::JSON,
    &data::TOML,
    &data::INI,
    &data::CSS,
    &data::MARKUP,
    &data::DIFF,
];

/// The row of a name, in lower case.
pub fn named(name: &str) -> Option<&'static LangSpec> {
    ALL.iter().copied().find(|spec| !name.is_empty() && spec.names.split(' ').any(|known| known == name))
}
