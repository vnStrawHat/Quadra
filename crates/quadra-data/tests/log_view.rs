//! The log view (specs/13, "Data"): the lines that show, the newest kept in view until the
//! user scrolls away, and the filter of the bar.

use std::rc::Rc;

use quadra_data::{Log, log_view};
use quadra_kit::dark;
use quadra_testing::Harness;
use quadra_tree::Role;

/// Lines of requests, every tenth an error in red.
fn requests(from: usize, n: usize) -> Vec<(i64, String)> {
    let line = |i: usize| match i % 10 {
        3 => format!("\u{1b}[31mERROR\u{1b}[0m request {i} failed"),
        _ => format!("INFO request {i} ok"),
    };
    (from..from + n).map(|i| (1_793_966_400_000 + i as i64 * 250, line(i))).collect()
}

fn page(lines: usize) -> (Harness, Log) {
    let log = Log::new();
    log.append(requests(0, lines));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_kit::keymap()));
    let shown = log.clone();
    h.mount(move || quadra_kit::root().child(log_view(&shown)));
    settle(&mut h);
    (h, log)
}

fn settle(h: &mut Harness) {
    h.frame();
    while h.frame_if_needed().is_some() {}
}

/// Whether the bar's button of this name is on.
fn is_on(h: &mut Harness, name: &str) -> bool {
    let button = h.get_by_role(Role::Button, name).node();
    h.tree
        .semantic_tree()
        .nodes
        .iter()
        .any(|n| h.tree.node_of(n.id) == Some(button) && n.semantics.checked == Some(true))
}

#[test]
fn the_newest_line_stays_in_view_until_the_user_scrolls_away() {
    let (mut h, log) = page(100);
    // The window is 600 px: the bar, then some thirty rows of 18 px, the last ones.
    assert!(
        h.has_text("INFO request 99 ok") && h.has_text("12:00:24.750") && !h.has_text("INFO request 0 ok")
    );
    assert!(h.has_text("100 lines") && is_on(&mut h, "Follow"));
    // An error says what it says, without its escape sequences; no row is taller for a line
    // of a thousand letters.
    assert!(h.has_text("ERROR request 93 failed"));
    log.push(0, &"long ".repeat(200));
    log.append(requests(101, 49));
    settle(&mut h);
    assert!(
        h.has_text("INFO request 149 ok") && h.has_text("150 lines") && !h.has_text("INFO request 99 ok")
    );
    let rows = h.query(|n| n.semantics.role == Role::Text && n.semantics.name.starts_with("INFO request"));
    assert!(rows.iter().all(|&row| h.tree.window_rect(row).height == 18.0));

    // The wheel towards the start: it does not follow any more, and lines that come move nothing.
    h.wheel(400.0, 300.0, 3.0);
    settle(&mut h);
    assert!(!is_on(&mut h, "Follow") && !h.has_text("INFO request 149 ok"));
    let before = h.semantics();
    log.append(requests(150, 50));
    settle(&mut h);
    assert_eq!(h.semantics().replace("200 lines", "150 lines"), before);
    // Back at the end, it follows again.
    h.wheel(400.0, 300.0, -1000.0);
    settle(&mut h);
    assert!(is_on(&mut h, "Follow") && h.has_text("INFO request 199 ok"));
    log.append(requests(200, 10));
    settle(&mut h);
    assert!(h.has_text("INFO request 209 ok"));
    // The button stops it where it is, and takes it up again.
    h.get_by_role(Role::Button, "Follow").click();
    log.append(requests(210, 40));
    settle(&mut h);
    assert!(
        !is_on(&mut h, "Follow") && h.has_text("INFO request 209 ok") && !h.has_text("INFO request 249 ok")
    );
    h.get_by_role(Role::Button, "Follow").click();
    settle(&mut h);
    assert!(is_on(&mut h, "Follow") && h.has_text("INFO request 249 ok"));
}

