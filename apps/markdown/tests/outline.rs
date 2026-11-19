//! The outline (specs/21, "Outline"): the headings nested and cut to the sidebar, the row of
//! the section at the top selected as the panes move, a row chosen taking the panes there.

use std::rc::Rc;

use markdown::{MdApp, Model, PREVIEW, RAW, keymap};
use quadra::{Key, Mods, Role};
use quadra_testing::Harness;

const LONG: &str = "A heading far too long to fit in the sidebar of the outline, so it ends with an ellipsis";

/// A title, then sections of a heading and three paragraphs; section `k`'s heading is on line
/// 2 + 8 × k, and the second section holds one of a deeper level.
fn document() -> String {
    let mut text = String::from("# Title\n\n");
    for k in 0..30 {
        let name = if k == 1 { LONG.to_string() } else { format!("Part {k}") };
        text += &format!("## {name}\n\nOne {k}.\n\nTwo {k}.\n\nThree {k}.\n\n");
    }
    text.replace("## Part 2\n", "### Part 2\n")
}

fn app(name: &str) -> (Harness, Model) {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/markdown-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("outline-{name}.md"));
    std::fs::write(&path, document()).unwrap();
    let mut h = Harness::new();
    h.size = quadra::Size::new(1200.0, 700.0);
    h.tree.set_keymap(Rc::new(keymap()));
    let built = Rc::new(std::cell::RefCell::new(None));
    let tell = built.clone();
    h.mount(move || {
        let md = MdApp::new(Some(path));
        *tell.borrow_mut() = Some(md.model.clone());
        md.view()
    });
    let m: Model = built.borrow_mut().take().unwrap();
    frames(&mut h);
    (h, m)
}

/// Runs frames until nothing is left to do, 50 at most: a loop that never stands still fails.
fn frames(h: &mut Harness) {
    h.frame();
    for n in 0.. {
        if h.frame_if_needed().is_none() {
            return;
        }
        assert!(n < 50, "the frames never stand still");
    }
}

fn rows(h: &Harness) -> Vec<String> {
    let items = h.query(|n| n.semantics.role == Role::TreeItem);
    items.into_iter().map(|id| h.tree.meaning(id).name).collect()
}

fn selected(h: &Harness) -> Vec<String> {
    let items = h.query(|n| n.semantics.role == Role::TreeItem && n.semantics.selected);
    items.into_iter().map(|id| h.tree.meaning(id).name).collect()
}

#[test]
fn the_headings_nested_and_a_long_one_cut_to_the_sidebar() {
    let (mut h, _m) = app("nested");
    let names = rows(&h);
    // The rows that show are there: those the sidebar's height holds.
    assert!(names.len() > 20, "{names:?}");
    assert_eq!(names[..4], ["Title", "Part 0", LONG, "Part 2"]);
    let long = h.get_by_role(Role::TreeItem, LONG).node();
    let deep = h.get_by_role(Role::TreeItem, "Part 2").node();
    let (long, deep) = (h.tree.window_rect(long), h.tree.window_rect(deep));
    assert!(long.x + long.width <= 240.0 + 0.5, "the row stays in the sidebar: {long:?}");
    let label = |id| h.tree.window_rect(id);
    // Its label, not the preview's heading.
    let inside: Vec<_> = h.query(|n| n.semantics.role == Role::Text && n.semantics.name == LONG);
    let inside: Vec<_> = inside.into_iter().filter(|&id| label(id).x < 240.0).collect();
    assert_eq!(inside.len(), 1);
    assert!(inside.iter().all(|&id| label(id).x + label(id).width <= 240.0 + 0.5), "cut where the row ends");
    assert!(deep.x == long.x, "the rows are as wide as the tree");
    // Part 2 is inside the long one: closing it hides it.
    h.get_by_role(Role::TreeItem, LONG).click();
    h.key(Key::Left, Mods::NONE);
    frames(&mut h);
    assert!(!rows(&h).iter().any(|name| name == "Part 2"));
    // The outline hides and shows.
    h.key(Key::O, Mods::CTRL | Mods::SHIFT);
    frames(&mut h);
    assert!(h.query(|n| n.semantics.role == Role::Tree).is_empty());
    h.key(Key::O, Mods::CTRL | Mods::SHIFT);
    frames(&mut h);
    assert_eq!(h.query(|n| n.semantics.role == Role::Tree).len(), 1);
}

#[test]
fn the_section_at_the_top_is_selected_and_a_row_chosen_goes_there() {
    let (mut h, m) = app("follows");
    // Preview: a row chosen puts its heading at the preview's top.
    assert_eq!(m.layout.get_untracked(), PREVIEW);
    h.get_by_role(Role::TreeItem, "Part 20").click();
    frames(&mut h);
    let line = 2 + 8 * 20;
    let top =
        m.places.with_untracked(|p| p.iter().find(|it| it.line == line).map(|it| it.top + 24.0).unwrap());
    assert!((m.preview_y.get_untracked() - top).abs() < 1.0);
    assert_eq!(selected(&h), ["Part 20"]);
    // Raw: the editor's top follows, and a row chosen takes its line there.
    h.key(Key::Digit1, Mods::CTRL);
    frames(&mut h);
    assert_eq!(m.layout.get_untracked(), RAW);
    // The user moves the editor (the wheel over it), and it stands in section 7.
    h.wheel(700.0, 400.0, 3.0);
    m.pos.set((2 + 8 * 7 + 3) as f64);
    frames(&mut h);
    assert_eq!(selected(&h), ["Part 7"], "the section at the top");
    h.get_by_role(Role::TreeItem, "Part 12").click();
    frames(&mut h);
    assert!((m.pos.get_untracked() - (2 + 8 * 12) as f64).abs() < 1e-3, "{}", m.pos.get_untracked());
    assert_eq!(m.caret.get_untracked(), 2 + 8 * 12, "the caret at the heading");
    assert_eq!(selected(&h), ["Part 12"]);
}

#[test]
fn the_last_heading_chosen_stays_chosen_where_the_panes_end() {
    let (mut h, m) = app("last");
    for layout in [Key::Digit1, Key::Digit3] {
        h.key(layout, Mods::CTRL);
        frames(&mut h);
        h.get_by_role(Role::TreeItem, "Part 20").click();
        frames(&mut h);
        // The last heading, in the last screenful: the panes stop at their ends below it.
        h.key(Key::End, Mods::NONE);
        frames(&mut h);
        assert_eq!(selected(&h), ["Part 29"], "{:?}", m.layout.get_untracked());
    }
    // A pane moved by the user: the row follows it again.
    h.wheel(700.0, 400.0, -3.0);
    m.preview_y.set(0.0);
    frames(&mut h);
    assert_eq!(selected(&h), ["Title"]);
}
