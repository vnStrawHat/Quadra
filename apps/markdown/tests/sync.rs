//! Side by side (specs/21, "Side by side"): the pane moved drives and the other follows by the
//! headings, with no echo; a heading pressed in the preview takes the editor there, and the
//! caret on a heading's line shows it in the preview.

use std::rc::Rc;
use std::time::Duration;

use markdown::{Driver, MdApp, Model, SIDE_BY_SIDE, keymap};
use quadra::Role;
use quadra_testing::Harness;

/// Sections of a heading and three paragraphs; section `k`'s heading is on line 8 × k.
fn sections(n: usize) -> String {
    (0..n).map(|k| format!("## Part {k}\n\nOne {k}.\n\nTwo {k}.\n\nThree {k}.\n\n")).collect()
}

fn app(name: &str) -> (Harness, Model) {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/markdown-tests");
    std::fs::create_dir_all(&path).unwrap();
    let path = path.join(format!("sync-{name}.md"));
    std::fs::write(&path, sections(30)).unwrap();
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
    m.layout.set(SIDE_BY_SIDE);
    h.settle();
    (h, m)
}

/// The top of the heading of `line` in the preview's scroll.
fn heading_y(m: &Model, line: usize) -> f32 {
    m.places.with_untracked(|p| p.iter().find(|it| it.line == line).map(|it| it.top + 24.0).unwrap())
}

#[test]
fn the_editor_drives_and_a_heading_at_its_top_is_at_the_previews() {
    let (mut h, m) = app("drives");
    m.pos.set(80.0); // Part 10 at the editor's top
    h.settle();
    assert_eq!(m.driver.get_untracked(), Driver::Editor);
    assert!((m.preview_y.get_untracked() - heading_y(&m, 80)).abs() < 1.0);
    // Half way to the next heading: half way in the preview too.
    m.pos.set(84.0);
    h.settle();
    let half = (heading_y(&m, 80) + heading_y(&m, 88)) / 2.0;
    assert!((m.preview_y.get_untracked() - half).abs() < 1.0);
}

#[test]
fn the_wheel_over_the_preview_makes_it_drive_with_no_echo() {
    let (mut h, m) = app("wheel");
    let pane = h.get_by_role(Role::Group, "Rendered").node();
    let r = h.tree.window_rect(pane);
    h.wheel(r.x + 100.0, r.y + 100.0, -20.0);
    h.settle();
    assert_eq!(m.driver.get_untracked(), Driver::Preview);
    let y = m.preview_y.get_untracked();
    assert!(y > 0.0);
    // The editor followed, and its moving did not move the preview back.
    assert!(m.pos.get_untracked() > 0.0);
    h.advance(Duration::from_millis(50));
    h.settle();
    assert_eq!(m.preview_y.get_untracked(), y);
    // Between two headings, the editor is as far between their lines.
    let lo = (0..30).map(|k| k * 8).rfind(|&l| heading_y(&m, l) <= y).unwrap();
    assert!(m.pos.get_untracked() >= lo as f64 && m.pos.get_untracked() < lo as f64 + 8.0);
}

#[test]
fn a_heading_pressed_in_the_preview_takes_the_editor_there() {
    let (mut h, m) = app("pressed");
    h.get_by_role(Role::Heading, "Part 2").click();
    h.settle();
    assert_eq!(m.caret.get_untracked(), 16, "the caret at its line");
    assert!((m.pos.get_untracked() - 16.0).abs() < 1e-3, "the line at the editor's top");
    assert!((m.preview_y.get_untracked() - heading_y(&m, 16)).abs() < 1.0, "and at the preview's");
    assert_eq!(m.flash.get_untracked(), Some(16), "tinted");
    h.advance(Duration::from_millis(700));
    h.settle();
    assert_eq!(m.flash.get_untracked(), None, "for a moment");
}

#[test]
fn the_caret_on_a_heading_shows_it_in_the_preview() {
    let (mut h, m) = app("caret");
    m.editor.edit(|w| {
        let at = w.doc.buffer.line_start(24);
        w.doc.set_cursor(at, at, false);
    });
    h.settle();
    assert_eq!((m.caret.get_untracked(), m.flash.get_untracked()), (24, Some(24)));
    assert_eq!(m.reveal.get_untracked(), Some(24));
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

#[test]
fn a_layout_changed_keeps_where_each_pane_is() {
    let (mut h, m) = app("layouts");
    // Raw, scrolled down: side by side, the editor is where it was.
    h.key(quadra::Key::Digit1, quadra::Mods::CTRL);
    frames(&mut h);
    m.pos.set(80.0);
    frames(&mut h);
    h.key(quadra::Key::Digit2, quadra::Mods::CTRL);
    frames(&mut h);
    assert!((m.pos.get_untracked() - 80.0).abs() < 1e-3, "{}", m.pos.get_untracked());
    let at = h.tree.window_rect(h.query(|n| n.semantics.role == Role::TextInput)[0]).y;
    assert!((at - (40.0 + 45.0 - 80.0 * 22.0)).abs() < 1.0, "the editor's rows moved: {at}");
    assert!((m.preview_y.get_untracked() - heading_y(&m, 80)).abs() < 1.0);
    // Preview and back: the preview is where it was.
    let y = m.preview_y.get_untracked();
    h.key(quadra::Key::Digit3, quadra::Mods::CTRL);
    frames(&mut h);
    h.key(quadra::Key::Digit2, quadra::Mods::CTRL);
    frames(&mut h);
    assert!((m.preview_y.get_untracked() - y).abs() < 1.0, "{} {y}", m.preview_y.get_untracked());
    assert!((m.pos.get_untracked() - 80.0).abs() < 1e-3);
}

#[test]
fn a_heading_shown_once_does_not_pull_the_preview_back() {
    let (mut h, m) = app("reveal");
    m.editor.edit(|w| {
        let at = w.doc.buffer.line_start(8);
        w.doc.set_cursor(at, at, false);
    });
    frames(&mut h);
    // Far away, then a letter typed there.
    m.pos.set(160.0);
    frames(&mut h);
    m.editor.edit(|w| {
        let at = w.doc.buffer.line_start(162);
        w.doc.set_cursor(at, at, false);
        w.doc.insert_text("x", true);
    });
    frames(&mut h);
    h.advance(Duration::from_millis(700));
    frames(&mut h);
    let want = heading_y(&m, 160);
    assert!((m.preview_y.get_untracked() - want).abs() < 40.0, "{} {want}", m.preview_y.get_untracked());
}
