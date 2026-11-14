//! The diff (specs/13, "Components"): the lines of two texts as one column and side by
//! side, the part of a line that changed, and the folds.

use std::rc::Rc;

use quadra_kit::{DiffMode, dark, diff, token};
use quadra_platform::{Key, Mods};
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::Role;

const OLD: &str = "fn main() {\n    let x = 1;\n    run(x);\n}\n";
const NEW: &str = "fn main() {\n    let x = 22;\n    run(x);\n    done();\n}\n";

fn page(old: &str) -> (Harness, Signal<String>, Signal<DiffMode>) {
    let (old, mode) = (signal(old.to_string()), signal(DiffMode::Inline));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_kit::keymap()));
    h.mount(move || quadra_kit::root().child(diff(old, NEW).mode(mode)));
    h.frame();
    (h, old, mode)
}

/// The lines that show, from the top and then from the left: where each starts, and its name.
fn lines(h: &Harness) -> Vec<(f32, f32, String)> {
    let tree = h.tree.semantic_tree();
    let lines = tree.nodes.iter().filter(|n| n.semantics.role == Role::Cell);
    let mut lines: Vec<_> = lines.map(|n| (n.bounds.y, n.bounds.x, n.semantics.name.clone())).collect();
    lines.sort_by(|a, b| a.partial_cmp(b).unwrap());
    lines
}

fn names(h: &Harness) -> Vec<String> {
    lines(h).into_iter().map(|(_, _, name)| name).collect()
}

/// The instances painted in a colour: where each starts and how wide it is.
fn filled(h: &Harness, color: quadra_prim::Color) -> Vec<(f32, f32)> {
    let layers = h.renderer.steps().iter().filter_map(|step| match step {
        quadra_render::Step::Layer { id, .. } => Some(h.tree.scene().layer(*id)),
        quadra_render::Step::Run { .. } => None,
    });
    let of = layers.flat_map(|layer| layer.instances().iter());
    of.filter(|i| i.color == color.to_array()).map(|i| (i.bounds[0], i.bounds[2])).collect()
}

#[test]
fn the_lines_are_one_column_or_the_old_text_beside_the_new() {
    let (mut h, old, mode) = page(OLD);
    // One column: the line that goes, then the one that comes, each on a row of 18 px.
    let inline = [
        "fn main() {",
        "Removed:     let x = 1;",
        "Added:     let x = 22;",
        "    run(x);",
        "Added:     done();",
        "}",
    ];
    assert_eq!(names(&h), inline);
    assert!(lines(&h).iter().enumerate().all(|(i, line)| (line.0, line.1) == (i as f32 * 18.0, 0.0)));
    // What changed in the line that was edited is filled: one letter of 7.2 px where it
    // was "1", two where it is "22", after two numbers, the mark and twelve letters, from
    // a whole pixel to a whole pixel.
    let ink = |role| dark().color(role).alpha(0.3);
    let at = (2.0 * (2.0 * 7.2 + 16.0) + 16.0 + 12.0 * 7.2_f32).round();
    let close = |found: Vec<(f32, f32)>, x: f32, width: f32| {
        found.len() == 1 && (found[0].0 - x).abs() < 0.01 && (found[0].1 - width).abs() < 0.01
    };
    assert!(close(filled(&h, ink(token::ERROR)), at, 7.0), "{:?}", filled(&h, ink(token::ERROR)));
    assert!(close(filled(&h, ink(token::OK)), at, 15.0), "{:?}", filled(&h, ink(token::OK)));

    // Side by side: a line of both in both halves, the edited line beside what it became,
    // and nothing beside the line that only came.
    mode.set(DiffMode::SideBySide);
    h.frame();
    let sides = lines(&h);
    let said: Vec<&str> = sides.iter().map(|line| line.2.as_str()).collect();
    assert_eq!(
        said[..4],
        ["fn main() {", "fn main() {", "Removed:     let x = 1;", "Added:     let x = 22;"]
    );
    assert_eq!(said[4..], ["    run(x);", "    run(x);", "Added:     done();", "}", "}"]);
    assert_eq!(((sides[2].0, sides[2].1), (sides[3].0, sides[3].1)), ((18.0, 0.0), (18.0, 401.0)));
    assert_eq!((sides[6].0, sides[6].1), (54.0, 401.0));

    // Another text: the lines are compared again.
    old.set(NEW.replace("run", "walk"));
    h.frame();
    assert_eq!(names(&h)[4..6], ["Removed:     walk(x);", "Added:     run(x);"]);
    mode.set(DiffMode::Inline);
    old.set(NEW.to_string());
    h.frame();
    assert_eq!(names(&h), ["fn main() {", "    let x = 22;", "    run(x);", "    done();", "}"]);
}

/// A text of `n` numbered lines.
fn numbered(n: usize) -> String {
    (0..n).map(|i| format!("line {i}\n")).collect()
}

