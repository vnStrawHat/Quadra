//! The tokenizers, a line at a time: each token written `text:kind`, lines in order so that
//! what one leaves open the next starts in.

use quadra_code::{Token, Tokenizer, tokenizer_for};

fn written(line: &str, tokens: &[Token]) -> String {
    let one = |t: &Token| format!("{}:{}", &line[t.start..t.end], format!("{:?}", t.kind).to_lowercase());
    tokens.iter().map(one).collect::<Vec<_>>().join(" ")
}

fn step(lang: &str, line: &str, state: u32) -> (String, u32) {
    let mut out = Vec::new();
    let next = tokenizer_for(lang).tokenize(line, state, &mut out);
    (written(line, &out), next)
}

fn spans(lang: &str, lines: &[&str]) -> Vec<String> {
    let mut state = 0;
    let mut line = |line: &&str| {
        let (text, next) = step(lang, line, state);
        state = next;
        text
    };
    lines.iter().map(&mut line).collect()
}

#[track_caller]
fn has(line: &str, items: &[&str]) {
    for item in items {
        assert!(line.contains(item), "missing {item} in: {line}");
    }
}

#[test]
fn v_has_its_words_its_strings_and_comments_that_nest() {
    let s = spans("v", &["pub fn (mut d Doc) move(m Move) int {"]);
    assert_eq!(
        s[0],
        "pub:keyword fn:keyword (:punctuation mut:keyword Doc:type ):punctuation move:function \
         (:punctuation Move:type ):punctuation int:type {:punctuation"
    );
    let s =
        spans("v", &[r"x := 'a\n${b.c}' // done", "if a.len > 0x1F { return none }", r"@[heap] r'\d' `a`"]);
    assert_eq!(s[0], r":=:operator 'a:str \n${:escape b.c:variable }:escape ':str // done:comment");
    assert_eq!(
        s[1],
        "if:control .:punctuation len:property >:operator 0x1F:number {:punctuation return:control \
         none:constant }:punctuation"
    );
    assert_eq!(s[2], r"@[heap]:attribute r'\d':str `a`:str");
    // A comment in a comment, and a string, go on over lines.
    let (s, state) = step("v", "a /* x /* y */", 0);
    assert_eq!((s.as_str(), state), ("/* x /* y */:comment", 1));
    assert_eq!(step("v", "still */ b()", state), ("still */:comment b:function ():punctuation".into(), 0));
    let (_, state) = step("v", "s := 'one", 0);
    assert!(state > 0);
    assert_eq!(step("v", "two' + 1", state), ("two':str +:operator 1:number".into(), 0));
}

#[test]
fn yaml_has_keys_values_and_scalars_in_blocks() {
    let s = spans(
        "yaml",
        &[
            "apiVersion: apps/v1 # api",
            "  - name: \"web:1\"",
            "replicas: 3",
            "on: [true, null, 1.5e3]",
            "base: &b !!map {a: *x}",
            "--- # doc",
            "url: http://x:80",
        ],
    );
    assert_eq!(s[0], "apiVersion:property ::punctuation apps/v1:str # api:comment");
    assert_eq!(s[1], "-:punctuation name:property ::punctuation \"web:1\":str");
    assert_eq!(s[2], "replicas:property ::punctuation 3:number");
    assert_eq!(
        s[3],
        "on:property ::punctuation [:punctuation true:constant ,:punctuation null:constant ,:punctuation \
         1.5e3:number ]:punctuation"
    );
    assert_eq!(
        s[4],
        "base:property ::punctuation &b:attribute !!map:tag {:punctuation a:property ::punctuation \
         *x:attribute }:punctuation"
    );
    assert_eq!(s[5], "---:keyword # doc:comment");
    assert_eq!(s[6], "url:property ::punctuation http://x:80:str");
    // A block scalar lasts while lines are blank or deeper than its key.
    let (_, state) = step("yaml", "  - run: |", 0);
    assert!(state > 0);
    let (s, state) = step("yaml", "      echo # not a comment", state);
    assert_eq!(s, "echo # not a comment:str");
    let (_, state) = step("yaml", "", state);
    assert!(state > 0);
    assert_eq!(step("yaml", "    next: 1", state), ("next:property ::punctuation 1:number".into(), 0));
    // A colon with no value after it in a flow is read, and the line ends.
    assert_eq!(spans("yaml", &["{a:,}"])[0], "{:punctuation a:str :,}:punctuation");
}

