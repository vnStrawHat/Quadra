//! Actions and the keymap (specs/09): a keystroke, or a sequence of them, in a context, is an
//! action; the action goes from the focused node up to the first node that handles it; a key
//! that is no handled action goes on as a key.

mod common;

use std::cell::RefCell;
use std::rc::Rc;

use common::Window;
use quadra_platform::{Key, Mods};
use quadra_tree::{Action, Keymap, Keystroke, NodeId};

#[derive(Debug, PartialEq)]
struct Save;
impl Action for Save {
    const NAME: &'static str = "file::save";
}

#[derive(Debug, PartialEq)]
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

struct Page {
    w: Window,
    log: Log,
    editor: NodeId,
    line: NodeId,
    side: NodeId,
}

/// An editor (a context of that name) with a line in it, and a side panel: three nodes that
/// can have the focus, under the root.
fn page(keymap: Keymap) -> Page {
    let mut w = Window::new();
    let (line, side) = (w.block(100.0, 20.0, 0x202020), w.block(100.0, 20.0, 0x303030));
    let editor = w.column(&[line]);
    for node in [editor, line, side] {
        w.tree.set_focusable(node, true);
    }
    w.tree.set_key_context(editor, "editor");
    let root = w.column(&[editor, side]);
    w.tree.set_root(root);
    w.tree.set_keymap(Rc::new(keymap));
    w.frame();
    Page { w, log: Log::default(), editor, line, side }
}

impl Page {
    fn handle<A: Action + std::fmt::Debug>(&mut self, node: NodeId, name: &'static str) {
        let log = self.log.clone();
        self.w.tree.on_action::<A>(node, move |action| log.borrow_mut().push(format!("{name}: {action:?}")));
    }

    fn press(&mut self, key: Key, mods: Mods) {
        self.w.tree.key(key, 0, mods, true, false);
        self.w.tree.key(key, 0, mods, false, false);
    }

