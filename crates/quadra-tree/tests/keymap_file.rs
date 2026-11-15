//! Contexts as predicates and the keymap file (specs/09): `editor && !find` holds in an
//! editor but not in its find bar; a file's bindings name actions and win over the defaults;
//! a wrong line is reported with its number and none of the file is taken.

mod common;

use std::cell::RefCell;
use std::rc::Rc;

use common::Window;
use quadra_platform::{Key, Mods};
use quadra_tree::{Action, Actions, Keymap, NodeId};

#[derive(Clone, Debug, PartialEq)]
struct Save;
impl Action for Save {
    const NAME: &'static str = "file::save";
}

#[derive(Clone, Debug, PartialEq)]
struct Comment;
impl Action for Comment {
    const NAME: &'static str = "editor::comment";
}

#[derive(Debug, PartialEq)]
struct SelectTab(usize);
impl Action for SelectTab {
    const NAME: &'static str = "tabs::select";
}

type Log = Rc<RefCell<Vec<String>>>;

/// An editor with its find bar in it, and a side panel; what the root is sent is logged.
fn page(keymap: Keymap) -> (Window, Log, [NodeId; 3]) {
    let mut w = Window::new();
    let (find, side) = (w.block(100.0, 20.0, 0x202020), w.block(100.0, 20.0, 0x303030));
    let editor = w.column(&[find]);
    for (node, name) in [(editor, "editor"), (find, "find"), (side, "side")] {
        w.tree.set_focusable(node, true);
        w.tree.set_key_context(node, name);
    }
    let root = w.column(&[editor, side]);
    w.tree.set_root(root);
    w.tree.set_keymap(Rc::new(keymap));
    let log = Log::default();
    let to = log.clone();
    w.tree.on_action::<Save>(root, move |_| to.borrow_mut().push("save".into()));
    let to = log.clone();
    w.tree.on_action::<Comment>(root, move |_| to.borrow_mut().push("comment".into()));
    let to = log.clone();
    w.tree.on_action::<SelectTab>(root, move |tab| to.borrow_mut().push(format!("tab {}", tab.0)));
    w.frame();
    (w, log, [editor, find, side])
}

fn press(w: &mut Window, log: &Log, key: Key, mods: Mods) -> Vec<String> {
    w.tree.key(key, 0, mods, true, false);
    w.tree.key(key, 0, mods, false, false);
    std::mem::take(&mut *log.borrow_mut())
}

#[test]
fn a_context_is_a_predicate_over_the_names_around_the_focus() {
    let mut keymap = Keymap::new();
    keymap.bind("ctrl-/", Save).unwrap();
    keymap.bind_in("editor && !find", "ctrl-/", Comment).unwrap();
    keymap.bind_in("find || side", "f3", SelectTab(3)).unwrap();
    for wrong in ["", "editor &&", "!", "an editor", "a | b"] {
        assert!(keymap.bind_in(wrong, "f4", Save).is_err(), "{wrong:?}");
    }
    let (mut w, log, [editor, find, side]) = page(keymap);
    w.tree.focus(editor);
    assert_eq!(press(&mut w, &log, Key::Slash, Mods::CTRL), ["comment"]);
    assert!(press(&mut w, &log, Key::F3, Mods::NONE).is_empty());
    w.tree.focus(find); // in the editor, but in its find bar
    assert_eq!(press(&mut w, &log, Key::Slash, Mods::CTRL), ["save"]);
    assert_eq!(press(&mut w, &log, Key::F3, Mods::NONE), ["tab 3"]);
    w.tree.focus(side);
    assert_eq!(press(&mut w, &log, Key::F3, Mods::NONE), ["tab 3"]);
}

#[test]
fn a_keymap_file_binds_named_actions_over_the_defaults() {
    let mut actions = Actions::new();
    actions.add(Save).add(Comment).add_with(|rest: &str| rest.parse().ok().map(SelectTab));
    let mut keymap = Keymap::new();
    keymap.bind("ctrl-s", Comment).unwrap(); // a default the file replaces
    let file = "# The user's.\nctrl-s = file::save\n\n[editor && !find]\nctrl-k ctrl-c = editor::comment\nctrl-1 = tabs::select 1\n";
    keymap.load(file, &actions).unwrap();
    assert_eq!(keymap.keys_of::<Comment>().map(<[_]>::len), Some(2));

    // Each wrong line is reported by its number, and none of the file is taken.
    let wrong = [
        ("f5 = file::save\nctrl-? = file::save", "line 2"),
        ("[editor &&]", "line 1"),
        ("f5 = file::open", "no action"),
        ("f5 = file::save now", "does not take"),
        ("f5 = tabs::select one", "does not take"),
        ("f5 file::save", "line 1"),
    ];
    for (text, says) in wrong {
        let error = keymap.load(text, &actions).unwrap_err();
        assert!(error.0.contains(says), "{text:?}: {error}");
    }

    let (mut w, log, [editor, find, _]) = page(keymap);
    w.tree.focus(editor);
    assert_eq!(press(&mut w, &log, Key::S, Mods::CTRL), ["save"]);
    assert_eq!(press(&mut w, &log, Key::Digit1, Mods::CTRL), ["tab 1"]);
    press(&mut w, &log, Key::K, Mods::CTRL);
    assert_eq!(press(&mut w, &log, Key::C, Mods::CTRL), ["comment"]);
    w.tree.focus(find);
    assert!(press(&mut w, &log, Key::Digit1, Mods::CTRL).is_empty());
    assert!(press(&mut w, &log, Key::F5, Mods::NONE).is_empty()); // the wrong files left nothing
}
