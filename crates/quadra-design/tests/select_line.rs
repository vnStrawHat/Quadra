//! A select's value and options (specs/13, "More of the fields"): the value one line however
//! narrow the field, cut with an ellipsis; an option's detail in its menu row alone.

use std::rc::Rc;

use quadra_design::{dark, select};
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::{Relation, Role};

const WORDS: &str = "No keys: add one in Settings";

#[test]
fn a_select_narrower_than_its_value_keeps_it_on_one_line() {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        let options = vec!["~/.ssh/id_ed25519_work_laptop".to_string()];
        quadra_design::root().child(select(options, signal(usize::MAX)).placeholder(WORDS).width(120.0))
    });
    h.frame();
    // The field is named by its words; they are the text in its first box.
    let field = h.get_by_text(WORDS).node();
    let holder = h.tree.children(field)[0];
    let node = h.tree.children(holder)[0];
    let (words, face) = (h.tree.window_rect(node), h.tree.window_rect(field));
    assert!(words.height <= face.height && words.bottom() <= face.bottom(), "{words:?} in {face:?}");
    assert!(words.right() <= face.right(), "{words:?} in {face:?}");
}

#[test]
fn an_option_s_detail_is_in_its_menu_row_alone_and_describes_it() {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        let way = select(["Local", "Remote", "Dynamic"], signal(0))
            .details(["this computer \u{2192} host", "", "SOCKS proxy"])
            .width(104.0);
        quadra_design::root().child(way)
    });
    h.frame();
    let says = |h: &Harness, words: &str| !h.query(|n| n.semantics.name.contains(words)).is_empty();
    assert!(!says(&h, "this computer"), "the field keeps the name alone");
    h.get_by_role(Role::Button, "Local").click();
    h.frame();
    while h.frame_if_needed().is_some() {}
    let told = |h: &Harness, name: &str| {
        let tree = h.tree.semantic_tree();
        let rows: Vec<_> = tree
            .nodes
            .iter()
            .filter(|n| n.semantics.role == Role::MenuItem && n.semantics.name == name)
            .collect();
        assert_eq!(
            rows.len(),
            1,
            "{name}: named by its name alone
{}",
            h.semantics()
        );
        // The detail's text, which the row points at as what describes it.
        let by = rows[0].semantics.relations.iter().find(|(kind, _)| *kind == Relation::DescribedBy);
        by.map(|(_, to)| tree.nodes.iter().find(|n| n.id == *to).unwrap().semantics.name.clone())
    };
    assert_eq!(told(&h, "Local").as_deref(), Some("this computer \u{2192} host"));
    assert_eq!(told(&h, "Remote"), None);
    assert_eq!(told(&h, "Dynamic").as_deref(), Some("SOCKS proxy"));
}
