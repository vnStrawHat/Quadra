//! A table of live data (specs/13, "Live data"): rows known by their keys keep what the
//! user chose when the data changes, and a cell that changed is tinted for a moment.

use std::rc::Rc;
use std::time::Duration;

use quadra_design::{CellKind, Column, dark, table};
use quadra_platform::{Key, Mods};
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::{NodeId, Role};

/// The name, the latency and the age in seconds of a service.
type Services = Signal<Vec<(String, i64, i64)>>;

/// Six services in columns 200, 100 and 80 px wide; the latency of the one at `i` is
/// `60 - 10 i`, so sorting by it turns the rows round.
fn page() -> (Harness, Services, Signal<Vec<usize>>) {
    page_with(None)
}

/// The same, the source saying in `changes` which rows it changed.
fn page_with(changes: Option<Signal<Vec<usize>>>) -> (Harness, Services, Signal<Vec<usize>>) {
    let service = |i: i64| (format!("svc-{i}"), 60 - 10 * i, 0);
    let (services, chosen): (Services, _) = (signal((0..6).map(service).collect()), signal(Vec::new()));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(move || {
        let columns = [
            Column::new("Name").width(200.0),
            Column::new("Latency").numeric().width(100.0),
            Column::new("Age").kind(CellKind::Age).width(80.0),
        ];
        let cell = move |row: usize, column: usize| {
            services.with(|all| match (all.get(row), column) {
                (Some(service), 0) => service.0.clone(),
                (Some(service), 1) => service.1.to_string(),
                (Some(service), _) => service.2.to_string(),
                (None, _) => String::new(),
            })
        };
        let key = move |row: usize| services.with(|all| all[row].0.clone());
        let rows = table(columns, move || services.with(Vec::len), cell).selected(chosen).keys(key);
        quadra_design::root().child(match changes {
            Some(changes) => rows.changes(changes),
            None => rows,
        })
    });
    settle(&mut h);
    (h, services, chosen)
}

/// Frames until nothing is left to do; a tint that fades is given the second it takes.
fn settle(h: &mut Harness) {
    h.frame();
    while h.frame_if_needed().is_some() {
        h.advance(Duration::from_secs(1));
    }
}

/// A click in the row at a place: the header is 28 px, a row 26.
fn click(h: &mut Harness, place: usize) {
    h.click(50.0, 28.0 + 26.0 * place as f32 + 13.0);
    settle(h);
}

/// The names of the selected services.
fn chosen_names(services: Services, chosen: Signal<Vec<usize>>) -> Vec<String> {
    services.with_untracked(|all| chosen.get_untracked().iter().map(|&row| all[row].0.clone()).collect())
}

#[test]
fn what_is_chosen_stays_with_its_rows_when_the_data_changes() {
    let (mut h, services, chosen) = page();
    click(&mut h, 1);
    assert_eq!(chosen_names(services, chosen), ["svc-1"]);
    // The first service goes and the others come the other way round: the row is found.
    services.update(|all| {
        all.remove(0);
        all.reverse();
    });
    settle(&mut h);
    assert_eq!(
        (chosen.get_untracked(), chosen_names(services, chosen)),
        (vec![4], vec!["svc-1".to_string()])
    );
    // The keys are still on it: the arrow goes to the row above it, which is svc-2.
    h.key(Key::Up, Mods::NONE);
    settle(&mut h);
    assert_eq!(chosen_names(services, chosen), ["svc-2"]);

    // Sorted by latency, svc-5 (10 ms) is first. It stays selected where the sort puts it
    // when its latency becomes the largest, and the arrow goes on from there.
    h.click(250.0, 14.0);
    settle(&mut h);
    click(&mut h, 0);
    assert_eq!(chosen_names(services, chosen), ["svc-5"]);
    services.update(|all| all.iter_mut().find(|service| service.0 == "svc-5").unwrap().1 = 900);
    settle(&mut h);
    assert_eq!(chosen_names(services, chosen), ["svc-5"]);
    h.key(Key::Up, Mods::NONE);
    settle(&mut h);
    assert_eq!(chosen_names(services, chosen), ["svc-1"], "svc-1, 50 ms, is the row before the last");

    // A selected row that goes is not selected any more, and no other is in its place.
    services.update(|all| all.retain(|service| service.0 != "svc-1"));
    settle(&mut h);
    assert!(chosen.get_untracked().is_empty());
}

