//! The palette's parts (specs/13, the command palette): groups under headings, a lead and a
//! state in a row, a note in the query, a line under the rows, and a picker under a control
//! whose current row is first.

use std::rc::Rc;

use quadra_design::{Command, Status, Tone, badge, button, command_palette, dark, key_label, token};
use quadra_platform::{Key, Mods};
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::Role;
use quadra_view::{NodeRef, div};

fn harness() -> Harness {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h
}

fn settle(h: &mut Harness) {
    h.frame();
    while h.frame_if_needed().is_some() {}
}

/// The rows and the headings, top to bottom, the row the arrows are on in brackets.
fn lines(h: &mut Harness) -> Vec<String> {
    let found = h.query(|n| matches!(n.semantics.role, Role::ListItem | Role::Heading));
    let says = h.tree.semantic_tree();
    let mut lines: Vec<(f32, String)> = found
        .iter()
        .map(|id| {
            let node = says.nodes.iter().find(|n| h.tree.node_of(n.id) == Some(*id)).unwrap();
            let name = node.semantics.name.clone();
            let name = match node.semantics.role {
                Role::Heading => format!("# {name}"),
                _ if node.semantics.selected => format!("[{name}]"),
                _ => name,
            };
            (h.tree.window_rect(*id).y, name)
        })
        .collect();
    lines.sort_by(|a, b| a.0.total_cmp(&b.0));
    lines.into_iter().map(|(_, name)| name).collect()
}

#[test]
fn rows_come_in_groups_under_headings_with_their_parts_a_note_and_a_line_under_them() {
    let (open, ran) = (signal(true), signal(Vec::<&str>::new()));
    let mut h = harness();
    h.mount(move || {
        let did = move |what: &'static str| move || ran.update(|all| all.push(what));
        let commands = move || {
            vec![
                Command::new("worker-07", did("worker"))
                    .group("Resources")
                    .lead(|| badge("Node", Tone::Neutral))
                    .status(Status::Failed, "NotReady")
                    .detail("23 pods")
                    .keys("ctrl-shift-c"),
                Command::new("Cordon", did("cordon")).group("Commands for this node"),
                Command::new("api-server", did("api"))
                    .group("Resources")
                    .lead(|| badge("Pod", Tone::Neutral)),
                Command::new("Drain", did("drain")).group("Commands for this node"),
            ]
        };
        let palette = command_palette(open, commands).note(|| "in prod-cluster".to_string());
        let palette = palette.foot("Enter open · Esc close", |n| format!("{n} results"));
        quadra_design::root().child(palette)
    });
    settle(&mut h);
    let all = ["# Resources", "[worker-07]", "api-server", "# Commands for this node", "Cordon", "Drain"];
    assert_eq!(lines(&mut h), all);
    for words in
        ["Node", "Pod", "NotReady · 23 pods", "Ctrl-Shift-C", "in prod-cluster", "Enter open · Esc close"]
    {
        assert!(h.has_text(words), "{words}");
    }
    // The state right after the title, muted; the note in the query mono and in text_2.
    let theme = dark().theme();
    let title = h.query(|n| n.semantics.role == Role::Text && n.semantics.name == "worker-07")[0];
    let state = h.get_by_text("NotReady · 23 pods").node();
    let (title_at, state_at) = (h.tree.window_rect(title), h.tree.window_rect(state));
    assert!(
        state_at.x > title_at.right() && state_at.x - title_at.right() < 30.0,
        "{title_at:?} {state_at:?}"
    );
    assert_eq!(h.tree.inherited(state).color, theme.resolve(token::TEXT_2.into()));
    let note = h.get_by_text("in prod-cluster").node();
    let note = h.tree.inherited(note);
    assert_eq!(
        (note.style.family, note.color),
        (quadra_design::fonts::MONO.into(), theme.resolve(token::TEXT_2.into()))
    );
    assert!(h.has_text("4 results"));
    // The arrows pass the headings by; Enter runs the row they are on.
    h.key(Key::Down, Mods::NONE);
    h.key(Key::Down, Mods::NONE);
    settle(&mut h);
    assert_eq!(lines(&mut h)[4], "[Cordon]");
    // A query keeps the groups, the best of each first; the count follows it.
    h.get_by_role(Role::TextInput, "Type a command…").set_value("r");
    settle(&mut h);
    let rows = lines(&mut h).iter().filter(|l| !l.starts_with('#')).count();
    assert!(h.has_text(&format!("{rows} results")));
    h.get_by_role(Role::TextInput, "Type a command…").set_value("dra");
    settle(&mut h);
    assert_eq!(lines(&mut h), ["# Commands for this node", "[Drain]"]);
    assert!(h.has_text("1 results"));
    h.key(Key::Enter, Mods::NONE);
    settle(&mut h);
    assert_eq!((ran.get_untracked(), open.get_untracked()), (vec!["drain"], false));
}