#[test]
fn the_filter_lists_or_dims_by_a_text_or_a_pattern() {
    let (mut h, log) = page(100);
    h.get_by_role(Role::TextInput, "Filter").set_value("ERROR");
    settle(&mut h);
    // Ten of a hundred, all in view; lines that come are tested as they come.
    assert!(
        h.has_text("10 of 100") && h.has_text("ERROR request 3 failed") && !h.has_text("INFO request 99 ok")
    );
    log.append(requests(100, 10));
    settle(&mut h);
    assert!(h.has_text("11 of 110") && h.has_text("ERROR request 103 failed"));
    // Dimmed, every line is listed.
    h.get_by_role(Role::Button, "Dim").click();
    settle(&mut h);
    assert!(
        h.has_text("110 lines")
            && h.has_text("INFO request 109 ok")
            && h.has_text("ERROR request 103 failed")
    );
    h.get_by_role(Role::Button, "Dim").click();
    // A pattern, and one that is not one.
    h.get_by_role(Role::Button, ".*").click();
    h.get_by_role(Role::TextInput, "Filter").set_value(r"request \d+5 ");
    settle(&mut h);
    assert!(
        h.has_text("10 of 110")
            && h.has_text("INFO request 105 ok")
            && !h.has_text("ERROR request 103 failed")
    );
    h.get_by_role(Role::TextInput, "Filter").set_value("(");
    settle(&mut h);
    assert!(h.has_text("Bad pattern") && h.has_text("INFO request 109 ok"));
    // Cleared: nothing, and the lines after it.
    log.clear();
    log.push(0, "after");
    h.get_by_role(Role::TextInput, "Filter").set_value("");
    settle(&mut h);
    assert!(h.has_text("1 lines") && h.has_text("after") && !h.has_text("INFO request 109 ok"));
}

#[test]
fn a_million_lines_cost_what_the_rows_that_show_cost() {
    let (mut h, log) = page(1_000_000);
    assert!(h.has_text("INFO request 999999 ok") && h.has_text("1000000 lines"));
    let mounted = h.tree.len();
    // Lines that come while it follows: the rows that show are built again, and no more
    // nodes are held than before.
    log.append(requests(1_000_000, 160));
    let (stats, _) = h.frame();
    assert!(h.has_text("INFO request 1000159 ok") && h.tree.len() == mounted);
    assert!(stats.nodes_laid_out < 2000 && stats.nodes_painted < 300, "{stats:?}");
    // A filter reads every line once.
    log.set_filter("request 77777", false, false);
    assert_eq!(log.listed(), 11);
}

#[test]
fn lines_are_selected_and_copied_and_a_long_one_scrolls_sideways() {
    use quadra_platform::{Key, Mods};
    let (mut h, log) = page(100);
    // A click selects a line, Shift and an arrow the next too; Ctrl-C copies them with
    // their times and without their escape sequences.
    h.get_by_text("ERROR request 93 failed").click();
    h.key(Key::Down, Mods::SHIFT);
    h.key(Key::C, Mods::CTRL);
    let copied = h.clipboard.read_text().unwrap_or_default();
    assert_eq!(copied, "12:00:23.250 ERROR request 93 failed\n12:00:23.500 INFO request 94 ok");
    // Other lines listed: none is selected.
    h.get_by_role(Role::TextInput, "Filter").set_value("request 9");
    settle(&mut h);
    h.key(Key::C, Mods::CTRL);
    assert_eq!(h.clipboard.read_text().unwrap_or_default(), copied); // nothing copied over it

    // The wheel with Shift moves what a long line says; its time stays.
    h.get_by_role(Role::TextInput, "Filter").set_value("");
    log.push(1_793_966_500_000, &format!("INFO {}", "wide ".repeat(300)));
    settle(&mut h);
    let line = |h: &Harness| {
        let found = h.query(|n| n.semantics.role == Role::Text && n.semantics.name.starts_with("INFO wide"));
        h.tree.window_rect(found[0]).x
    };
    let time = |h: &Harness| h.tree.window_rect(h.query(|n| n.semantics.name == "12:01:40.000")[0]).x;
    let (x, at) = (line(&h), time(&h));
    let y = h.tree.window_rect(h.query(|n| n.semantics.name == "12:01:40.000")[0]).y + 9.0;
    h.tree.wheel(quadra_prim::Point::new(0.0, -1.0), false, quadra_prim::Point::new(300.0, y), Mods::SHIFT);
    settle(&mut h);
    assert!(line(&h) < x && time(&h) == at, "{x} -> {}", line(&h));
}