/// The tints of the row at a place: the row's first child holds them in a box.
fn tints(h: &Harness, place: usize) -> Vec<NodeId> {
    let rows = h.query(|n| n.semantics.role == Role::Row);
    let held = h.tree.children(h.tree.children(rows[1 + place])[0])[0];
    h.tree.children(held).to_vec()
}

#[test]
fn a_cell_that_says_something_else_is_tinted_and_the_tint_fades() {
    let (mut h, services, _) = page();
    assert!((0..6).all(|place| tints(&h, place).is_empty()));
    // Another latency for svc-2, a minute more for all, and a service that is new.
    services.update(|all| {
        all[2].1 = 77;
        all.iter_mut().for_each(|service| service.2 += 60);
        all.push(("svc-9".to_string(), 5, 0));
    });
    h.frame();
    // One tint, over the cell of the latency and above the row's seam: an age changes all
    // the time, and what is new has not changed.
    let marks: Vec<usize> = (0..7).map(|place| tints(&h, place).len()).collect();
    assert_eq!(marks, [0, 0, 1, 0, 0, 0, 0]);
    let mark = tints(&h, 2)[0];
    let rect = h.tree.window_rect(mark);
    assert_eq!((rect.x, rect.width, rect.height, h.tree.pose(mark).opacity), (200.0, 100.0, 25.0, 1.0));

    // It fades by the clock: a frame writes its opacity, and lays out and paints nothing.
    h.advance(Duration::from_millis(400));
    let (stats, _) = h.frame();
    assert!((h.tree.pose(mark).opacity - 0.5).abs() < 0.01);
    assert_eq!((stats.nodes_laid_out, stats.nodes_painted), (0, 0));
    // A change meanwhile tints its own cell, and the first tint goes on from where it was.
    services.update(|all| all[0].0 = "svc-zero".to_string());
    services.update(|all| all[3].1 = 1);
    h.advance(Duration::from_millis(200));
    h.frame();
    h.frame();
    assert_eq!((tints(&h, 0).len(), tints(&h, 3).len()), (0, 1), "a row of another key is another row");
    assert!((h.tree.pose(tints(&h, 2)[0]).opacity - 0.25).abs() < 0.01);

    // When all have faded the boxes are gone and the frames stop.
    h.advance(Duration::from_millis(900));
    settle(&mut h);
    assert!((0..7).all(|place| tints(&h, place).is_empty()));
    assert!(!h.tree.animating() && h.frame_if_needed().is_none());
}

#[test]
fn a_source_that_says_which_rows_it_changed_has_only_those_read() {
    let changes = signal(Vec::new());
    let (mut h, services, _) = page_with(Some(changes));
    // Two latencies change and the source tells of one: the other is not read.
    services.update(|all| (all[2].1, all[4].1) = (77, 78));
    changes.set(vec![2]);
    h.frame();
    let marks: Vec<usize> = (0..6).map(|place| tints(&h, place).len()).collect();
    assert_eq!(marks, [0, 0, 1, 0, 0, 0]);
    // When rows come or go, every row is read.
    services.update(|all| {
        all[3].1 = 1;
        all.push(("svc-9".to_string(), 5, 0));
    });
    h.frame();
    let marks: Vec<usize> = (0..7).map(|place| tints(&h, place).len()).collect();
    assert_eq!(marks, [0, 0, 1, 1, 1, 0, 0], "svc-4, not read when it changed, is read with the others");
}
