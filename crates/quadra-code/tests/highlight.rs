//! The highlighter (behaviour/18, "Highlighting"): after any edits, the tokens of a line are
//! those of reading the text from its start, and a view far down reads only what it needs.

use quadra_code::{Doc, Highlighter, Token, tokenizer_for};
use quadra_testing::fuzz::Rng;

fn fresh(doc: &Doc, name: &str) -> Vec<Vec<Token>> {
    let t = tokenizer_for(name);
    let (mut state, mut all) = (0, Vec::new());
    for line in 0..doc.buffer.line_count() {
        let mut out = Vec::new();
        state = t.tokenize(&doc.buffer.line(line), state, &mut out);
        all.push(out);
    }
    all
}

#[test]
fn tokens_after_edits_are_those_of_reading_from_the_start() {
    let pieces = ["/*", "*/", "\"", "\n", "x", " ", "// c", "r#\"", "\"#", "fn a() {}\n"];
    let mut rng = Rng::new(7);
    for _ in 0..40 {
        let mut doc = Doc::new("fn main() {\n    let s = \"a\";\n}\n/* one\ntwo */ x\n");
        let mut h = Highlighter::new(tokenizer_for("a.rs"));
        let mut out = Vec::new();
        for _ in 0..30 {
            let at = rng.below(doc.buffer.len() + 1);
            let at = (0..=at).rev().find(|&i| doc.text().is_char_boundary(i)).unwrap_or(0);
            doc.cursors = vec![quadra_code::Cursor::at(at)];
            doc.insert_text(rng.pick(&pieces), false);
            h.sync(&mut doc);
            // A view looks at some lines; the rest are left stale.
            let line = rng.below(doc.buffer.line_count());
            h.tokens(&doc.buffer, line, &mut out);
            assert_eq!(out, fresh(&doc, "a.rs")[line]);
        }
        let want = fresh(&doc, "a.rs");
        for (line, want) in want.iter().enumerate() {
            h.tokens(&doc.buffer, line, &mut out);
            assert_eq!(&out, want, "line {line}");
        }
    }
}

#[test]
fn an_edit_deep_in_a_file_reads_only_from_its_line() {
    let text = "let a = 1; // x\n".repeat(10_000);
    let mut doc = Doc::new(&text);
    let mut h = Highlighter::new(tokenizer_for("a.js"));
    let mut out = Vec::new();
    h.tokens(&doc.buffer, 9_000, &mut out);
    assert_eq!(h.valid(), 9_001);
    doc.cursors = vec![quadra_code::Cursor::at(doc.buffer.line_start(8_990))];
    doc.insert_text("y", true);
    h.sync(&mut doc);
    assert_eq!(h.valid(), 8_990);
    h.tokens(&doc.buffer, 8_995, &mut out);
    assert_eq!(h.valid(), 8_996);
}
