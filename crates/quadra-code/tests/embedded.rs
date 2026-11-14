//! Code in markup and patterns in scripts (specs/13, "Code"): what a `<script>` or a
//! `<style>` holds is read by the rows of its language, over lines; a `/` where a value
//! starts opens a pattern, and after a value divides.

use quadra_code::{Token, tokenizer_for};

/// The tokens of the lines, each written `text:kind`, what one leaves open the next starts in.
fn spans(lang: &str, lines: &[&str]) -> Vec<String> {
    let mut state = 0;
    let mut out = Vec::new();
    for line in lines {
        let mut tokens: Vec<Token> = Vec::new();
        state = tokenizer_for(lang).tokenize(line, state, &mut tokens);
        let one = |t: &Token| format!("{}:{}", &line[t.start..t.end], format!("{:?}", t.kind).to_lowercase());
        out.push(tokens.iter().map(one).collect::<Vec<_>>().join(" "));
    }
    out
}

#[track_caller]
fn has(line: &str, items: &[&str]) {
    for item in items {
        assert!(line.contains(item), "missing {item} in: {line}");
    }
}

#[test]
fn what_a_script_or_a_style_holds_is_read_as_its_language() {
    let s = spans(
        "html",
        &[
            "<script type=\"module\">const a = \"</b>\"; /* x",
            "y */ if (a) {}</script><p class=x>",
            "<STYLE>.btn {",
            "  color: #fff; }</STYLE> <b>",
        ],
    );
    has(&s[0], &["script:tag", "type:attribute", "const:keyword", "\"</b>\":str", "/* x:comment"]);
    // A comment goes on into the next line, and the tag that ends the script is markup again.
    has(&s[1], &["y */:comment", "if:control", "</:punctuation", "script:tag", "p:tag", "class:attribute"]);
    has(&s[2], &["STYLE:tag", "btn:type"]);
    has(&s[3], &["color:property", "#fff:number", "STYLE:tag", "b:tag"]);
    // Outside them, words are text.
    let s = spans("html", &["<p>const a = 1</p>"]);
    assert!(!s[0].contains("const:keyword"), "{}", s[0]);
}

#[test]
fn a_slash_where_a_value_starts_opens_a_pattern() {
    let s =
        spans("js", &["let r = /a[/]b\\/c/gi, n = x / 2 / y;", "return /^#/.test(s) ? (a) / b : [1] / 2;"]);
    has(&s[0], &["/a[/]b\\/c/gi:regexp"]);
    assert_eq!(s[0].matches(":regexp").count(), 1, "{}", s[0]);
    has(&s[1], &["/^#/:regexp"]);
    assert_eq!(s[1].matches(":regexp").count(), 1, "{}", s[1]);
    // A slash with no end on its line divides.
    let s = spans("ts", &["a = b /c"]);
    assert!(!s[0].contains("regexp"), "{}", s[0]);
}
