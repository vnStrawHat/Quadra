//! Rows: the scripting languages, the shells, SQL and build files.

use super::{LangSpec, PLAIN};

// ponytail: the fields of f-strings, heredocs and the =begin blocks of Ruby read as plain
// string or as code.
pub const PYTHON: LangSpec = LangSpec {
    names: "python py python3 py3 pyw pyi gyp",
    keywords: "def class lambda import from as global nonlocal pass del in is not and or async await with assert yield self cls",
    control: "if elif else for while break continue return try except finally raise match case",
    types: "int float str bool bytes list dict set tuple object complex",
    constants: "True False None",
    line_comments: &["#"],
    quotes: "\"'",
    triple: true,
    prefixes: "rbfuRBFU",
    at_attr: true,
    cap_type: true,
    ..PLAIN
};

pub const RUBY: LangSpec = LangSpec {
    names: "ruby rb rake gemspec",
    keywords: "def class module end do begin rescue ensure alias and or not self super yield lambda proc require require_relative include extend attr_accessor attr_reader attr_writer private protected public then",
    control: "if elsif else unless case when while until for in break next redo retry return",
    constants: "true false nil",
    line_comments: &["#"],
    quotes: "\"'",
    multi_quotes: "`",
    interps: &["#{"],
    interp_in: "\"`",
    at_var: true,
    cap_type: true,
    ..PLAIN
};

pub const PHP: LangSpec = LangSpec {
    names: "php phtml",
    keywords: "function class interface trait extends implements namespace use new public private protected static abstract final const var echo print global as instanceof clone include include_once require require_once fn match readonly enum php array list isset unset empty",
    control: "if else elseif for foreach while do switch case default break continue return try catch finally throw yield",
    types: "int float string bool object mixed void callable iterable",
    constants: "true false null",
    fold: true,
    line_comments: &["//", "#"],
    block_open: "/*",
    block_close: "*/",
    quotes: "\"'",
    interp_in: "\"",
    dollar_var: true,
    cap_type: true,
    ..PLAIN
};

pub const LUA: LangSpec = LangSpec {
    names: "lua luau",
    keywords: "local function end and or not in then do self",
    control: "if elseif else for while repeat until return break goto",
    constants: "true false nil",
    line_comments: &["--"],
    block_open: "--[[",
    block_close: "]]",
    quotes: "\"'",
    long_brackets: true,
    ..PLAIN
};

/// The quotes of a shell end with their line: one stray apostrophe must not colour the rest.
pub const SH: LangSpec = LangSpec {
    names: "sh bash shell zsh ksh fish console terminal shellsession bashrc zshrc profile ebuild",
    keywords: "function local export readonly declare typeset unset alias source in time",
    control: "if then else elif fi for while until do done case esac return break continue exit select",
    builtins: "echo printf cd pwd read test eval exec set shift trap wait kill cat ls grep sed awk mkdir rm cp mv ln chmod chown curl wget git sudo apt brew npm make tar find xargs touch head tail sort uniq wc cut tr tee env which ssh docker",
    constants: "true false",
    line_comments: &["#"],
    comment_at_word: true,
    quotes: "\"'`",
    raw_quotes: "'",
    interps: &["${", "$("],
    interp_in: "\"`",
    dollar_var: true,
    no_dot: true,
    key_equals: true,
    ..PLAIN
};

pub const PS: LangSpec = LangSpec {
    names: "powershell ps1 psm1 psd1 pwsh posh",
    keywords: "function param begin process end filter class enum using module -eq -ne -gt -ge -lt -le -like -notlike -match -notmatch -contains -notcontains -in -notin -and -or -not -xor -is -isnot -replace -split -join -f",
    control: "if elseif else switch foreach for while do until break continue return try catch finally throw trap exit",
    fold: true,
    line_comments: &["#"],
    block_open: "<#",
    block_close: "#>",
    quotes: "\"'",
    raw_quotes: "'",
    interps: &["$("],
    interp_in: "\"",
    dollar_var: true,
    ident_dash: true,
    ..PLAIN
};

pub const BAT: LangSpec = LangSpec {
    names: "bat cmd batch dosbatch",
    keywords: "echo set setlocal endlocal call shift pause title cls start cd pushd popd del copy move mkdir rmdir dir type not exist defined errorlevel in do off on equ neq lss leq gtr geq",
    control: "if else for goto exit",
    fold: true,
    line_comments: &["rem", "::"],
    quotes: "\"",
    percent_var: true,
    colon_label: true,
    no_dot: true,
    ..PLAIN
};

pub const SQL: LangSpec = LangSpec {
    names: "sql mysql pgsql postgres postgresql sqlite plsql tsql psql",
    keywords: "select from where insert into values update set delete create table alter drop index view primary key foreign references join inner left right outer full cross on as and or not in is like ilike between exists distinct group by order having limit offset union all case when then else end asc desc default unique constraint check database schema if begin commit rollback transaction with returning grant revoke truncate replace cascade natural using over partition window add column",
    types: "int integer bigint smallint tinyint serial bigserial decimal numeric real float double precision char varchar text boolean bool date time timestamp timestamptz interval blob json jsonb uuid bytea",
    constants: "null true false",
    fold: true,
    line_comments: &["--"],
    block_open: "/*",
    block_close: "*/",
    quotes: "'\"`",
    raw_quotes: "'\"`",
    ..PLAIN
};

pub const MAKE: LangSpec = LangSpec {
    names: "makefile gnumakefile make mk mak",
    keywords: "ifeq ifneq ifdef ifndef else endif include define endef export unexport override vpath",
    line_comments: &["#"],
    quotes: "\"'",
    dollar_var: true,
    no_dot: true,
    key_equals: true,
    label_colon: true,
    ..PLAIN
};

// ponytail: the words of instructions are known anywhere on a line, not only first.
pub const DOCKER: LangSpec = LangSpec {
    names: "dockerfile docker containerfile",
    keywords: "from run cmd label maintainer expose env add copy entrypoint volume user workdir arg onbuild stopsignal healthcheck shell as",
    fold: true,
    line_comments: &["#"],
    quotes: "\"'",
    dollar_var: true,
    no_dot: true,
    ..PLAIN
};
