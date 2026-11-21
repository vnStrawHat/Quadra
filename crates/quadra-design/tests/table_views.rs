//! A table's cells that show views (specs/13, "What a cell shows"): built once for each row
//! that is mounted, bound to the row and what the cell says, and a button in one takes its
//! press from the row.

use std::cell::Cell;
use std::rc::Rc;

use quadra_design::{CellKind, Column, button, dark, table};
use quadra_platform::{Key, Mods};
use quadra_prim::Point;
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::Role;
use quadra_view::{row, text};

#[test]
fn a_cell_shows_a_view_bound_to_its_row_and_a_button_in_it_does_not_select_the_row() {
    let (built, revealed, chosen) = (Rc::new(Cell::new(0)), signal(Vec::<usize>::new()), signal(Vec::new()));
    let names = signal((0..500).map(|i| format!("key-{i:03}")).collect::<Vec<_>>());
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    let count = built.clone();
    h.mount(move || {
        let value = Column::new("Value").view(move |at, says| {
            count.set(count.get() + 1);
            let reveal = button("Reveal")
                .on_click(move || revealed.update(|all| all.push(quadra_reactive::untrack(|| at.get()))));
            row().gap(8.0).child(text(move || format!("{} bytes", says.get().len()))).child(reveal)
        });
        let columns = [Column::new("Key").mono(), value, Column::new("Note").muted()];
        let cell = move |r: usize, c: usize| match c {
            0 | 1 => names.with(|all| all[r].clone()),
            _ => "set by the operator".to_string(),
        };
        quadra_design::root().child(table(columns, move || names.with(Vec::len), cell).selected(chosen))
    });
    h.frame();
    let first = built.get();
    assert!(first > 0 && h.has_text("7 bytes") && h.has_text("set by the operator"));

    // The button's press reveals its row's key, and does not select the row.
    let reveal = h.query(|n| n.semantics.role == Role::Button && n.semantics.name == "Reveal")[0];
    let at = h.tree.window_rect(reveal).center();
    h.click(at.x, at.y);
    h.frame();
    assert_eq!((revealed.get_untracked(), chosen.get_untracked()), (vec![0], vec![]));
    // A press elsewhere in the row selects it.
    let key = h.query(|n| n.semantics.name == "key-002")[0];
    let at = h.tree.window_rect(key).center();
    h.click(at.x, at.y);
    h.frame();
    assert_eq!(chosen.get_untracked(), vec![2]);

    // Scrolled, the rows are reused: no view is built again, and they show their new rows.
    h.tree.wheel(Point::new(0.0, -2000.0), false, Point::new(300.0, 300.0), Mods::NONE);
    h.frame();
    assert!(h.query(|n| n.semantics.name == "key-000").is_empty() && built.get() == first);
    // What a cell says changes: its view follows, and is not built again.
    names.update(|all| all.iter_mut().for_each(|name| name.push_str("-long")));
    h.frame();
    assert!(h.has_text("12 bytes") && built.get() == first, "{}", h.semantics());
}

#[test]
fn an_age_that_is_no_number_shows_nothing_and_sorts_after_every_age() {
    let jobs = [("job-a", "120"), ("job-b", ""), ("job-c", "30"), ("job-d", "-")];
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(move || {
        let columns = [Column::new("Job"), Column::new("Age").kind(CellKind::Age)];
        let cell = move |r: usize, c: usize| if c == 0 { jobs[r].0 } else { jobs[r].1 }.to_string();
        quadra_design::root().child(table(columns, || 4, cell))
    });
    h.frame();
    assert!(h.has_text("2m") && h.has_text("30s") && !h.has_text("0s"), "{}", h.semantics());
    // The jobs from the top, after a click on the title of the ages.
    let sorted = |h: &mut Harness| {
        h.get_by_role(Role::Cell, "Age").click();
        h.frame();
        let mut names = ["job-a", "job-b", "job-c", "job-d"];
        let y = |h: &Harness, name: &str| h.tree.window_rect(h.query(|n| n.semantics.name == name)[0]).y;
        names.sort_by(|a, b| y(h, a).total_cmp(&y(h, b)));
        names
    };
    assert_eq!(sorted(&mut h), ["job-c", "job-a", "job-b", "job-d"]);
    // Those with no age stay last, in the order turned as ties are.
    assert_eq!(sorted(&mut h), ["job-a", "job-c", "job-d", "job-b"]);
}

