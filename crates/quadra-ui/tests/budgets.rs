//! The structural budgets of specs/14: what a frame visits for each kind of change, in a
//! window that also holds 10,000 unrelated nodes. These are counts, the same on any machine.
//! The rows about input, style variants and themes join as those are built; the virtual list's
//! rows are in tests/list.rs.

use quadra_prim::Color;
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::NodeId;
use quadra_ui::{column, div, mount_root, row, scroll, text};

struct Page {
    h: Harness,
    label: Signal<String>,
    note: Signal<String>,
    tone: Signal<Color>,
    row_tone: Signal<Color>,
    scroll_y: Signal<f32>,
    /// The layer of the scrolling list.
    list: NodeId,
}

/// On the left a scrolling list of 10,000 rows, one of which follows a signal. On the right a
/// card of a fixed size with a label, then a line whose note is followed by a tag, then a
/// footer.
fn page() -> Page {
    let label = signal("hello".to_string());
    let note = signal("a note".to_string());
    let tone = signal(Color::rgb(0x224466));
    let row_tone = signal(Color::rgb(0x101010));
    let scroll_y = signal(0.0);
    let item = move |i: u32| {
        let cell = div().size(100.0, 2.0);
        if i == 70 { cell.bg(row_tone) } else { cell.bg(Color::rgb(0x202020 + i)) }
    };
    let list = scroll(scroll_y, column().children((0..10_000).map(item))).container(div().size(200.0, 600.0));
    let card = div().size(200.0, 60.0).bg(tone).child(text(label).size(10.0));
    let line = row().child(text(note).size(10.0)).child(text("tag").size(10.0));
    let footer = div().size(300.0, 20.0).bg(Color::rgb(0x333333));
    let side = column().width(300.0).child(card).child(line).child(footer);
    let mut h = Harness::new();
    let root = mount_root(&mut h.tree, row().child(list).child(side));
    h.frame();
    let viewport = h.tree.children(root)[0];
    let list = h.tree.children(viewport)[0];
    assert!(h.tree.len() > 10_000 && h.tree.is_layer(list));
    Page { h, label, note, tone, row_tone, scroll_y, list }
}

#[test]
fn nothing_changed_is_no_frame() {
    let mut p = page();
    assert!(p.h.frame_if_needed().is_none());
    p.tone.set(p.tone.get()); // an equal value changes nothing
    p.label.set("hello".to_string());
    assert!(p.h.frame_if_needed().is_none());
}

#[test]
fn a_text_that_changes_and_keeps_its_size() {
    let mut p = page();
    p.label.set("world".to_string());
    let (frame, drawn) = p.h.frame();
    assert_eq!((frame.bindings_run, frame.writes, frame.nodes_painted), (1, 1, 1));
    assert!(frame.nodes_laid_out <= 3, "{frame:?}"); // the text; its card answers as before
    assert_eq!((frame.layers_patched, frame.layers_rebuilt), (1, 0));
    assert_eq!((drawn.instances_uploaded, drawn.layers_uploaded), (5, 1)); // its five glyphs
}

#[test]
fn a_text_that_changes_its_size_moves_what_follows_it() {
    let mut p = page();
    p.note.set("a longer note".to_string());
    let (frame, drawn) = p.h.frame();
    assert_eq!((frame.bindings_run, frame.nodes_painted), (1, 2)); // the note, and the tag after it
    assert!(frame.nodes_laid_out <= 8, "{frame:?}"); // up to the column of a fixed width
    assert_eq!((frame.layers_rebuilt, drawn.layers_uploaded), (1, 1)); // more glyphs: one layer
}

#[test]
fn one_row_among_ten_thousand() {
    let mut p = page();
    p.row_tone.set(Color::rgb(0xFF0000));
    let (frame, drawn) = p.h.frame();
    assert_eq!((frame.bindings_run, frame.nodes_painted, frame.nodes_laid_out), (1, 1, 0));
    assert_eq!((drawn.instances_uploaded, drawn.layers_uploaded), (1, 1));
}

#[test]
fn scrolling_plain_content() {
    let mut p = page();
    // The first scroll after a rest shows the bar: its thumb is painted, one instance.
    p.scroll_y.set(100.0);
    let (frame, drawn) = p.h.frame();
    assert_eq!((frame.nodes_painted, frame.nodes_laid_out, drawn.instances_uploaded), (1, 0, 1));
    // From then on a scroll is two constants: where the content is, and where the thumb.
    p.scroll_y.set(500.0);
    let (frame, drawn) = p.h.frame();
    assert_eq!((frame.nodes_painted, frame.nodes_laid_out, frame.layers_composed), (0, 0, 2));
    assert_eq!((frame.writes, drawn.instances_uploaded, drawn.layers_uploaded), (2, 0, 0));
    assert_eq!(p.h.tree.window_rect(p.list).y, -500.0);
    // Past what was painted around the window: the 900 rows around the new place (and the
    // column they are in), once.
    p.scroll_y.set(3_000.0);
    let (frame, drawn) = p.h.frame();
    assert_eq!((frame.nodes_painted, frame.nodes_laid_out, drawn.layers_uploaded), (901, 0, 1));
    p.scroll_y.set(3_500.0);
    assert_eq!(p.h.frame().0.nodes_painted, 0);
    p.scroll_y.set(1.0e9); // past the end: kept at the end
    p.h.frame();
    assert_eq!(p.h.tree.window_rect(p.list).y, 600.0 - 20_000.0);
}

#[test]
fn a_fade_or_a_slide_of_a_layer() {
    let mut p = page();
    for step in 1..=10 {
        p.h.tree.set_layer_opacity(p.list, 1.0 - step as f32 * 0.05);
        let (frame, drawn) = p.h.frame();
        assert_eq!((frame.nodes_painted, frame.layers_composed, drawn.instances_uploaded), (0, 1, 0));
    }
}

#[test]
fn a_colour_that_changes_every_frame() {
    let mut p = page();
    for step in 1..=10 {
        p.tone.set(Color::rgb(0x224466 + step * 0x010101));
        let (frame, drawn) = p.h.frame();
        assert_eq!((frame.bindings_run, frame.nodes_painted, frame.nodes_laid_out), (1, 1, 0));
        assert_eq!(drawn.instances_uploaded, 1);
    }
}