    fn take(&mut self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
}

#[test]
fn keystrokes_are_parsed_and_wrong_ones_are_reported() {
    let stroke = |key, mods| Ok(Keystroke { key, mods });
    assert_eq!(Keystroke::parse("ctrl-s"), stroke(Key::S, Mods::CTRL));
    assert_eq!(Keystroke::parse("Ctrl-Shift-P"), stroke(Key::P, Mods::CTRL | Mods::SHIFT));
    assert_eq!(Keystroke::parse("f5"), stroke(Key::F5, Mods::NONE));
    assert_eq!(Keystroke::parse("alt-enter"), stroke(Key::Enter, Mods::ALT));
    assert_eq!(Keystroke::parse("ctrl-,"), stroke(Key::Comma, Mods::CTRL));
    assert_eq!(Keystroke::parse("ctrl-minus"), stroke(Key::Minus, Mods::CTRL));
    assert_eq!(Keystroke::parse("f"), stroke(Key::F, Mods::NONE));
    for wrong in ["", "ctrl-", "hyper-x", "ctrl-f13", "save"] {
        assert!(Keystroke::parse(wrong).is_err(), "{wrong:?}");
    }
    let mut keymap = Keymap::new();
    assert!(keymap.bind("ctrl-k ctrl-?", Comment).is_err() && keymap.bind("  ", Save).is_err());
    keymap.bind("ctrl-k ctrl-c", Comment).unwrap();
    assert_eq!(keymap.keys_of::<Comment>().map(<[_]>::len), Some(2));
    assert!(keymap.keys_of::<Save>().is_none());
}

#[test]
fn an_action_goes_up_from_the_focus_to_the_first_node_that_handles_it() {
    let mut keymap = Keymap::new();
    keymap.bind("ctrl-s", Save).unwrap();
    keymap.bind("ctrl-3", SelectTab(3)).unwrap();
    let mut p = page(keymap);
    let root = p.w.tree.root().unwrap();
    p.handle::<Save>(root, "root");
    p.handle::<Save>(p.editor, "editor");
    p.handle::<SelectTab>(root, "root");
    let log = p.log.clone();
    p.w.tree.on_key(root, true, move |e| log.borrow_mut().push(format!("key {:?}", e.key)));

    p.w.tree.focus(p.line);
    p.press(Key::S, Mods::CTRL);
    assert_eq!(p.take(), ["editor: Save"]); // the nearest handler, and the key is not a key
    p.w.tree.focus(p.side);
    p.press(Key::S, Mods::CTRL);
    assert_eq!(p.take(), ["root: Save"]);
    p.press(Key::Digit3, Mods::CTRL);
    assert_eq!(p.take(), ["root: SelectTab(3)"]); // an action carries its data
    p.press(Key::S, Mods::NONE);
    assert_eq!(p.take(), ["key S"]); // no binding: a key

    // A button or a menu dispatches the same action without a key.
    assert!(p.w.tree.dispatch(&Save) && !p.w.tree.dispatch(&Comment));
    assert_eq!(p.take(), ["root: Save"]);
}

#[test]
fn an_action_nobody_handles_leaves_the_key_a_key() {
    let mut keymap = Keymap::new();
    keymap.bind("ctrl-s", Save).unwrap();
    let mut p = page(keymap);
    let log = p.log.clone();
    p.w.tree.on_key(p.side, true, move |e| log.borrow_mut().push(format!("key {:?}", e.key)));
    p.w.tree.focus(p.side);
    p.press(Key::S, Mods::CTRL);
    assert_eq!(p.take(), ["key S"]);
}

#[test]
fn the_binding_of_the_context_nearest_the_focus_wins_and_later_bindings_win() {
    let mut keymap = Keymap::new();
    keymap.bind("ctrl-/", Save).unwrap();
    keymap.bind_in("editor", "ctrl-/", Comment).unwrap();
    keymap.bind("f2", SelectTab(1)).unwrap();
    keymap.bind("f2", SelectTab(2)).unwrap(); // as a user's file after the defaults
    let mut p = page(keymap);
    let root = p.w.tree.root().unwrap();
    p.handle::<Save>(root, "root");
    p.handle::<Comment>(root, "root");
    p.handle::<SelectTab>(root, "root");

    p.w.tree.focus(p.line); // inside the editor
    p.press(Key::Slash, Mods::CTRL);
    assert_eq!(p.take(), ["root: Comment"]);
    p.w.tree.focus(p.side); // outside it: the binding for everywhere
    p.press(Key::Slash, Mods::CTRL);
    assert_eq!(p.take(), ["root: Save"]);
    p.press(Key::F2, Mods::NONE);
    assert_eq!(p.take(), ["root: SelectTab(2)"]);
}

#[test]
fn a_sequence_of_keys_is_one_binding() {
    let mut keymap = Keymap::new();
    keymap.bind("ctrl-k ctrl-c", Comment).unwrap();
    keymap.bind("ctrl-k s", Save).unwrap();
    let mut p = page(keymap);
    let root = p.w.tree.root().unwrap();
    p.handle::<Comment>(root, "root");
    p.handle::<Save>(root, "root");
    let log = p.log.clone();
    p.w.tree.on_key(root, true, move |e| log.borrow_mut().push(format!("key {:?}", e.key)));

    p.press(Key::K, Mods::CTRL);
    assert!(p.take().is_empty()); // taken: the next key decides
    p.press(Key::Ctrl, Mods::CTRL); // a modifier on its way to a keystroke is not one
    p.take();
    p.press(Key::C, Mods::CTRL);
    assert_eq!(p.take(), ["root: Comment"]);
    p.press(Key::K, Mods::CTRL);
    p.press(Key::S, Mods::NONE);
    assert_eq!(p.take(), ["root: Save"]);
    // A sequence that leads nowhere is dropped, and its last key is an ordinary key.
    p.press(Key::K, Mods::CTRL);
    p.press(Key::X, Mods::NONE);
    assert_eq!(p.take(), ["key X"]);
    p.press(Key::C, Mods::CTRL);
    assert_eq!(p.take(), ["key C"]); // nothing was pending any more
}
