//! Finding (behaviour/18, "Find"): cases, whole words, patterns, replacing as one undo.

use quadra_code::{Doc, Query, next_match};

fn q(text: &str) -> Query {
    Query { text: text.to_string(), ..Query::default() }
}

#[test]
fn a_query_matches_its_other_case_unless_told_and_whole_words_alone() {
    let doc = Doc::new("Việt viỆt việtnam a_việt\nVIỆT");
    let found = q("việt").matches(&doc).unwrap();
    assert_eq!(found.len(), 5);
    assert_eq!(&doc.text()[found[4].clone()], "VIỆT");
    assert_eq!(Query { case: true, ..q("việt") }.matches(&doc).unwrap().len(), 2);
    let words = Query { word: true, ..q("việt") }.matches(&doc).unwrap();
    assert_eq!(words.len(), 3);
    assert_eq!(q("").matches(&doc).unwrap(), vec![]);
    assert_eq!(q("a.b").matches(&Doc::new("axb a.b")).unwrap(), vec![4..7]);
}

#[test]
fn a_pattern_matches_or_says_what_is_wrong() {
    let doc = Doc::new("x1 y22 z");
    let digits = Query { regex: true, ..q(r"\d+") };
    assert_eq!(digits.matches(&doc).unwrap(), vec![1..2, 4..6]);
    assert!(Query { regex: true, ..q("(") }.matches(&doc).is_err());
}

#[test]
fn next_goes_round_and_replace_all_is_one_undo() {
    let m = vec![0..1, 5..6, 9..10];
    assert_eq!(next_match(&m, 3, false), Some(1));
    assert_eq!(next_match(&m, 10, false), Some(0));
    assert_eq!(next_match(&m, 5, true), Some(0));
    assert_eq!(next_match(&m, 0, true), Some(2));
    assert_eq!(next_match(&[], 0, false), None);
    let mut doc = Doc::new("a b a b a");
    let found = q("a").matches(&doc).unwrap();
    doc.replace_all(&found, "cc");
    assert_eq!(doc.text(), "cc b cc b cc");
    assert!(doc.undo());
    assert_eq!(doc.text(), "a b a b a");
    doc.replace(4..5, "x");
    assert_eq!(doc.text(), "a b x b a");
}