#[test]
fn the_languages_like_c() {
    let s = spans(
        "c",
        &[
            "#include <stdio.h>",
            "static int f(void) { return 0x1F; } // x",
            "/* a",
            r#"b */ char *s = "a\n";"#,
        ],
    );
    assert_eq!(s[0], "#include:keyword <stdio.h>:str");
    has(
        &s[1],
        &["static:keyword", "int:type", "f:function", "return:control", "0x1F:number", "// x:comment"],
    );
    assert_eq!(s[2], "/* a:comment");
    has(&s[3], &["b */:comment", "\"a:str", r"\n:escape"]);
    let s = spans("java", &["@Override public String name() { return null; }"]);
    has(&s[0], &["@Override:attribute", "public:keyword", "String:type", "null:constant"]);
    let s = spans("kotlin", &["val s = \"hi $name ${a.b}\""]);
    has(&s[0], &["val:keyword", "$name:variable", "${:escape", "a.b:variable"]);
    let s = spans("swift", &[r#"let s = "v \(x)" /* a /* b */ c */ + 1"#]);
    has(&s[0], &["let:keyword", r"\(:escape", "x:variable", "/* a /* b */ c */:comment", "1:number"]);
    let s = spans("go", &["func main() { s := `raw", "still` }"]);
    has(&s[0], &["func:keyword", "main:function", "`raw:str"]);
    has(&s[1], &["still`:str", "}:punctuation"]);
    let s = spans(
        "rs",
        &[
            r##"fn f<'a>(x: &'a str) -> char { let r = r#"q"#; println!("{}"); 'x' }"##,
            "#[derive(Debug)] let n = 1_000.5e-3;",
        ],
    );
    has(&s[0], &["fn:keyword", "<:operator 'a:attribute >:operator", "&:operator 'a:attribute str:type"]);
    has(&s[0], &[r##"r#"q"#:str"##, "println!:function", "'x':str"]);
    has(&s[1], &["#[derive(Debug)]:attribute", "let:keyword", "1_000.5e-3:number"]);
    // Two hashes and more: closed by a quote and as many hashes, over lines too.
    let first = r####"let a = r##"x"#y"##; let b = r###"one"###;"####;
    let s = spans("rs", &[first, r####"let c = r##"q"#"####, r####"z"## + 1;"####]);
    has(&s[0], &[r####"r##"x"#y"##:str"####, r####"r###"one"###:str"####, "let:keyword"]);
    has(&s[1], &[r####"r##"q"#:str"####]);
    has(&s[2], &[r####"z"##:str"####, "+:operator"]);
    let s = spans("javascript", &["const a = `x ${b}", "y`; if (a) { return f(1) } // c"]);
    has(&s[0], &["const:keyword", "`x :str", "${:escape", "b:variable"]);
    has(&s[1], &["y`:str", "if:control", "return:control", "f:function", "// c:comment"]);
    let s = spans("ts", &["interface A { n: number; s: string }"]);
    has(&s[0], &["interface:keyword", "A:type", "number:type"]);
}

#[test]
fn the_languages_of_scripts_and_shells() {
    let s = spans(
        "python",
        &["@dataclass", "def f(x: int) -> None:  # c", "s = f\"a\" + \"\"\"doc", "end\"\"\" + 1.5"],
    );
    assert_eq!(s[0], "@dataclass:attribute");
    has(&s[1], &["def:keyword", "f:function", "int:type", "None:constant", "# c:comment"]);
    has(&s[2], &["f\"a\":str", "\"\"\"doc:str"]);
    has(&s[3], &["end\"\"\":str", "1.5:number"]);
    let s = spans("ruby", &["def hi; puts \"a #{@x}\" end"]);
    has(&s[0], &["def:keyword", "#{:escape", "@x:variable", "end:keyword"]);
    let s = spans("lua", &["local x = [[a", "b]] --[[ c", "d ]] -- e"]);
    has(&s[0], &["local:keyword", "[[a:str"]);
    has(&s[1], &["b]]:str", "--[[ c:comment"]);
    assert_eq!(s[2], "d ]]:comment -- e:comment");
    let s = spans("php", &["<?php echo \"v $x\"; // c"]);
    has(&s[0], &["php:keyword", "echo:keyword", "$x:variable", "// c:comment"]);
    let s = spans("bash", &["if [ -f \"$HOME/x\" ]; then echo 'a' # c", "a#b X=1 ${y}"]);
    has(&s[0], &["if:control", "$HOME:variable", "then:control", "echo:function", "'a':str", "# c:comment"]);
    assert!(!s[1].contains("comment"), "{}", s[1]);
    let s = spans("pwsh", &["Get-Item -Path $p -eq 1 <# a", "b #>"]);
    has(&s[0], &["Get-Item:function", "-Path:attribute", "$p:variable", "-eq:keyword", "<# a:comment"]);
    assert_eq!(s[1], "b #>:comment");
    let s = spans("bat", &["@echo off", "REM note", ":loop", "set X=%PATH%"]);
    has(&s[0], &["echo:keyword", "off:keyword"]);
    assert_eq!((s[1].as_str(), s[2].as_str()), ("REM note:comment", ":loop:function"));
    has(&s[3], &["set:keyword", "%PATH%:variable"]);
    let s = spans("SQL", &["SELECT name, COUNT(*) FROM t WHERE x = 'a' -- c"]);
    has(&s[0], &["SELECT:keyword", "COUNT:function", "FROM:keyword", "'a':str", "-- c:comment"]);
    let s = spans("Makefile", &["CC := gcc", "all: $(OBJ)", "\t$(CC) -o $@"]);
    has(&s[0], &["CC:property"]);
    has(&s[1], &["all:function", "$(OBJ):variable"]);
    has(&s[2], &["$(CC):variable", "$@:variable"]);
    let s = spans("dockerfile", &["FROM alpine AS base", "RUN echo $HOME # c"]);
    has(&s[0], &["FROM:keyword", "AS:keyword"]);
    has(&s[1], &["RUN:keyword", "$HOME:variable", "# c:comment"]);
}

#[test]
fn the_formats_of_data_style_and_markup() {
    let s = spans("json", &[r#"{"a": [1, true, null], "b": "x\"y"}"#]);
    has(
        &s[0],
        &["\"a\":property", "1:number", "true:constant", "null:constant", "\"b\":property", "\\\":escape"],
    );
    let s = spans("toml", &["[tool.x]", "name = \"q\" # c", "n = 0x1f"]);
    assert_eq!(s[0], "[tool.x]:tag");
    has(&s[1], &["name:property", "\"q\":str", "# c:comment"]);
    has(&s[2], &["n:property", "0x1f:number"]);
    let s = spans("ini", &["; c", "[core]", "editor = vim"]);
    assert_eq!((s[0].as_str(), s[1].as_str()), ("; c:comment", "[core]:tag"));
    has(&s[2], &["editor:property"]);
    let s =
        spans("scss", &[".btn:hover, #id {", "  max-width: 10px; color: #fff; // c", "}", "@media (x) {"]);
    has(&s[0], &["btn:type", ":hover:keyword", "#id:constant"]);
    has(&s[1], &["max-width:property", "10px:number", "#fff:number", "// c:comment"]);
    has(&s[3], &["@media:keyword"]);
    let s = spans(
        "html",
        &["<!DOCTYPE html>", "<a href=\"x\" data-id=1>T &amp; U</a> <!-- c", "d --> <img", "  src=\"y\" />"],
    );
    has(&s[0], &["<!:punctuation", "DOCTYPE:keyword"]);
    let in_tag = ["<:punctuation", "a:tag", "href:attribute", "\"x\":str", "data-id:attribute"];
    has(&s[1], &in_tag);
    has(&s[1], &["&amp;:escape", "</:punctuation", "<!-- c:comment"]);
    has(&s[2], &["d -->:comment", "img:tag"]);
    has(&s[3], &["src:attribute", "\"y\":str", "/>:punctuation"]);
    let s = spans("patch", &["--- a/x", "@@ -1 +1 @@", "-old", "+new", " same"]);
    assert_eq!(s, ["--- a/x:keyword", "@@ -1 +1 @@:escape", "-old:variable", "+new:str", ""]);
}

#[test]
fn markdown_has_its_blocks_and_the_marks_in_a_line() {
    assert_eq!(step("md", "## Title", 0), ("## Title:tag".into(), 0));
    assert_eq!(step("md", "```v", 0), ("```:punctuation v:type".into(), 1));
    assert_eq!(step("md", "fn main() {}", 1), ("fn main() {}:str".into(), 1));
    assert_eq!(step("md", "```", 1), ("```:punctuation".into(), 0));
    assert_eq!(step("md", "$$", 0).1, 3);
    assert_eq!(step("md", "x^2 $$", 3), ("x^2 $$:escape".into(), 0));
    let (s, _) = step("md", "- [x] **done** and *maybe* `code` [a](http://b) $y$", 0);
    assert_eq!(
        s,
        "-:operator [x]:operator **done**:number *maybe*:keyword `code`:str [a]:function \
         (http://b):comment $y$:escape"
    );
    assert_eq!(step("md", "> quoted snake_case", 0).0, ">:comment");
    assert_eq!(step("md", "---", 0).0, "---:punctuation");
    assert_eq!(
        step("md", "see <https://a.b/c> and http://d", 0).0,
        "<https://a.b/c>:comment http://d:comment"
    );
}

#[test]
fn a_tokenizer_is_found_by_a_name_of_a_file_or_of_a_fence() {
    let names = [
        "x.py",
        "Python",
        "python title=x",
        "py {.cls}",
        "src/A.RS",
        "rust",
        "C++",
        "cpp",
        "h",
        "cs",
        "csharp",
        "x.jsx",
        "js {.x}",
        "mjs",
        "TSX",
        "sh",
        "zsh",
        "pwsh",
        "cmd",
        "Dockerfile",
        "GNUmakefile",
        "make",
        "jsonc",
        "conf",
        "svg",
        "xml",
        "diff",
        "kt",
        "swift",
        "dart",
        "rb",
        "lua",
        "sql",
        "toml",
        "css",
        "go",
        "java",
        "php",
        "my file.py",
        "C:\\work\\main.c",
    ];
    for name in names {
        assert!(matches!(tokenizer_for(name), Tokenizer::Lang(_)), "{name}");
    }
    assert_eq!(tokenizer_for("a/b.V"), Tokenizer::V);
    assert_eq!(tokenizer_for("deploy.yml"), Tokenizer::Yaml);
    assert_eq!(tokenizer_for("md"), Tokenizer::Markdown);
    for name in ["README", "nonsense", ""] {
        assert_eq!(tokenizer_for(name), Tokenizer::Plain, "{name}");
    }
}
