//! The table (specs/behaviour/09, "Data"): rows mounted as they show, columns sorted by a
//! click on their title and made wider by dragging their edge, rows selected by the pointer
//! and the keys, and copied.

use std::rc::Rc;

use quadra_design::{CellKind, Column, age_text, dark, table};
use quadra_platform::{Key, Mods, MouseButton};
use quadra_prim::Point;
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::{PointerInput, Role};

struct Page {
    h: Harness,
    /// Name, CPU, status and age in seconds of each pod.
    pods: Signal<Vec<(String, f64, &'static str, i64)>>,
    chosen: Signal<Vec<usize>>,
}

/// A thousand pods: the CPU of pod `i` is `(i * 37) % 100`, so sorting by it moves rows.
fn page() -> Page {
    page_with(None)
}

/// The same, with a cursor on cells in `cells`.
fn page_with(cells: Option<Signal<usize>>) -> Page {
    let pod = |i: usize| {
        let status = ["Running", "Pending", "Failed"][i % 3];
        (format!("pod-{i:04}"), ((i * 37) % 100) as f64, status, 30 + 50 * i as i64)
    };
    let (pods, chosen) = (signal((0..1000).map(pod).collect::<Vec<_>>()), signal(Vec::new()));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(move || {
        let columns = [
            Column::new("Name").width(200.0),
            Column::new("CPU").numeric().width(80.0),
            Column::new("Status").kind(CellKind::Status),
            Column::new("Age").kind(CellKind::Age).width(80.0),
        ];
        let cell = move |row: usize, column: usize| {
            pods.with(|pods| match (&pods[row], column) {
                (pod, 0) => pod.0.clone(),
                (pod, 1) => pod.1.to_string(),
                (pod, 2) => pod.2.to_string(),
                (pod, _) => pod.3.to_string(),
            })
        };
        let sheet = table(columns, move || pods.with(Vec::len), cell).selected(chosen);
        quadra_design::root().child(match cells {
            Some(column) => sheet.cells(column),
            None => sheet,
        })
    });
    h.frame();
    Page { h, pods, chosen }
}

impl Page {
    fn settle(&mut self) {
        self.h.frame();
        while self.h.frame_if_needed().is_some() {}
    }

    /// The names of the pods in the rows that show, the highest first.
    fn names(&mut self) -> Vec<String> {
        let rows = self.h.query(|n| n.semantics.role == Role::Row);
        let mut shown: Vec<(f32, String)> = Vec::new();
        for row in rows.into_iter().skip(1) {
            let name = self.h.tree.children(self.h.tree.children(row)[0])[0];
            let text = self.h.tree.widget_mut::<quadra_tree::Text>(name).unwrap().text().to_string();
            shown.push((self.h.tree.window_rect(row).y, text));
        }
        shown.sort_by(|a, b| a.0.total_cmp(&b.0));
        shown.into_iter().map(|(_, name)| name).collect()
    }

    fn click(&mut self, x: f32, y: f32, mods: Mods) {
        for input in [PointerInput::Move, PointerInput::Down, PointerInput::Up] {
            self.h.tree.pointer(input, MouseButton::Left, Point::new(x, y), mods, self.h.now());
        }
        self.settle();
    }