#[test]
fn a_cell_is_one_line_cut_where_its_column_ends() {
    let long = "registry.example.com/platform/api-gateway:2026.11.21-8e77d10";
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        let columns = [
            Column::new("Image").width(120.0),
            Column::new("Status").kind(CellKind::Status).width(90.0),
            Column::new("Next"),
        ];
        let cell = move |_: usize, c: usize| [long, "CrashLoopBackOff", "after"][c].to_string();
        quadra_design::root().child(table(columns, || 2, cell))
    });
    h.frame();
    // The letters painted: none under the two rows (a header of 28, rows of 26), so nothing
    // is painted over what comes after.
    let layers = h.renderer.steps().iter().filter_map(|step| match step {
        quadra_render::Step::Layer { id, world } => Some((h.tree.scene().layer(*id), world.transform.ty)),
        quadra_render::Step::Run { .. } => None,
    });
    let glyphs = layers.flat_map(|(layer, y)| layer.instances().iter().map(move |i| (i, y)));
    let glyphs = glyphs.filter(|(i, _)| i.flags & quadra_render::FLAG_GLYPH != 0);
    let lowest = glyphs.map(|(i, y)| i.bounds[1] + i.bounds[3] + y).fold(0.0f32, f32::max);
    assert!(lowest > 0.0 && lowest <= 28.0 + 2.0 * 26.0, "{lowest}");
}

#[test]
fn a_row_is_activated_by_a_double_click_or_enter_and_not_by_the_arrows_or_its_buttons() {
    let (opened, pressed) = (signal(Vec::<usize>::new()), signal(0));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(move || {
        let copy =
            Column::new("").view(move |_, _| button("Copy").on_click(move || pressed.update(|n| *n += 1)));
        let columns = [Column::new("Pod"), copy];
        let cell = |r: usize, c: usize| if c == 0 { format!("pod-{r}") } else { String::new() };
        let pods = table(columns, || 5, cell).on_activate(move |row| opened.update(|all| all.push(row)));
        quadra_design::root().child(pods)
    });
    h.frame();
    let center =
        |h: &Harness, name: &str| h.tree.window_rect(h.query(|n| n.semantics.name == name)[0]).center();

    // A double click on a row opens it; the arrows only move, and Enter opens where they are.
    let at = center(&h, "pod-2");
    h.click(at.x, at.y);
    h.click(at.x, at.y);
    h.frame();
    assert_eq!(opened.get_untracked(), [2]);
    h.key(Key::Down, Mods::NONE);
    h.key(Key::Down, Mods::NONE);
    h.frame();
    assert_eq!(opened.get_untracked(), [2]);
    h.key(Key::Enter, Mods::NONE);
    h.frame();
    assert_eq!(opened.get_untracked(), [2, 4]);

    // A double click on a button in a row, and Enter on it, are the button's.
    let copy = h.query(|n| n.semantics.role == Role::Button && n.semantics.name == "Copy")[1];
    let at = h.tree.window_rect(copy).center();
    h.click(at.x, at.y);
    h.click(at.x, at.y);
    h.key(Key::Enter, Mods::NONE);
    h.frame();
    assert_eq!((opened.get_untracked(), pressed.get_untracked()), (vec![2, 4], 3));
}

#[test]
fn a_dense_table_has_a_lower_header_in_small_muted_words_and_12_px_between_words() {
    use quadra_design::token;

    let chosen = signal(vec![0usize]);
    let mut h = Harness::new();
    let theme = dark().theme();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        let columns = [Column::new("Name").width(120.0), Column::new("Restarts").width(80.0).numeric()];
        let cell = |r: usize, c: usize| if c == 0 { format!("pod-{r}") } else { r.to_string() };
        quadra_design::root().child(table(columns, || 3, cell).dense().selected(chosen))
    });
    h.frame();
    // The header: 26 px, its titles 12 px in the second text colour, the first 12 px in.
    let title = h.get_by_text("Name").node();
    let head = h.query(|n| n.semantics.role == Role::Row)[0];
    assert_eq!(h.tree.window_rect(head).height, 26.0);
    assert_eq!(h.tree.inherited(title).style.size, 12.0);
    assert_eq!(h.tree.inherited(title).color, theme.resolve(token::TEXT_2.into()));
    let words = h.tree.children(title)[0];
    assert_eq!(h.tree.window_rect(words).x, 12.0);
    // A row: its first words 12 px in; the next column's start where the first column ends,
    // its words 12 px after.
    let cell = h.get_by_text("pod-1").node();
    let (words, next) = (h.tree.children(cell)[0], h.tree.children(h.tree.parent(cell).unwrap())[1]);
    let (cell, words, next) = (h.tree.window_rect(cell), h.tree.window_rect(words), h.tree.window_rect(next));
    assert_eq!((words.x, cell.width, next.x - cell.x), (12.0, 120.0, 120.0));
    // The selected row's bar runs along all of its left edge.
    let rows = h.query(|n| n.semantics.role == Role::Row);
    let first = h.tree.window_rect(rows[1]);
    let bar = *h.tree.children(rows[1]).last().unwrap();
    let bar = h.tree.window_rect(bar);
    assert_eq!((bar.x, bar.y, bar.width, bar.height), (first.x, first.y, 2.0, first.height - 1.0));
}