#[test]
fn lines_far_from_a_change_fold_into_a_row_that_opens() {
    let old = numbered(40).replace("line 20\n", "line twenty\n");
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_kit::keymap()));
    let text = signal(old);
    h.mount(move || quadra_kit::root().child(diff(text, numbered(40)).context(2)));
    h.frame();
    // Two lines kept on each side of the change, and a row for the others.
    let around = ["line 18", "line 19", "Removed: line twenty", "Added: line 20", "line 21", "line 22"];
    assert_eq!(names(&h), around);
    assert!(h.has_text("18 unchanged lines") && h.has_text("17 unchanged lines"));
    // A click shows the lines of one fold; Enter on the other, those of the other.
    h.get_by_role(Role::Button, "18 unchanged lines").click();
    h.frame();
    assert_eq!((names(&h).len(), &names(&h)[0][..]), (24, "line 0"));
    h.get_by_role(Role::Button, "17 unchanged lines").focus();
    h.key(Key::Enter, Mods::default());
    h.frame();
    assert!(names(&h).contains(&"line 30".to_string()) && !h.has_text("17 unchanged lines"));
    // Other texts: the folds are closed.
    text.set(numbered(40).replace("line 5\n", ""));
    h.frame();
    assert_eq!(names(&h), ["line 3", "line 4", "Added: line 5", "line 6", "line 7"]);
    assert!(h.has_text("3 unchanged lines") && h.has_text("32 unchanged lines"));
}

#[test]
fn texts_of_a_hundred_thousand_lines_cost_what_the_rows_that_show_cost() {
    let (old, mode) = (numbered(100_000).replace("line 500\n", ""), signal(DiffMode::Inline));
    let new = numbered(100_000).replace("line 70000\n", "line seventy thousand\n");
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || quadra_kit::root().child(diff(old, new).mode(mode)));
    h.settle(); // compared on the pool: so many lines would hold a frame
    h.frame();
    // Some thirty-four rows of a few nodes each, of the 100 001 there are.
    let mounted = h.tree.len();
    assert!(mounted < 600 && names(&h)[0] == "line 0", "{mounted} nodes");
    // Laid out side by side: the rows that show are built again, and no more.
    mode.set(DiffMode::SideBySide);
    let (stats, _) = h.frame();
    assert!(stats.nodes_laid_out < 3000 && h.tree.len() < mounted * 2, "{stats:?}");
    assert_eq!(names(&h)[..2], ["line 0", "line 0"]);
}

#[test]
fn rows_are_selected_and_copied_and_f7_goes_to_the_next_change() {
    let (mut h, _, mode) = page(OLD);
    h.get_by_role(Role::Table, "Diff").focus();
    let copied = |h: &mut Harness| {
        h.key(Key::C, Mods::CTRL);
        h.clipboard.read_text().unwrap_or_default()
    };
    // F7 selects the first row of a run of changes; Shift and an arrow, the next one too.
    h.key(Key::F7, Mods::NONE);
    h.key(Key::Down, Mods::SHIFT);
    assert_eq!(copied(&mut h), "-    let x = 1;\n+    let x = 22;");
    h.key(Key::F7, Mods::NONE);
    assert_eq!(copied(&mut h), "+    done();");
    h.key(Key::F7, Mods::SHIFT);
    assert_eq!(copied(&mut h), "-    let x = 1;");
    // Side by side, a row is both halves; a click selects it.
    mode.set(DiffMode::SideBySide);
    h.frame();
    h.get_by_text("Removed:     let x = 1;").click();
    assert_eq!(copied(&mut h), "-    let x = 1;\n+    let x = 22;");
}

#[test]
fn the_wheel_with_shift_moves_what_the_lines_say_and_not_their_numbers() {
    let long = |c: char| format!("{}{c}{}\n", "x".repeat(30), "x".repeat(200));
    let (old, new) = (long('1'), long('2'));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || quadra_kit::root().child(diff(old, new)));
    h.frame();
    let ink = dark().color(token::ERROR).alpha(0.3);
    let (before, numbers) = (filled(&h, ink), lines(&h));
    let at = quadra_prim::Point::new(300.0, 9.0);
    h.tree.wheel(quadra_prim::Point::new(0.0, -1.0), false, at, Mods::SHIFT);
    h.frame();
    let after = filled(&h, ink);
    assert!(after.len() == 1 && after[0].0 < before[0].0, "{before:?} {after:?}");
    assert_eq!(lines(&h), numbers); // the rows stay where they were
}

#[test]
fn lines_take_the_colours_they_are_given() {
    // "fn" in the colour of a keyword wherever a line starts with it, in both texts.
    let colours = |text: &str| {
        let line =
            |line: &str| if line.starts_with("fn") { vec![(0..2, token::SYNTAX_KEYWORD)] } else { vec![] };
        text.split('\n').map(line).collect()
    };
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || quadra_kit::root().child(diff(OLD, NEW).colours(colours)));
    h.frame();
    // Two letters of the first line, after its numbers and its mark; and nothing else.
    let keyword = filled(&h, dark().color(token::SYNTAX_KEYWORD));
    let x = 2.0 * (2.0 * 7.2 + 16.0) + 16.0;
    assert_eq!(keyword.len(), 2, "{keyword:?}");
    assert!(keyword.iter().all(|(at, _)| *at >= x - 1.0 && *at < x + 15.0), "{keyword:?}");
}

#[test]
fn long_lines_wrap_and_their_rows_are_as_high() {
    let long = format!("short\n{}\nend\n", "word ".repeat(60));
    let changed = format!("short\n{}more\nend\n", "word ".repeat(60));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        let view = diff(long.clone(), changed.clone()).wrap();
        quadra_kit::root().child(quadra_ui::column().width(400.0).height(400.0).child(view))
    });
    h.frame();
    h.settle();
    // 300 letters of 7.2 px in a column of 400 less the numbers: several lines high.
    let rows = lines(&h);
    let (first, removed, added) = (rows[0].0, rows[1].0, rows[2].0);
    assert_eq!((first, removed), (0.0, 18.0));
    assert!(added - removed > 18.0 * 3.0, "{rows:?}");
    // Nothing scrolls sideways: the wheel turned so moves nothing.
    assert!(rows.iter().all(|row| row.1 == 0.0));
}
