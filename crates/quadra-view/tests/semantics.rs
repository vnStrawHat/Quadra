//! The semantic tree (specs/12): nodes that mean something to a user, with what holds them
//! skipped. A test finds controls in it and drives them as assistive technology would, and a
//! listener hears only of what changed.

use quadra_platform::{Key, Mods};
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::{Mirror, Role, SemanticAction};
use quadra_view::{Place, column, div, overlay, row, scroll, show, text, text_input};

struct Form {
    h: Harness,
    title: Signal<String>,
    done: Signal<bool>,
    saved: Signal<u32>,
}

/// A heading, a title field, a checkbox with a label, a button that is disabled while the
/// title is empty, and a long list in a scroll view.
fn form() -> Form {
    let (title, done, saved) = (signal(String::new()), signal(false), signal(0u32));
    let view = move || {
        let field = div().size(200.0, 20.0).child(text_input(title).size(10.0));
        let check = row().role(Role::Checkbox).checked(done).focusable();
        let check = check.on_click(move |_| done.update(|d| *d = !*d)).child(text("Done"));
        let save = div().role(Role::Button).test_id("save").disabled(move || title.get().is_empty());
        let save = save.on_click(move |_| saved.update(|n| *n += 1)).child(text("Save")).child(text("now"));
        let item = |i: u32| div().role(Role::ListItem).height(20.0).child(text(format!("Item {i}")));
        let list = scroll(signal(0.0), column().role(Role::List).label("Items").children((0..50).map(item)));
        let heading = div().role(Role::Heading).child(text("New note"));
        let saved_note =
            show(move || saved.get() > 0, move || text(move || format!("Saved {} times", saved.get())));
        column().child(heading).child(field).child(check).child(save).child(saved_note).child(list)
    };
    let mut h = Harness::new();
    h.mount(view);
    h.frame();
    Form { h, title, done, saved }
}

#[test]
fn the_tree_holds_what_means_something_named_by_what_it_holds() {
    let f = form();
    let tree = f.h.semantics();
    let lines: Vec<&str> = tree.lines().take(7).collect();
    assert_eq!(
        lines,
        [
            "Window",
            "  Heading \"New note\"",
            "  TextInput = \"\"",
            "  Checkbox \"Done\" [unchecked]",
            "  Button \"Save now\" [disabled] #save",
            "  ScrollView",
            "    List \"Items\"",
        ]
    );
    // The boxes between them are not in it, and neither are the texts that name a control.
    assert_eq!(tree.lines().filter(|l| l.contains("ListItem")).count(), 50);
    assert!(tree.contains("    ListItem \"Item 49\""));
    assert!(!tree.lines().any(|line| line.trim_start().starts_with("Text ")));
}

#[test]
fn a_test_finds_controls_and_drives_them_without_the_pointer() {
    let mut f = form();
    f.h.get_by_role(Role::TextInput, "").type_text("Plan");
    f.h.get_by_role(Role::Checkbox, "Done").click();
    f.h.frame();
    assert_eq!((f.title.get_untracked().as_str(), f.done.get_untracked()), ("Plan", true));
    assert!(f.h.semantics().contains("Checkbox \"Done\" [checked] [focused]")); // a click focuses
    assert!(f.h.semantics().contains("Button \"Save now\" #save")); // no longer disabled

    f.h.get_by_test_id("save").click();
    f.h.get_by_role(Role::Button, "Save now").click();
    f.h.frame();
    assert_eq!(f.saved.get_untracked(), 2);
    assert!(f.h.has_text("Saved 2 times") && !f.h.has_text("Saved 1 times"));

    f.h.get_by_text("Plan").set_value("Plan B"); // found by what it holds; replaced whole
    f.h.frame();
    assert_eq!(f.title.get_untracked(), "Plan B");
    // An item far below the window: a click reaches it all the same.
    let item = f.h.get_by_text("Item 40").node();
    assert!(f.h.tree.window_rect(item).y > 600.0);
    assert!(f.h.tree.act(item, &SemanticAction::Click) && !f.h.tree.act(item, &SemanticAction::Toggle));
    // Toggling a checkbox that registered nothing for it is a click on it.
    f.h.get_by_role(Role::Checkbox, "Done").act(SemanticAction::Toggle);
    assert!(!f.done.get_untracked());
}

#[test]
#[should_panic(expected = "2 nodes are called \"Twin\", not one")]
fn a_query_that_finds_two_nodes_fails_with_the_tree() {
    let mut h = Harness::new();
    h.mount(|| column().child(text("Twin")).child(text("Twin")));
    h.frame();
    h.get_by_text("Twin");
}

