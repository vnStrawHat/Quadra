//! Where the blocks of a document are (specs/13, "Markdown"): written after layout, for an
//! outline and for two panes that scroll together; a line brought into view; a document of
//! tens of thousands of lines that mounts the blocks that show.

use std::rc::Rc;

use quadra_design::dark;
use quadra_md::{Place, markdown};
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_view::{column, scroll};

/// Sections of a heading and a paragraph, `n` of them: each starts on line 4 × k.
fn sections(n: usize) -> String {
    (0..n).map(|k| format!("## Part {k}\n\nThe words of part {k}.\n\n")).collect()
}

fn harness() -> Harness {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h
}

#[test]
fn the_places_of_the_blocks_are_written_after_layout() {
    let (places, source) = (signal(Vec::<Place>::new()), signal(sections(3)));
    let mut h = harness();
    h.mount(move || quadra_design::root().child(markdown(source).places(places)));
    h.settle();
    let lines: Vec<usize> = places.get_untracked().iter().map(|place| place.line).collect();
    assert_eq!(lines, [0, 2, 4, 6, 8, 10]);
    let all = places.get_untracked();
    assert_eq!(all[0].top, 0.0);
    assert!(all.windows(2).all(|two| two[1].top >= two[0].top + two[0].height), "{all:?}");
    // A part put in at the top moves the others down, and their lines.
    source.set(format!("## New\n\nFirst.\n\n{}", sections(3)));
    h.settle();
    let after = places.get_untracked();
    assert_eq!((after.len(), after[2].line), (8, 4));
    assert!(after[2].top > all[0].top);
}

#[test]
fn a_line_is_brought_into_view_in_a_view_that_scrolls_it() {
    let (wanted, scroll_y) = (signal(None::<usize>), signal(0.0));
    let mut h = harness();
    h.mount(move || {
        let doc = markdown(sections(60)).reveal(move || wanted.get());
        let page = scroll(scroll_y, doc).container(column().height(300.0));
        quadra_design::root().child(page)
    });
    h.settle();
    wanted.set(Some(160)); // the heading of part 40
    h.settle();
    assert!(scroll_y.get_untracked() > 1000.0, "{}", scroll_y.get_untracked());
    assert!(h.has_text("Part 40"));
}

fn far(h: &mut Harness, scroll_y: Signal<f32>, wanted: Signal<Option<usize>>, places: Signal<Vec<Place>>) {
    // Ten thousand sections, forty thousand lines.
    let source = sections(10_000);
    h.mount(move || {
        let doc = markdown(source.clone()).scrolls(scroll_y).places(places).reveal(move || wanted.get());
        quadra_design::root().child(column().height(400.0).child(doc))
    });
    h.settle();
}

#[test]
fn a_document_that_scrolls_itself_mounts_the_blocks_that_show() {
    let (scroll_y, wanted, places) = (signal(0.0), signal(None), signal(Vec::new()));
    let mut h = harness();
    far(&mut h, scroll_y, wanted, places);
    // A few dozen blocks of twenty thousand are mounted, and only theirs are placed.
    assert!(h.tree.len() < 600, "{} nodes", h.tree.len());
    let shown = places.get_untracked();
    assert!(!shown.is_empty() && shown.len() < 40 && shown[0].line == 0, "{shown:?}");
    // The heading of part 7 000 brought into view: it is mounted, and placed where the
    // document is scrolled to.
    wanted.set(Some(28_000));
    h.settle();
    assert!(h.has_text("Part 7000"));
    let at = scroll_y.get_untracked();
    let placed = places.get_untracked();
    let part = placed.iter().find(|place| place.line == 28_000).expect("placed");
    assert!(part.top >= at && part.top < at + 400.0, "{part:?} at {at}");
}

#[test]
fn a_heading_pressed_says_its_source_line() {
    let told = signal(None::<usize>);
    let mut h = harness();
    h.mount(move || {
        quadra_design::root().child(markdown(sections(3)).on_heading(move |line| told.set(Some(line))))
    });
    h.settle();
    h.get_by_role(quadra_tree::Role::Heading, "Part 1").click();
    h.settle();
    assert_eq!(told.get_untracked(), Some(4));
    h.get_by_text("The words of part 2.").click();
    h.settle();
    assert_eq!(told.get_untracked(), Some(4), "a paragraph is no heading");
}

#[test]
fn a_document_gone_before_its_layout_writes_no_places() {
    let (places, source, shown) = (signal(Vec::<Place>::new()), signal(sections(2)), signal(true));
    let mut h = harness();
    h.mount(move || {
        let doc = move || markdown(source).places(places);
        quadra_design::root().child(quadra_view::show(move || shown.get(), doc))
    });
    h.settle();
    // A change that asks for its places, and the document removed in the same pass.
    quadra_reactive::batch(|| {
        source.set(sections(3));
        shown.set(false);
    });
    h.settle();
    assert_eq!(places.get_untracked().len(), 4, "the places it last had");
}