    fn key(&mut self, key: Key, mods: Mods) {
        self.h.key(key, mods);
        self.settle();
    }
}

#[test]
fn rows_show_as_their_columns_say_and_are_sorted_by_a_click_on_a_title() {
    let mut p = page();
    let tree = p.h.semantics();
    for line in ["Table", "Cell \"Name\"", "Cell \"pod-0000\"", "Cell \"Running\"", "Cell \"30s\""] {
        assert!(tree.contains(line), "{line} is not in:\n{tree}");
    }
    // The header is 28 px and a row 26: of a thousand rows, those of the window are mounted.
    let rows = p.h.query(|n| n.semantics.role == Role::Row);
    let (head, first) = (p.h.tree.window_rect(rows[0]), p.h.tree.window_rect(rows[1]));
    assert_eq!((head.height, first.y, first.height, rows.len() < 40), (28.0, 28.0, 24.0, true));
    assert_eq!(p.names()[..3], ["pod-0000", "pod-0001", "pod-0002"]);
    assert_eq!(
        [age_text(45), age_text(720), age_text(10_800), age_text(518_400)],
        ["45s", "12m", "3h", "6d"]
    );

    // A click on a title sorts from the least; another, from the most; numbers as numbers.
    p.h.get_by_role(Role::Cell, "CPU").click();
    p.settle();
    assert_eq!(p.names()[..3], ["pod-0000", "pod-0100", "pod-0200"]); // 0 % CPU, in the order given
    p.h.get_by_role(Role::Cell, "CPU").click();
    p.settle();
    assert_eq!(p.names()[0], "pod-0927"); // the last of those at 99 %: the order turned round
    p.h.get_by_role(Role::Cell, "Name").click();
    p.settle();
    assert_eq!(p.names()[..2], ["pod-0000", "pod-0001"]);
    // What the cells say changes: they follow, and so does the order.
    p.pods.update(|pods| pods[5].0 = "a-first".to_string());
    p.settle();
    assert_eq!(p.names()[..2], ["a-first", "pod-0000"]);
}

#[test]
fn rows_are_selected_by_the_pointer_and_the_keys_and_copied() {
    let mut p = page();
    let y = |row: f32| 28.0 + 24.0 * row + 12.0;
    p.click(50.0, y(2.0), Mods::NONE);
    assert_eq!(p.chosen.get_untracked(), [2]);
    p.click(50.0, y(4.0), Mods::CTRL); // with Ctrl: one more, or one less
    p.click(50.0, y(2.0), Mods::CTRL);
    assert_eq!(p.chosen.get_untracked(), [4]);
    p.click(50.0, y(7.0), Mods::SHIFT); // with Shift: from the last one clicked to this one
    assert_eq!(p.chosen.get_untracked(), [2, 3, 4, 5, 6, 7]);
    assert!(p.h.semantics().contains("Row [selected]"));

    // The keys move the row they are on and select it; with Shift the selection grows.
    p.click(50.0, y(1.0), Mods::NONE);
    p.key(Key::Down, Mods::NONE);
    p.key(Key::Down, Mods::SHIFT);
    p.key(Key::Down, Mods::SHIFT);
    assert_eq!(p.chosen.get_untracked(), [2, 3, 4]);
    // Copied as text: a line a row, a tab between cells.
    p.key(Key::C, Mods::CTRL);
    let copied = p.h.clipboard.read_text().unwrap();
    assert_eq!(copied.lines().next(), Some("pod-0002\t74\tFailed\t130"));
    assert_eq!(copied.lines().count(), 3);

    p.key(Key::PageDown, Mods::NONE);
    assert_eq!(p.chosen.get_untracked(), [24]);
    p.key(Key::End, Mods::NONE);
    assert!(p.chosen.get_untracked() == [999] && p.names().contains(&"pod-0999".to_string()));
    p.key(Key::Home, Mods::NONE);
    p.key(Key::Up, Mods::NONE);
    assert!(p.chosen.get_untracked() == [0] && p.names()[0] == "pod-0000");
    // Sorted, the selection is of the rows of the data, not of the places they show at.
    p.h.get_by_role(Role::Cell, "CPU").click();
    p.h.get_by_role(Role::Cell, "CPU").click();
    p.settle();
    p.click(50.0, y(0.0), Mods::NONE);
    assert_eq!(p.chosen.get_untracked(), [927]);
}

#[test]
fn a_column_is_made_wider_by_dragging_its_edge() {
    let mut p = page();
    let name = p.h.get_by_role(Role::Cell, "Name").node();
    let cpu = |h: &mut Harness| {
        let cell = h.get_by_role(Role::Cell, "0").node();
        h.tree.window_rect(cell)
    };
    assert_eq!((p.h.tree.window_rect(name).width, cpu(&mut p.h).x), (195.0, 200.0));
    // The 5 px at the edge of the title: pressed, moved 60 px, let go.
    p.h.pointer(PointerInput::Move, 198.0, 14.0);
    p.h.pointer(PointerInput::Down, 198.0, 14.0);
    p.h.pointer(PointerInput::Move, 258.0, 300.0);
    p.settle();
    assert_eq!((p.h.tree.window_rect(name).width, cpu(&mut p.h).x), (255.0, 260.0));
    p.h.pointer(PointerInput::Move, -400.0, 300.0); // never under its least
    p.h.pointer(PointerInput::Up, -400.0, 300.0);
    p.h.pointer(PointerInput::Move, 500.0, 300.0);
    p.settle();
    assert_eq!(cpu(&mut p.h).x, 48.0);
    assert_eq!(p.names()[0], "pod-0000"); // the press on the edge did not sort
}

#[test]
fn a_pinned_column_stays_while_the_others_scroll_and_hidden_ones_do_not_show() {
    let hidden = signal(Vec::new());
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(move || {
        let columns = (0..8).map(|c| Column::new(format!("C{c}")).width(150.0));
        let table = table(columns, || 20, |row, column| format!("r{row}c{column}"));
        quadra_view::column().size(500.0, 400.0).child(table.pinned(1).hidden(hidden))
    });
    h.frame();
    let x = |h: &mut Harness, name: &str| {
        let node = h.get_by_text(name).node();
        h.tree.window_rect(node).x
    };
    let (first, second) = (x(&mut h, "r0c0"), x(&mut h, "r0c1"));
    let title = x(&mut h, "C1");
    h.tree.wheel(Point::new(0.0, -3.0), false, Point::new(300.0, 60.0), Mods::SHIFT);
    h.frame();
    assert_eq!(x(&mut h, "r0c0"), first); // pinned
    let moved = second - x(&mut h, "r0c1");
    assert!(moved > 0.0 && title - x(&mut h, "C1") == moved, "the header moves with the rows");
    // Scrolled to the end, the last column ends at the table's edge.
    h.tree.wheel(Point::new(0.0, -100.0), false, Point::new(300.0, 60.0), Mods::SHIFT);
    h.frame();
    let last = h.get_by_text("r0c7").node();
    let last = h.tree.window_rect(last);
    assert!((last.right() - 500.0).abs() < 1.0, "{last:?}");

    // A hidden column does not show, and copying leaves it out.
    hidden.set(vec![1, 2, 3, 4, 5, 6]);
    h.frame();
    assert!(!h.has_text("r0c3") && h.has_text("r0c7"));
    h.get_by_text("r2c0").click();
    h.key(Key::C, Mods::CTRL);
    assert_eq!(h.clipboard.read_text().as_deref(), Some("r2c0\tr2c7"));
}

#[test]
fn a_cursor_on_cells_is_moved_by_the_arrows_and_the_pointer_and_copies_its_cell() {
    let column = signal(0);
    let mut p = page_with(Some(column));
    let y = |row: f32| 28.0 + 24.0 * row + 12.0;
    // A click puts it on a cell; to a reader that cell is selected, and its row is.
    p.click(250.0, y(2.0), Mods::NONE);
    assert_eq!((column.get_untracked(), p.chosen.get_untracked()), (1, vec![2]));
    assert!(p.h.semantics().contains("Cell \"74\" [selected]"), "{}", p.h.semantics());
    // The arrows move it over the columns and stop at the ends.
    p.key(Key::Right, Mods::NONE);
    p.key(Key::Right, Mods::NONE);
    p.key(Key::Right, Mods::NONE);
    assert_eq!(column.get_untracked(), 3);
    p.key(Key::Down, Mods::NONE);
    p.key(Key::Left, Mods::NONE);
    p.key(Key::C, Mods::CTRL);
    assert_eq!((p.h.clipboard.read_text().unwrap(), column.get_untracked()), ("Running".to_string(), 2));
    for _ in 0..4 {
        p.key(Key::Left, Mods::NONE);
    }
    assert_eq!((column.get_untracked(), p.chosen.get_untracked()), (0, vec![3]));
}

#[test]
fn the_menu_of_the_header_hides_and_shows_columns() {
    let mut p = page();
    for input in [PointerInput::Move, PointerInput::Down, PointerInput::Up] {
        p.h.tree.pointer(input, MouseButton::Right, Point::new(50.0, 14.0), Mods::NONE, p.h.now());
    }
    p.settle();
    let menu = p.h.semantics();
    assert!(menu.contains("MenuItem \"CPU\" [checked]"), "{menu}");
    p.h.get_by_role(Role::MenuItem, "CPU").click();
    p.settle();
    // It is gone from the header and the rows, and the menu checks it no more.
    let tree = p.h.semantics();
    assert!(!tree.contains("Cell \"CPU\"") && !tree.contains("Cell \"74\""), "{tree}");
    for input in [PointerInput::Move, PointerInput::Down, PointerInput::Up] {
        p.h.tree.pointer(input, MouseButton::Right, Point::new(50.0, 14.0), Mods::NONE, p.h.now());
    }
    p.settle();
    assert!(p.h.semantics().contains("MenuItem \"CPU\" [unchecked]"), "{}", p.h.semantics());
}