#[test]
fn a_listener_hears_everything_once_and_then_what_changed() {
    let mut f = form();
    assert!(f.h.tree.take_semantic_update().is_none()); // nobody listens: nothing is built
    f.h.tree.listen(true);
    let first = f.h.tree.take_semantic_update().unwrap();
    assert_eq!(first.nodes.len(), 1 + 6 + 50); // the window, five controls and a scroll view, fifty items
    assert!(f.h.tree.take_semantic_update().is_none()); // nothing changed

    f.done.set(true);
    f.h.frame();
    let update = f.h.tree.take_semantic_update().unwrap();
    let roles: Vec<Role> = update.nodes.iter().map(|n| n.semantics.role).collect();
    assert_eq!((roles, update.nodes[0].semantics.checked), (vec![Role::Checkbox], Some(true)));

    f.h.get_by_role(Role::TextInput, "").type_text("x"); // the value, the button's state, the focus
    f.h.frame();
    let update = f.h.tree.take_semantic_update().unwrap();
    let mut roles: Vec<Role> = update.nodes.iter().map(|n| n.semantics.role).collect();
    roles.sort_by_key(|r| format!("{r:?}"));
    assert_eq!(roles, [Role::Button, Role::TextInput]);
    assert_eq!(update.focus, update.nodes.iter().find(|n| n.focused).map(|n| n.id));
    f.h.tree.listen(false);
    f.done.set(false);
    f.h.frame();
    assert!(f.h.tree.take_semantic_update().is_none());
}

/// Runs a frame and gives its update to a listener: what the listener then holds is the tree.
fn follow(mirror: &mut Mirror, h: &mut Harness) {
    h.frame();
    if let Some(update) = h.tree.take_semantic_update() {
        mirror.apply(update).unwrap();
    }
    let mut tree = h.tree.semantic_tree().nodes;
    tree.sort_by_key(|n| n.id);
    assert_eq!(mirror.nodes(), tree);
}

#[test]
fn a_listener_that_applies_what_it_is_sent_holds_the_tree() {
    let mut f = form();
    let mut mirror = Mirror::default();
    f.h.tree.listen(true);
    follow(&mut mirror, &mut f.h);
    f.done.set(true);
    follow(&mut mirror, &mut f.h);
    f.h.get_by_role(Role::TextInput, "").type_text("Plan"); // a value, a state, the focus
    follow(&mut mirror, &mut f.h);
    f.h.get_by_test_id("save").click(); // a text that was not there appears
    follow(&mut mirror, &mut f.h);
    f.h.wheel(100.0, 300.0, -3.0); // the list scrolls: its items are elsewhere
    follow(&mut mirror, &mut f.h);

    // A menu above the window, a box that means nothing and can have the focus, and a text
    // that is at once a node of its own and part of an item's name.
    let (open, name) = (signal(false), signal("one".to_string()));
    let menu = move || {
        let item = div().role(Role::MenuItem).child(text(name));
        overlay(Place::Center, column().role(Role::Menu).child(item).child(text("Close")))
    };
    let mut h = Harness::new();
    h.mount(move || {
        let page = column().child(div().size(50.0, 20.0).focusable()).child(div().child(text(name)));
        page.child(show(move || open.get(), menu))
    });
    let mut mirror = Mirror::default();
    h.frame();
    h.tree.listen(true);
    follow(&mut mirror, &mut h);
    open.set(true);
    follow(&mut mirror, &mut h);
    assert!(h.semantics().contains("  Menu\n    MenuItem \"one\"\n    Text \"Close\""));
    name.set("two".to_string());
    follow(&mut mirror, &mut h);
    h.key(Key::Tab, Mods::NONE); // the focus is on what means nothing: the window has it
    follow(&mut mirror, &mut h);
    let tree = h.tree.semantic_tree();
    assert_eq!(tree.focus, Some(tree.root));
    open.set(false);
    follow(&mut mirror, &mut h);
    assert!(!h.semantics().contains("Menu"));
}

#[test]
fn a_field_is_named_and_described_by_other_nodes_and_a_tab_controls_its_panel() {
    use quadra_tree::Relation;
    use quadra_view::NodeRef;
    let (label, hint, panel) = (NodeRef::new(), NodeRef::new(), NodeRef::new());
    let (l, d, p) = (label.clone(), hint.clone(), panel.clone());
    let mut h = Harness::new();
    h.mount(move || {
        let field = div().role(Role::TextInput).labelled_by(&l).described_by(&d).focusable();
        let tab = div().role(Role::Tab).controls(&p).child(text("General"));
        column()
            .child(div().node_ref(&l).child(text("Title")))
            .child(field)
            .child(div().node_ref(&d).child(text("Shown in the list")))
            .child(tab)
            .child(div().role(Role::TabPanel).node_ref(&p).child(text("Body")))
    });
    h.frame();
    let tree = h.tree.semantic_tree();
    let says = |role: Role| &tree.nodes.iter().find(|n| n.semantics.role == role).unwrap().semantics;
    // Each points at the node of the semantic tree that holds the other: the texts themselves.
    let id_of = |name: &str| tree.nodes.iter().find(|n| n.semantics.name == name).unwrap().id;
    assert_eq!(
        says(Role::TextInput).relations,
        [(Relation::LabelledBy, id_of("Title")), (Relation::DescribedBy, id_of("Shown in the list"))]
    );
    let panel_id = tree.nodes.iter().find(|n| n.semantics.role == Role::TabPanel).unwrap().id;
    assert_eq!(says(Role::Tab).relations, [(Relation::Controls, panel_id)]);
}
