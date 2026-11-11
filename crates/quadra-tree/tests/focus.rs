//! Focus and keys (specs/09): a press focuses what can have the focus, Tab walks the tree
//! inside a focus scope, keys go down the path to the focused node and back up, and a focus
//! whose node is removed goes to the nearest ancestor that can have it.

mod common;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use common::Window;
use quadra_layout::Display;
use quadra_platform::{Key, Mods, MouseButton};
use quadra_prim::Point;
use quadra_tree::{NodeId, PointerInput, State};

type Log = Rc<RefCell<Vec<String>>>;

struct Page {
    w: Window,
    log: Log,
    form: NodeId,
    name: NodeId,
    label: NodeId,
    email: NodeId,
    dialog: NodeId,
    ok: NodeId,
    cancel: NodeId,
}

/// A form of three 100 x 20 rows (two fields that can have the focus around a label that
/// cannot) at the top, then a dialog with two buttons. The form itself can have the focus.
fn page() -> Page {
    let mut w = Window::new();
    let [name, label, email, ok, cancel] = [0, 1, 2, 3, 4].map(|i| w.block(100.0, 20.0, 0x202020 + i));
    let form = w.column(&[name, label, email]);
    let dialog = w.column(&[ok, cancel]);
    for node in [form, name, email, ok, cancel] {
        w.tree.set_focusable(node, true);
    }
    let root = w.column(&[form, dialog]);
    w.tree.set_root(root);
    w.frame();
    Page { w, log: Log::default(), form, name, label, email, dialog, ok, cancel }
}

impl Page {
    fn click(&mut self, x: f32, y: f32) {
        for input in [PointerInput::Down, PointerInput::Up] {
            self.w.tree.pointer(input, MouseButton::Left, Point::new(x, y), Mods::NONE, Instant::now());
        }
    }

    fn press(&mut self, key: Key, mods: Mods) {
        self.w.tree.key(key, 0, mods, true, false);
        self.w.tree.key(key, 0, mods, false, false);
    }

    fn has(&self, node: NodeId, state: State) -> bool {
        self.w.tree.state(node).contains(state)
    }

    fn watch_focus(&mut self, node: NodeId, name: &'static str) {
        let log = self.log.clone();
        let say = move |gained: bool| {
            log.borrow_mut().push(format!("{name} {}", if gained { "focus" } else { "blur" }))
        };
        self.w.tree.on_focus(node, say);
    }

