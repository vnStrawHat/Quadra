//! Settings (specs/13, "Components"): pages of rows in a column and a page, in both layouts.

use std::rc::Rc;

use quadra_kit::{Settings, SettingsGroup, SettingsPage, dark, settings};
use quadra_platform::{Key, Mods};
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::{Role, SemanticAction};
use quadra_ui::div;

/// Two short pages, and one long enough to scroll.
fn prefs(current: Signal<usize>, query: Signal<String>) -> Settings {
    let group = |title: &str, rows: &[&str]| {
        let about = |row: &&str| format!("About {row}");
        rows.iter()
            .fold(SettingsGroup::new(title), |group, row| group.row(row, &about(row), div().size(20.0, 20.0)))
    };
    let many: Vec<String> = (0..10).map(|i| format!("Row {i}")).collect();
    let many: Vec<&str> = many.iter().map(String::as_str).collect();
    let general =
        SettingsPage::new("General").group(group("General Settings", &["Autosave", "Restore Session"]));
    let editor = SettingsPage::new("Editor")
        .group(group("Cursor", &["Cursor Blink"]))
        .group(group("Font", &["Font Size"]));
    let long = SettingsPage::new("Long").group(group("First", &many)).group(group("Second", &many));
    let long = long.group(group("Third", &many));
    settings(current).page(general).page(editor).page(long).search(query)
}

fn mounted(cards: bool) -> (Harness, Signal<usize>, Signal<String>) {
    let (current, query) = (signal(0), signal(String::new()));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_kit::keymap()));
    h.mount(move || {
        let all = prefs(current, query);
        quadra_kit::root().child(if cards { all.cards() } else { all })
    });
    settle(&mut h);
    (h, current, query)
}

fn settle(h: &mut Harness) {
    h.frame();
    while h.frame_if_needed().is_some() {}
}

/// Whether the column has a row of that name, and whether it is the chosen one.
fn item(h: &Harness, name: &str) -> Option<bool> {
    let tree = h.tree.semantic_tree();
    let is = |n: &&quadra_tree::SemanticNode| n.semantics.role == Role::TreeItem && n.semantics.name == name;
    tree.nodes.iter().find(is).map(|n| n.semantics.selected)
}

/// Where what the page (right of the column) calls `name` is, from the top.
fn top_of(h: &Harness, name: &str) -> f32 {
    let said = h.query(|n| n.semantics.name == name);
    let rects = said.iter().map(|id| h.tree.window_rect(*id));
    rects.filter(|rect| rect.x > 260.0).map(|rect| rect.y).next().expect("the page says it")
}

#[test]
fn pages_are_chosen_in_the_column_and_a_search_keeps_what_matches() {
    let (mut h, current, query) = mounted(false);
    assert!(h.has_text("Autosave") && h.has_text("General Settings"));
    assert_eq!(
        (item(&h, "General"), item(&h, "Editor"), item(&h, "Cursor")),
        (Some(true), Some(false), None)
    );

    // The chevron shows the page's groups without choosing the page; a reader hides them.
    let editor = h.get_by_role(Role::TreeItem, "Editor").node();
    let editor = h.tree.window_rect(editor);
    h.click(editor.x + 14.0, editor.center().y);
    settle(&mut h);
    assert_eq!(
        (item(&h, "Cursor"), h.has_text("Cursor Blink"), current.get_untracked()),
        (Some(false), false, 0)
    );
    h.get_by_role(Role::TreeItem, "Editor").act(SemanticAction::Collapse);
    settle(&mut h);
    assert_eq!(item(&h, "Cursor"), None);

    // A group chooses its page; so do a click on a page, and Enter with the focus on it.
    h.get_by_role(Role::TreeItem, "Editor").act(SemanticAction::Expand);
    settle(&mut h);
    h.get_by_role(Role::TreeItem, "Cursor").click();
    settle(&mut h);
    assert_eq!(
        (h.has_text("Cursor Blink"), h.has_text("Autosave"), current.get_untracked()),
        (true, false, 1)
    );
    h.get_by_role(Role::TreeItem, "General").click();
    settle(&mut h);
    assert!(h.has_text("Autosave") && !h.has_text("Cursor Blink"));
    h.get_by_role(Role::TreeItem, "Editor").focus();
    h.key(Key::Enter, Mods::NONE);
    settle(&mut h);
    assert_eq!((item(&h, "Editor"), h.has_text("Cursor Blink")), (Some(true), true));

    // A search keeps the rows that hold the text, whatever the case and the accents; a page
    // that keeps none leaves the column.
    current.set(0);
    query.set("RÉSTORE".to_string());
    settle(&mut h);
    assert!(h.has_text("Restore Session") && !h.has_text("Autosave"));
    assert_eq!((item(&h, "General"), item(&h, "Editor")), (Some(true), None));
}

