//! Keys to the program on a Mac (specs/18, "Input"): Control is the program's, Command the
//! app's, and Option with a key that types is left to the layout.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_platform::{Key, KeyNames, Mods};
use quadra_reactive::provide;
use quadra_term::vt::ResizePolicy;
use quadra_term::{Link, Session, bind_keys, terminal};
use quadra_testing::Harness;
use quadra_tree::{Keymap, Role};
use quadra_view::column;

#[test]
fn on_a_mac_control_writes_to_the_program_and_command_does_not() {
    let mut h = Harness::new();
    let mut keys = Keymap::default();
    bind_keys(&mut keys);
    h.tree.set_keymap(Rc::new(keys.as_a_mac_has_them()));
    let wrote = Rc::new(RefCell::new(Vec::new()));
    let into = wrote.clone();
    h.mount(move || {
        provide(KeyNames::Signs);
        let s = Session::new(ResizePolicy::BottomAnchor);
        s.connect(Link::new(move |bytes| into.borrow_mut().extend_from_slice(bytes)));
        column().size(400.0, 140.0).child(terminal(&s).font(10.0, 14.0))
    });
    h.settle();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    h.tree.focus(id);
    h.settle();
    let mut key = |key, mods| {
        h.key(key, mods);
        h.settle();
        std::mem::take(&mut *wrote.borrow_mut())
    };
    assert_eq!(key(Key::C, Mods::SUPER), b"\x03", "Control+C interrupts");
    assert_eq!(key(Key::C, Mods::CTRL), b"", "Command+C copies, and the program is told nothing");
    assert_eq!(key(Key::K, Mods::CTRL), b"", "a key with Command is never the program's");
    assert_eq!(key(Key::A, Mods::ALT), b"", "Option+A is for the layout to type");
    assert_eq!(key(Key::Left, Mods::NONE), b"\x1b[D");
}
