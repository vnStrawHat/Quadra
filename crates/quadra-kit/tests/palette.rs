//! The command palette (specs/behaviour/15, "Picker / command palette"): a query over the
//! commands that match it, whatever the case and the accents, the best first.

use std::rc::Rc;

use quadra_kit::{Command, button, command_palette, dark, fuzzy, fuzzy_hits};
use quadra_platform::{Key, Mods};
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::Role;

struct Page {
    h: Harness,
    open: Signal<bool>,
    ran: Signal<Vec<&'static str>>,
}

fn page() -> Page {
    let (open, ran) = (signal(false), signal(Vec::new()));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_kit::keymap()));
    h.mount(move || {
        let did = move |what: &'static str| move || ran.update(|all| all.push(what));
        let commands = move || {
            vec![
                Command::new("Save file", did("save")).keys("ctrl-s"),
                Command::new("Open folder", did("open")),
                Command::new("H\u{1EA3}i Ph\u{F2}ng weather", did("weather")),
                Command::new("Toggle sidebar", did("sidebar")).keys("ctrl-b"),
            ]
        };
        let ask = button("Commands").test_id("ask").on_click(move || open.set(true));
        quadra_kit::root().child(ask).child(command_palette(open, commands))
    });
    h.frame();
    Page { h, open, ran }
}

impl Page {
    fn settle(&mut self) {
        self.h.frame();
        while self.h.frame_if_needed().is_some() {}
    }

    /// The titles of the rows, the highlighted one in brackets.
    fn rows(&mut self) -> Vec<String> {
        let rows = self.h.query(|n| n.semantics.role == Role::ListItem);
        let says = self.h.tree.semantic_tree();
        let name = |id: &quadra_tree::NodeId| {
            let node = says.nodes.iter().find(|n| self.h.tree.node_of(n.id) == Some(*id)).unwrap();
            let name = node.semantics.name.clone();
            if node.semantics.selected { format!("[{name}]") } else { name }
        };
        rows.iter().map(name).collect()
    }

    fn ask(&mut self, query: &str) -> Vec<String> {
        self.h.get_by_role(Role::TextInput, "Type a command…").set_value(query);
        self.settle();
        self.rows()
    }
}

#[test]
fn a_query_finds_the_commands_that_match_it_whatever_the_case_and_the_accents() {
    let mut p = page();
    let ask = p.h.get_by_test_id("ask").node();
    p.h.tree.focus(ask);
    p.h.key(Key::Enter, Mods::NONE);
    p.settle();
    // 608 px with its border, under the top of the window, with the focus in its query.
    let panel = p.h.tree.window_rect(p.h.tree.overlays().next().unwrap());
    let field = p.h.get_by_role(Role::TextInput, "Type a command…").node();
    assert_eq!((panel.width, panel.y, p.h.tree.focused()), (608.0, 80.0, Some(field)));
    assert_eq!(p.rows(), ["[Save file]", "Open folder", "H\u{1EA3}i Ph\u{F2}ng weather", "Toggle sidebar"]);
    // A row is 32 px, across the palette less its border and 4 px at each side; its keys are at its end.
    let (row, keys) = (p.h.get_by_role(Role::ListItem, "Save file").node(), p.h.get_by_text("Ctrl-S").node());
    let (row, keys) = (p.h.tree.window_rect(row), p.h.tree.window_rect(keys));
    assert_eq!((row.width, row.height, keys.right()), (598.0, 32.0, row.right() - 6.0));
    assert!(p.h.has_text("Ctrl-B"));

    assert_eq!(p.ask("SV"), ["[Save file]"]);
    assert_eq!(p.ask("hai ph"), ["[H\u{1EA3}i Ph\u{F2}ng weather]"]);
    // The best match is first: letters at the start and in a row count for more.
    assert_eq!(p.ask("o"), ["[Open folder]", "H\u{1EA3}i Ph\u{F2}ng weather", "Toggle sidebar"]);
    assert_eq!(p.ask("zz"), [] as [&str; 0]);
    assert!(p.h.has_text("No commands"));

    // The arrows go round the matches; Enter runs the one they are on and closes the palette.
    p.ask("");
    p.h.key(Key::Down, Mods::NONE);
    p.settle();
    assert_eq!(p.rows()[1], "[Open folder]");
    p.h.key(Key::Up, Mods::NONE);
    p.h.key(Key::Up, Mods::NONE);
    p.settle();
    assert_eq!(p.rows()[3], "[Toggle sidebar]");
    p.h.key(Key::Enter, Mods::NONE);
    p.settle();
    assert_eq!(
        (p.open.get_untracked(), p.ran.get_untracked(), p.h.tree.focused()),
        (false, vec!["sidebar"], Some(ask))
    );

    // A click on a row runs it; Escape gives the palette up; each time it opens the query is empty.
    p.open.set(true);
    p.settle();
    assert_eq!(p.rows().len(), 4);
    p.h.get_by_role(Role::ListItem, "Open folder").click();
    p.settle();
    assert_eq!(p.ran.get_untracked(), ["sidebar", "open"]);
    p.open.set(true);
    p.settle();
    p.h.key(Key::Escape, Mods::NONE);
    p.settle();
    assert_eq!((p.open.get_untracked(), p.ran.get_untracked().len()), (false, 2));
}

#[test]
fn letters_match_in_order_and_score_by_where_they_are() {
    assert_eq!(fuzzy_hits("sf", "Save file"), Some(vec![0, 5]));
    assert_eq!(fuzzy_hits("viet", "Ti\u{1EBF}ng Vi\u{1EC7}t"), Some(vec![6, 7, 8, 9]));
    assert_eq!((fuzzy_hits("fs", "Save file"), fuzzy("x", "Save")), (None, None));
    // At the start, and one after another, counts for more.
    assert!(fuzzy("sa", "Save").unwrap() > fuzzy("sv", "Save").unwrap());
    assert!(fuzzy("s", "Save").unwrap() > fuzzy("a", "Save").unwrap());
    assert_eq!(fuzzy("", "anything"), Some(0));
}
