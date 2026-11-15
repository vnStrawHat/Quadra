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

#[test]
fn matches_kept_through_changes_are_the_matches_found_again() {
    use quadra_code::Edit;
    let pieces = ["ab", "abab", " ", "\n", "Ab_", "x", "a", "b", "\n\n"];
    let mut doc = Doc::new(&"ab abab\nxab Ab\n".repeat(20));
    let mut seed = 5u64;
    let mut next = |n: usize| {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 33) as usize % n.max(1)
    };
    for (word, case) in [(false, false), (true, false), (false, true)] {
        let query = Query { word, case, ..q("ab") };
        let mut kept = query.matches(&doc).unwrap();
        let mut version = doc.version;
        for step in 0..300 {
            // An edit, or several at once (cursors), each removing a little and putting in
            // a piece; now and then one undone.
            let text = doc.buffer.text();
            let mut at: Vec<usize> = (0..1 + next(3)).map(|_| next(text.len() + 1)).collect();
            at.sort_unstable();
            at.dedup();
            let mut edits: Vec<Edit> = Vec::new();
            for a in at {
                let end = (a + next(3)).min(text.len());
                if edits.last().is_some_and(|e| e.range.end > a) {
                    continue;
                }
                edits.push(Edit { range: a..end, text: pieces[next(pieces.len())].into(), caret: None });
            }
            doc.apply(edits, false);
            if next(5) == 0 {
                doc.undo();
            }
            if next(2) == 0 {
                kept = query.rematch(&doc, version, kept).expect("a query of text is kept");
                version = doc.version;
                assert_eq!(kept, query.matches(&doc).unwrap(), "{word} {case} {step}");
            }
        }
    }
    // A pattern may match across lines: it is found again whole.
    assert!(Query { regex: true, ..q("a") }.rematch(&doc, 0, Vec::new()).is_none());
}
