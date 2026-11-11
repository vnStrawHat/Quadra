//! Views mounted in a headless window: a component runs once, and a change runs only the
//! bindings that read it (specs/05; the structural budgets of specs/14).

use quadra_prim::Color;
use quadra_reactive::{Signal, signal, stats};
use quadra_testing::Harness;
use quadra_tree::{FrameStats, NodeId};
use quadra_ui::{column, div, dynamic, each, mount_root, row, show, text};

/// The text of every text node under `id`, in order.
fn texts(h: &mut Harness, id: NodeId, out: &mut Vec<String>) {
    if let Some(text) = h.tree.widget_mut::<quadra_tree::Text>(id) {
        out.push(text.text().to_string());
    }
    for child in h.tree.children(id).to_vec() {
        texts(h, child, out);
    }
}

fn all_texts(h: &mut Harness) -> Vec<String> {
    let mut out = Vec::new();
    let root = h.tree.root().unwrap();
    texts(h, root, &mut out);
    out
}

#[test]
fn a_component_runs_once_and_a_change_runs_one_binding() {
    let runs = signal(0);
    let count = signal(0);
    let counter = move || {
        runs.update(|n| *n += 1);
        row().gap(8.0).child(text("count")).child(text(move || count.get().to_string()).size(10.0))
    };
    let mut h = Harness::new();
    mount_root(&mut h.tree, counter());
    h.frame();
    assert_eq!(all_texts(&mut h), ["count", "0"]);
    for n in 1..=5 {
        count.set(n);
        let (stats, drawn) = h.frame();
        assert_eq!((stats.bindings_run, stats.writes, stats.nodes_painted), (1, 1, 1));
        assert_eq!(drawn.instances_uploaded, 1); // one glyph
    }
    assert_eq!((runs.get(), all_texts(&mut h)[1].as_str()), (1, "5"));
    count.set(5); // equal: nothing is due
    assert!(h.frame_if_needed().is_none());
}

#[test]
fn any_property_takes_a_constant_a_signal_or_a_closure() {
    let (wide, on) = (signal(120.0), signal(false));
    let mut h = Harness::new();
    let view = column().child(
        div()
            .width(wide)
            .height(40.0)
            .bg(move || if on.get() { Color::rgb(0x00AA00) } else { Color::rgb(0xAA0000) }),
    );
    let root = mount_root(&mut h.tree, view);
    h.frame();
    let cell = h.tree.children(root)[0];
    assert_eq!((h.tree.rect(cell).width, h.tree.paint_style(cell).background), (120.0, Color::rgb(0xAA0000)));
    on.set(true); // a colour: painted, not laid out
    let (stats, _) = h.frame();
    assert_eq!((stats.nodes_painted, stats.nodes_laid_out), (1, 0));
    wide.set(200.0);
    h.frame();
    assert_eq!(h.tree.rect(cell).width, 200.0);
}

#[test]
fn a_keyed_list_keeps_the_nodes_of_the_keys_that_stay() {
    let items = signal(vec![(1, "one"), (2, "two"), (3, "three")]);
    let view =
        each(move || items.get(), |item| item.0, |item: Signal<(i32, &str)>| text(move || item.get().1));
    let mut h = Harness::new();
    let list = mount_root(&mut h.tree, view);
    h.frame();
    let before = h.tree.children(list).to_vec();
    assert_eq!(all_texts(&mut h), ["one", "two", "three"]);

    items.set(vec![(3, "three"), (4, "four"), (1, "ONE")]); // moved, new, changed, and one gone
    h.frame();
    let after = h.tree.children(list).to_vec();
    assert_eq!(all_texts(&mut h), ["three", "four", "ONE"]);
    assert_eq!((after[0], after[2]), (before[2], before[0])); // the same nodes, in new places
    assert!(!h.tree.is_alive(before[1]) && !before.contains(&after[1]));

    items.update(|v| v[1].1 = "FOUR"); // one item changes: its row's binding, nothing else
    let (stats, _): (FrameStats, _) = h.frame();
    assert_eq!((stats.nodes_painted, h.tree.children(list).to_vec()), (1, after));
}

#[test]
fn a_part_appears_and_disappears_and_its_state_goes_with_it() {
    let (open, label) = (signal(false), signal("panel"));
    let made = signal(0);
    let panel = move || {
        made.update(|n| *n += 1);
        let local = signal(7); // a component's own state: it lives as long as the part
        text(move || format!("{} {}", label.get(), local.get()))
    };
    let mut h = Harness::new();
    mount_root(&mut h.tree, column().child(text("top")).child(show(move || open.get(), panel)));
    h.frame();
    let closed = stats();
    assert_eq!((all_texts(&mut h), made.get()), (vec!["top".to_string()], 0));

    open.set(true);
    h.frame();
    assert_eq!(all_texts(&mut h), ["top", "panel 7"]);
    label.set("renamed"); // what the part reads updates it in place: it is not built again
    h.frame();
    assert_eq!((all_texts(&mut h)[1].as_str(), made.get()), ("renamed 7", 1));

    open.set(false);
    h.frame();
    assert_eq!(all_texts(&mut h), ["top"]);
    assert_eq!(stats(), closed); // its signal, its binding and its scope are gone
}

#[test]
fn a_dynamic_region_is_built_again_when_what_it_reads_changes() {
    let kind = signal(1);
    let view = dynamic(move || match kind.get() {
        1 => column().child(text("one")),
        n => column().child(text("many")).child(text(n.to_string())),
    });
    let mut h = Harness::new();
    mount_root(&mut h.tree, view);
    h.frame();
    assert_eq!(all_texts(&mut h), ["one"]);
    let nodes = h.tree.len();
    kind.set(3);
    h.frame();
    assert_eq!((all_texts(&mut h), h.tree.len()), (vec!["many".to_string(), "3".to_string()], nodes + 1));
}

#[test]
fn removing_the_root_frees_everything_the_views_made() {
    let before = stats();
    let n = signal(0);
    let mut h = Harness::new();
    let cell = move |i: i32| div().size(10.0, 10.0).child(text(move || (n.get() + i).to_string()));
    let root = mount_root(&mut h.tree, row().children((0..50).map(cell)));
    h.frame();
    assert!(stats().nodes > before.nodes + 50);
    h.tree.remove(root);
    let after = stats();
    assert_eq!((after.nodes, after.scopes), (before.nodes + 1, before.scopes)); // only `n` is left
}
