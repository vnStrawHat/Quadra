//! The links of rich text for the keyboard and a reader (specs/05, "Rules"): each is reached
//! by Tab, outlined while it has the focus, opened by Enter, and is a link to a reader.

use quadra_platform::{Key, Mods};
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::{Role, SemanticAction};
use quadra_view::{column, rich_text, span};

#[test]
fn links_are_reached_by_tab_opened_by_enter_and_known_to_a_reader() {
    let clicked = signal(String::new());
    let mut h = Harness::new();
    h.mount(move || {
        let words = rich_text([span("see "), span("one").link("1"), span(" and "), span("two").link("2")]);
        column().width(300.0).text_size(10.0).child(words.on_link(move |to| clicked.set(to.to_string())))
    });
    h.frame();
    let instances = |h: &Harness| h.tree.scene().layer(h.tree.scene().roots()[0]).instances().len();
    let before = instances(&h);
    // Tab goes to the first link, and its outline is painted.
    h.key(Key::Tab, Mods::NONE);
    h.frame();
    let one = h.get_by_role(Role::Link, "one").node();
    assert_eq!(h.tree.focused(), Some(one));
    assert_eq!(instances(&h), before + 1);
    h.key(Key::Enter, Mods::NONE);
    assert_eq!(clicked.get_untracked(), "1");
    // Then to the second; the first is outlined no more.
    h.key(Key::Tab, Mods::NONE);
    h.frame();
    assert_eq!(h.tree.focused(), Some(h.get_by_role(Role::Link, "two").node()));
    assert_eq!(instances(&h), before + 1);
    // A reader opens a link by its action; the text is still one, named by all of it.
    h.get_by_role(Role::Link, "one").act(SemanticAction::Click);
    assert_eq!(clicked.get_untracked(), "1");
    assert!(h.has_text("see one and two"));
}
