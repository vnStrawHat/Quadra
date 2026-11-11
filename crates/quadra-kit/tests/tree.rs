//! The tree (specs/behaviour/09, "Data"): rows that open and close what they hold, one of
//! them selected by the pointer or the keys; only the rows that show are mounted.

use std::collections::HashSet;
use std::rc::Rc;

use quadra_kit::{TreeNode, dark, icons, tree};
use quadra_platform::{Key, Mods};
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::Role;

struct Page {
    h: Harness,
    chosen: Signal<Option<String>>,
    open: Signal<HashSet<String>>,
}

/// `src` holding `kit` (two files) and `main.rs`, a `docs` that holds a thousand pages, and
/// a file beside them.
fn page() -> Page {
    let (chosen, open) = (signal(None), signal(HashSet::new()));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_kit::keymap()));
    h.mount(move || {
        let roots = || {
            let file = |name: &str| TreeNode::new(name, name).icon(icons::FILE);
            let kit = TreeNode::new("kit", "kit").children([file("button.rs"), file("menu.rs")]);
            let pages = (0..1000).map(|i| TreeNode::new(format!("page-{i}"), format!("Page {i}")));
            vec![
                TreeNode::new("src", "src").icon(icons::FOLDER).children([kit, file("main.rs")]),
                TreeNode::new("docs", "docs").children(pages),
                file("README.md"),
            ]
        };
        quadra_kit::root().child(tree(roots, chosen).open(open))
    });
    h.frame();
    Page { h, chosen, open }
}

impl Page {
    fn settle(&mut self) {
        self.h.frame();
        while self.h.frame_if_needed().is_some() {}
    }

    /// The rows that show, the highest first: each indented by its depth, and marked when
    /// it is open (`v`), closed (`>`) or selected (`*`).
    fn rows(&mut self) -> Vec<String> {
        let says = self.h.tree.semantic_tree();
        let mut shown: Vec<(f32, String)> = Vec::new();
        for node in says.nodes.iter().filter(|node| node.semantics.role == Role::TreeItem) {
            let id = self.h.tree.node_of(node.id).unwrap();
            let rect = self.h.tree.window_rect(id);
            let depth = ((self.h.tree.layout_style(id).padding.left - 8.0) / 20.0) as usize;
            let mark = match node.semantics.expanded {
                Some(true) => "v ",
                Some(false) => "> ",
                None => "",
            };
            let star = if node.semantics.selected { "*" } else { "" };
            shown.push((rect.y, format!("{}{mark}{}{star}", "  ".repeat(depth), node.semantics.name)));
        }
        shown.sort_by(|a, b| a.0.total_cmp(&b.0));
        shown.into_iter().map(|(_, row)| row).collect()
    }

    fn key(&mut self, key: Key) -> Vec<String> {
        self.h.key(key, Mods::NONE);
        self.settle();
        self.rows()
    }

    fn click(&mut self, name: &str) -> Vec<String> {
        self.h.get_by_role(Role::TreeItem, name).click();
        self.settle();
        self.rows()
    }
}

#[test]
fn rows_open_and_close_what_they_hold_by_the_pointer() {
    let mut p = page();
    assert_eq!(p.rows(), ["> src", "> docs", "README.md"]);
    let src = p.h.get_by_role(Role::TreeItem, "src").node();
    assert_eq!(p.h.tree.rect(src).height, 26.0);
    // A click selects; a second one on what holds something opens it, and another closes it.
    assert_eq!(p.click("src"), ["> src*", "> docs", "README.md"]);
    assert_eq!(p.click("src"), ["v src*", "  > kit", "  main.rs", "> docs", "README.md"]);
    assert_eq!(p.click("kit"), ["v src", "  > kit*", "  main.rs", "> docs", "README.md"]);
    let rows = p.click("kit");
    assert_eq!(rows[..4], ["v src", "  v kit*", "    button.rs", "    menu.rs"]);
    assert_eq!((p.chosen.get_untracked().as_deref(), p.open.get_untracked().len()), (Some("kit"), 2));
    // A row two levels in has a guide for each level, before its chevron's place.
    let file = p.h.get_by_role(Role::TreeItem, "button.rs").node();
    assert_eq!(p.h.tree.children(file).len(), 2 + 3);
    assert_eq!(p.click("src")[1], "  v kit");
    assert_eq!(p.click("src"), ["> src*", "> docs", "README.md"]);

    // Of a thousand pages, the rows of the window are mounted.
    p.open.update(|open| _ = open.insert("docs".to_string()));
    p.settle();
    let rows = p.rows();
    assert!(rows.len() < 40 && rows[2] == "  Page 0", "{rows:?}");
}

#[test]
fn the_arrows_move_open_and_close() {
    let mut p = page();
    let frame = p.h.query(|n| n.semantics.role == Role::Tree)[0];
    p.h.tree.focus(frame);
    assert_eq!(p.key(Key::Down), ["> src*", "> docs", "README.md"]);
    // Right opens what is closed, then goes into it; left closes, then goes out.
    assert_eq!(p.key(Key::Right)[..3], ["v src*", "  > kit", "  main.rs"]);
    assert_eq!(p.key(Key::Right)[..2], ["v src", "  > kit*"]);
    assert_eq!(p.key(Key::Enter)[..3], ["v src", "  v kit*", "    button.rs"]);
    p.key(Key::Down);
    assert_eq!(p.key(Key::Left)[1], "  v kit*"); // from a file to what holds it
    assert_eq!(p.key(Key::Left)[..3], ["v src", "  > kit*", "  main.rs"]);
    assert_eq!(p.key(Key::Left)[0], "v src*");
    assert_eq!(p.key(Key::Left), ["> src*", "> docs", "README.md"]);
    assert_eq!(p.key(Key::Left), ["> src*", "> docs", "README.md"]); // nothing to go out to
    // End goes to the last row and Home to the first; the ends stop the arrows.
    assert_eq!(p.key(Key::End)[2], "README.md*");
    assert_eq!(p.key(Key::Down)[2], "README.md*");
    p.key(Key::Up);
    p.key(Key::Right);
    let rows = p.key(Key::End);
    assert!(rows.last().unwrap() == "README.md*" && rows.iter().any(|row| row == "  Page 999"), "{rows:?}");
    assert_eq!(p.key(Key::Home)[0], "> src*");
}
