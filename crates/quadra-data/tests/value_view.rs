//! The value viewer (specs/13, "Data"): a value as rows that open and close, the path of
//! the selected one in a bar, and copied.

use std::rc::Rc;

use quadra_data::{Value, parse_json, value_view};
use quadra_design::dark;
use quadra_platform::{Key, Mods};
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::Role;

const SAMPLE: &str = r#"{"name": "web", "replicas": 3, "ready": true, "selector": null,
 "labels": {"app": "web", "tier.kind": "frontend"},
 "containers": [{"name": "web", "image": "registry.local/web:1.4.2", "ports": [8080, 9090],
   "resources": {"limits": {"cpu": "500m"}}}]}"#;

fn page() -> (Harness, Signal<Rc<Value>>) {
    let value = signal(Rc::new(parse_json(SAMPLE).unwrap()));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(move || quadra_design::root().child(value_view(value)));
    h.frame();
    (h, value)
}

/// The labels of the rows, from the top.
fn labels(h: &Harness) -> Vec<String> {
    let rows = h.query(|n| n.semantics.role == Role::TreeItem);
    let mut all: Vec<(f32, String)> = Vec::new();
    for row in rows {
        let name = h.tree.semantic_tree().nodes.into_iter().find(|n| h.tree.node_of(n.id) == Some(row));
        all.push((h.tree.window_rect(row).y, name.unwrap().semantics.name));
    }
    all.sort_by(|a, b| a.0.total_cmp(&b.0));
    all.into_iter().map(|(_, label)| label).collect()
}

/// A click on the row with this label that is the `nth` from the top.
fn click(h: &mut Harness, label: &str, nth: usize) {
    let rows = h.query(|n| n.semantics.role == Role::TreeItem && n.semantics.name == label);
    let mut tops: Vec<f32> = rows.iter().map(|&row| h.tree.window_rect(row).y).collect();
    tops.sort_by(f32::total_cmp);
    h.click(200.0, tops[nth] + 12.0);
    h.frame();
}

#[test]
fn a_value_is_rows_two_levels_open_that_say_what_they_hold() {
    let (h, _) = page();
    let top = ["name:", "replicas:", "ready:", "selector:", "labels:", "app:", "tier.kind:", "containers:"];
    let inner = ["[0]", "name:", "image:", "ports:", "resources:"];
    assert_eq!(labels(&h), [&top[..], &inner[..]].concat());
    // Scalars as they are written, in quotes when they are texts; what is closed says how
    // many it holds, and what is open does not.
    for said in ["\"web\"", "3", "true", "null", "\"registry.local/web:1.4.2\"", "[2]", "{1}"] {
        assert!(h.has_text(said), "{said} is not said:\n{}", h.semantics());
    }
    assert!(!h.has_text("{2}") && !h.has_text("[1]"));
    assert!(h.has_text("Select a value"));
    // Rows are 24 px under a bar of 28, a level 16 px in.
    let rows = h.query(|n| n.semantics.role == Role::TreeItem && n.semantics.name == "app:");
    let words = h.query(|n| n.semantics.role == Role::Text && n.semantics.name == "app:");
    let (row, label) = (h.tree.window_rect(rows[0]), h.tree.window_rect(words[0]));
    assert_eq!((row.height, label.x), (24.0, 8.0 + 16.0 + 16.0 + 2.0 + 4.0));
}

#[test]
fn a_row_is_selected_opened_and_its_path_copied() {
    let (mut h, value) = page();
    // A click selects, and the bar says the path; a second one opens what holds things.
    click(&mut h, "ports:", 0);
    assert!(h.has_text(".containers[0].ports") && !h.has_text("8080"));
    click(&mut h, "ports:", 0);
    assert!(h.has_text("8080") && h.has_text("9090") && !h.has_text("[2]"));
    // The key for copying, with the focus in the rows, and the button: the path.
    h.key(Key::C, Mods::CTRL);
    assert_eq!(h.clipboard.read_text().as_deref(), Some(".containers[0].ports"));
    click(&mut h, "tier.kind:", 0);
    h.get_by_role(Role::Button, "Copy path").click();
    assert_eq!(h.clipboard.read_text().as_deref(), Some(".labels[\"tier.kind\"]"));
    // The path leads to the value.
    let read = value.get_untracked();
    assert_eq!(read.at(".labels[\"tier.kind\"]"), Some(&Value::Text("frontend".to_string())));
    // The arrows move, the left one closes.
    click(&mut h, "labels:", 0);
    h.key(Key::Left, Mods::NONE);
    h.frame();
    assert!(!labels(&h).contains(&"app:".to_string()) && h.has_text("{2}"));

    // Another value: what was opened is still open where it is still there.
    value.set(Rc::new(parse_json(&SAMPLE.replace("8080", "7070")).unwrap()));
    h.frame();
    assert!(h.has_text("7070") && h.has_text("9090") && !h.has_text("8080"));
    assert!(!labels(&h).contains(&"app:".to_string()));
}