#[test]
fn a_chosen_group_comes_to_the_top_of_its_page() {
    let (mut h, current, _) = mounted(false);
    h.get_by_role(Role::TreeItem, "Long").act(SemanticAction::Expand);
    settle(&mut h);
    h.get_by_role(Role::TreeItem, "Second").click();
    settle(&mut h);
    // The header's text is where the page's padding ends; the page's row brings the top back.
    assert_eq!((current.get_untracked(), top_of(&h, "Second")), (2, 24.0));
    h.get_by_role(Role::TreeItem, "Long").click();
    settle(&mut h);
    assert_eq!(top_of(&h, "Long"), 24.0);

    // The cards: the group itself comes under the page's header.
    let (mut h, _, _) = mounted(true);
    h.get_by_role(Role::TreeItem, "Long").click();
    settle(&mut h);
    let first = top_of(&h, "First");
    h.get_by_role(Role::TreeItem, "Third").click();
    settle(&mut h);
    assert_eq!((top_of(&h, "Third"), item(&h, "Third"), item(&h, "Long")), (first, Some(true), Some(false)));
}

#[test]
fn the_cards_list_the_groups_of_a_page_under_its_row() {
    let (mut h, current, query) = mounted(true);
    assert!(h.has_text("Autosave") && h.has_text("General Settings"));

    // A click on a page chooses it and opens its groups; the chevron alone closes them.
    h.get_by_role(Role::TreeItem, "Editor").click();
    settle(&mut h);
    assert_eq!(
        (h.has_text("Cursor Blink"), item(&h, "Font"), item(&h, "Editor")),
        (true, Some(false), Some(true))
    );
    let editor = h.get_by_role(Role::TreeItem, "Editor").node();
    let editor = h.tree.window_rect(editor);
    h.click(editor.right() - 18.0, editor.center().y);
    settle(&mut h);
    assert_eq!((h.has_text("Cursor Blink"), item(&h, "Font"), current.get_untracked()), (true, None, 1));
    h.click(editor.right() - 18.0, editor.center().y);
    settle(&mut h);
    h.get_by_role(Role::TreeItem, "Font").click();
    settle(&mut h);
    assert_eq!(
        (item(&h, "Font"), item(&h, "Editor"), h.has_text("Font Size")),
        (Some(true), Some(false), true)
    );

    // The chosen page keeps nothing of a search: the first page that keeps a row shows, and
    // the choice stays. A page with one group left lists none.
    current.set(0);
    query.set("font size".to_string());
    settle(&mut h);
    assert_eq!((item(&h, "General"), item(&h, "Font"), current.get_untracked()), (None, None, 0));
    assert!(h.has_text("Font Size") && !h.has_text("Autosave") && !h.has_text("Cursor Blink"));
}

#[test]
fn a_group_of_settings_stands_on_its_own_in_a_page() {
    let fps = signal(false);
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_kit::keymap()));
    h.mount(move || {
        let group =
            SettingsGroup::new("Interface").row("Show FPS monitor", "Frame times.", quadra_kit::switch(fps));
        quadra_kit::root().child(quadra_kit::settings_group(group))
    });
    h.frame();
    // Its title is a heading; a row is a group named by its title, and its control works.
    assert!(h.semantics().contains("Heading"), "{}", h.semantics());
    h.get_by_role(Role::Group, "Show FPS monitor");
    h.get_by_role(Role::Switch, "").click();
    h.frame();
    assert!(fps.get_untracked());
}