fn picker(h: &mut Harness, open: Signal<bool>, at: NodeRef, long: &'static str) {
    h.mount(move || {
        let commands = move || {
            vec![
                Command::new(long, || {}).status(Status::Failed, "unreachable"),
                Command::new("prod-cluster", || {}).status(Status::Running, "connected").current(),
            ]
        };
        let opener = div().node_ref(&at).child(button("prod-cluster").on_click(move || open.set(true)));
        let picker = command_palette(open, commands).under(&at).width(280.0).placeholder("Find a cluster…");
        quadra_design::root().child(div().padding(40.0).child(opener)).child(picker)
    });
}

#[test]
fn a_picker_hangs_under_its_opener_its_current_row_first_each_title_on_one_line() {
    let (open, at) = (signal(false), NodeRef::new());
    let mut h = harness();
    let long = "staging-cluster-in-the-region-far-to-the-west-with-a-very-long-name";
    picker(&mut h, open, at.clone(), long);
    settle(&mut h);
    h.get_by_role(Role::Button, "prod-cluster").click();
    settle(&mut h);
    // 280 px, its left edge under the opener's, 4 px under it.
    let opener = h.tree.window_rect(at.get().unwrap());
    let panel = h.tree.window_rect(h.tree.overlays().next().unwrap());
    assert_eq!((panel.width, panel.x, panel.y), (280.0, opener.x, opener.bottom() + 4.0));
    assert_eq!(lines(&mut h), ["[prod-cluster]", long]);
    // The long title is cut: no letter is painted under its row, and the state still shows.
    let row = h.get_by_role(Role::ListItem, long).node();
    let row = h.tree.window_rect(row);
    let layers = h.renderer.steps().iter().filter_map(|step| match step {
        quadra_render::Step::Layer { id, world } => Some((h.tree.scene().layer(*id), world.transform.ty)),
        quadra_render::Step::Run { .. } => None,
    });
    let glyphs = layers.flat_map(|(layer, y)| layer.instances().iter().map(move |i| (i, y)));
    let glyphs = glyphs.filter(|(i, _)| i.flags & quadra_render::FLAG_GLYPH != 0);
    let lowest = glyphs.map(|(i, y)| i.bounds[1] + i.bounds[3] + y).fold(0.0f32, f32::max);
    assert!(lowest > 0.0 && lowest <= row.bottom(), "{lowest} under {row:?}");
    let state = h.get_by_text("unreachable").node();
    assert!(row.contains(h.tree.window_rect(state).center()));
}

#[test]
fn a_dense_picker_has_mono_names_muted_states_a_footnote_and_keys_as_hints() {
    let open = signal(true);
    let mut h = harness();
    h.mount(move || {
        let commands = || {
            vec![
                Command::new("prod-cluster", || ()).mono().status(Status::Running, "connected").current(),
                Command::new("staging", || ()).mono().status(Status::Failed, "unreachable"),
            ]
        };
        let picker = command_palette(open, commands).dense().hint(false).width(280.0);
        let picker = picker.footnote("Every view shows the selected cluster only.");
        div().child(picker.foot_hints([("enter", "open"), ("escape", "close")], |n| format!("{n} clusters")))
    });
    settle(&mut h);
    let rows = h.query(|n| n.semantics.role == Role::ListItem);
    assert_eq!(rows.iter().map(|&row| h.tree.window_rect(row).height).collect::<Vec<_>>(), [24.0, 24.0]);
    // "Enter" only in the foot, a key as a key; the names in mono; the states' words muted.
    let enter = key_label("enter");
    assert_eq!(h.query(|n| n.semantics.name == enter).len(), 1);
    for said in ["open", "close", "2 clusters", "Every view shows the selected cluster only."] {
        assert!(h.has_text(said), "{said} is not said:\n{}", h.semantics());
    }
    let name = h.query(|n| n.semantics.role == Role::Text && n.semantics.name == "prod-cluster")[0];
    assert_eq!(h.tree.inherited(name).style.family, quadra_design::fonts::MONO.into());
    let word = h.get_by_text("unreachable").node();
    assert_eq!(h.tree.inherited(word).color, dark().theme().resolve(token::TEXT_3.into()));
}
