//! Find in a document (specs/13, "Markdown", Find): matches in its paragraphs, its code and
//! its tables, the switches, found again after an edit, and the current one brought into view.

use std::rc::Rc;

use quadra_code::Query;
use quadra_design::dark;
use quadra_md::{Find, markdown};
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_view::{column, scroll};

const FRUIT: &str =
    "# Fruit\n\nAn apple a day, pineapples too.\n\n```\nlet apple = 1;\n```\n\n| Name |\n|---|\n| Apple |\n";

fn page(source: Signal<String>, find: Find, scrolled: Signal<f32>) -> Harness {
    let mut h = Harness::new();
    h.size = quadra_prim::Size::new(800.0, 300.0);
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        let doc = markdown(move || source.get()).find(find);
        let pane = column().grow(1.0).min_height(0.0);
        quadra_design::root().child(pane.child(scroll(scrolled, column().child(doc))))
    });
    h.frame();
    h
}

fn query(text: &str, case: bool, word: bool) -> Option<Query> {
    Some(Query { text: text.into(), case, word, regex: false })
}

#[test]
fn matches_are_counted_in_words_code_and_tables_with_the_switches() {
    let (source, scrolled) = (signal(FRUIT.to_string()), signal(0.0));
    let find = Find { query: signal(None), current: signal(0), count: signal(0) };
    let mut h = page(source, find, scrolled);
    find.query.set(query("apple", false, false));
    h.settle();
    assert_eq!(find.count.get_untracked(), 4, "the paragraph's two, the code's and the cell's");
    find.query.set(query("apple", true, false));
    h.settle();
    assert_eq!(find.count.get_untracked(), 3, "Apple is not apple");
    find.query.set(query("apple", false, true));
    h.settle();
    assert_eq!(find.count.get_untracked(), 3, "pineapples is no whole word");
    source.set(format!("{FRUIT}\nOne more apple.\n"));
    h.settle();
    assert_eq!(find.count.get_untracked(), 4, "found again after an edit");
    find.query.set(None);
    h.settle();
    assert_eq!(find.count.get_untracked(), 0);
}

#[test]
fn the_current_match_is_brought_into_view() {
    let filler: String = (0..40).map(|k| format!("Line {k} of words.\n\n")).collect();
    let (source, scrolled) = (signal(format!("{filler}The needle is here.\n")), signal(0.0));
    let find = Find { query: signal(None), current: signal(0), count: signal(0) };
    let mut h = page(source, find, scrolled);
    find.query.set(query("needle", false, false));
    h.settle();
    assert_eq!(find.count.get_untracked(), 1);
    assert!(scrolled.get_untracked() > 1000.0, "scrolled to the end: {}", scrolled.get_untracked());
}