    fn take(&mut self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
}

#[test]
fn a_press_focuses_the_deepest_node_that_can_have_it() {
    let mut p = page();
    p.watch_focus(p.name, "name");
    p.watch_focus(p.form, "form");
    p.click(10.0, 10.0);
    assert_eq!((p.w.tree.focused(), p.take()), (Some(p.name), vec!["name focus".to_string()]));
    assert!(p.has(p.name, State::FOCUS) && !p.has(p.name, State::FOCUS_VISIBLE)); // no ring for a click
    assert!(p.has(p.form, State::FOCUS_WITHIN) && p.has(p.w.tree.root().unwrap(), State::FOCUS_WITHIN));
    assert!(!p.has(p.form, State::FOCUS));

    p.click(10.0, 30.0); // the label cannot have it: the form around it does
    assert_eq!(
        (p.w.tree.focused(), p.take()),
        (Some(p.form), vec!["name blur".to_string(), "form focus".to_string()])
    );
    assert!(!p.has(p.name, State::FOCUS) && p.has(p.form, State::FOCUS_WITHIN));
    p.click(500.0, 500.0); // nothing there can: the focus is taken away
    assert_eq!((p.w.tree.focused(), p.take()), (None, vec!["form blur".to_string()]));
    assert!(!p.has(p.w.tree.root().unwrap(), State::FOCUS_WITHIN));

    p.w.tree.focus(p.label); // a program asks for a node that cannot have it: nothing
    assert_eq!(p.w.tree.focused(), None);
    p.w.tree.focus(p.email);
    assert!(p.has(p.email, State::FOCUS) && !p.has(p.email, State::FOCUS_VISIBLE));
}

#[test]
fn tab_walks_the_tree_and_comes_round_and_shows_the_ring() {
    let mut p = page();
    let order = [p.form, p.name, p.email, p.ok, p.cancel];
    for expected in order {
        p.press(Key::Tab, Mods::NONE);
        assert_eq!(p.w.tree.focused(), Some(expected));
        assert!(p.has(expected, State::FOCUS_VISIBLE));
    }
    p.press(Key::Tab, Mods::NONE);
    assert_eq!(p.w.tree.focused(), Some(p.form)); // after the last, the first
    p.press(Key::Tab, Mods::SHIFT);
    assert_eq!(p.w.tree.focused(), Some(p.cancel));
    assert!(!p.has(p.form, State::FOCUS_VISIBLE));

    // What is not displayed is passed over, with what is under it.
    let hidden = quadra_layout::Style { display: Display::None, ..p.w.tree.layout_style(p.dialog).clone() };
    p.w.tree.set_layout_style(p.dialog, hidden);
    p.w.tree.focus(p.email);
    p.press(Key::Tab, Mods::NONE);
    assert_eq!(p.w.tree.focused(), Some(p.form));
    // A handler that stops Tab keeps the focus where it is.
    p.w.tree.on_key(p.form, true, |e| e.stop());
    p.press(Key::Tab, Mods::NONE);
    assert_eq!(p.w.tree.focused(), Some(p.form));
}

#[test]
fn a_focus_scope_keeps_tab_inside_it() {
    let mut p = page();
    p.w.tree.set_focus_scope(p.dialog, true);
    p.w.tree.focus(p.ok);
    let mut seen = Vec::new();
    for _ in 0..3 {
        p.press(Key::Tab, Mods::NONE);
        seen.push(p.w.tree.focused().unwrap());
    }
    assert_eq!(seen, [p.cancel, p.ok, p.cancel]);
    p.press(Key::Tab, Mods::SHIFT);
    assert_eq!(p.w.tree.focused(), Some(p.ok));
}

#[test]
fn keys_go_down_the_path_to_the_focus_and_back_up() {
    let mut p = page();
    let root = p.w.tree.root().unwrap();
    for (node, name) in [(root, "root"), (p.form, "form"), (p.name, "name")] {
        let log = p.log.clone();
        p.w.tree.on_key(node, true, move |e| log.borrow_mut().push(format!("{name} {:?}", e.key)));
        let log = p.log.clone();
        p.w.tree.on_key_capture(node, true, move |_| log.borrow_mut().push(format!("{name} first")));
    }
    let log = p.log.clone();
    p.w.tree.on_key(p.name, false, move |e| log.borrow_mut().push(format!("name up {:?}", e.key)));

    p.press(Key::A, Mods::NONE); // nothing has the focus: the root is the target
    assert_eq!(p.take(), ["root first", "root A"]);
    p.w.tree.focus(p.name);
    p.press(Key::Enter, Mods::NONE);
    assert_eq!(
        p.take(),
        ["root first", "form first", "name first", "name Enter", "form Enter", "root Enter", "name up Enter"]
    );
    p.w.tree.on_key(p.form, true, |e| e.stop());
    p.press(Key::B, Mods::NONE);
    assert_eq!(p.take().len(), 6); // three down, name, form, and no further; then the release
}

#[test]
fn a_focus_whose_node_is_removed_goes_to_an_ancestor_that_can_have_it() {
    let mut p = page();
    p.watch_focus(p.form, "form");
    p.w.tree.focus(p.name);
    p.w.tree.remove(p.name);
    assert_eq!((p.w.tree.focused(), p.take()), (Some(p.form), vec!["form focus".to_string()]));
    p.w.frame();
    p.w.tree.focus(p.ok);
    p.w.tree.remove(p.dialog); // an ancestor of the focus goes: nothing above can have it
    assert_eq!(p.w.tree.focused(), None);
    p.w.frame();
    p.press(Key::Tab, Mods::NONE);
    assert_eq!(p.w.tree.focused(), Some(p.form));
}

#[test]
fn a_window_without_the_keyboard_shows_no_focus_and_gets_it_back() {
    let mut p = page();
    p.press(Key::Tab, Mods::NONE);
    p.press(Key::Tab, Mods::NONE);
    assert!(p.has(p.name, State::FOCUS | State::FOCUS_VISIBLE));
    p.w.tree.set_active(false);
    assert!(!p.has(p.name, State::FOCUS) && !p.has(p.form, State::FOCUS_WITHIN));
    assert_eq!(p.w.tree.focused(), Some(p.name)); // it is still the node keys would go to
    p.w.tree.set_active(true);
    assert!(p.has(p.name, State::FOCUS | State::FOCUS_VISIBLE) && p.has(p.form, State::FOCUS_WITHIN));
}
