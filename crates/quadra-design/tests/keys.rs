//! The key input (specs/13, `key_input`): focused, it records the next keystroke, and the
//! app's bindings stay quiet while it does; Escape gives up, Backspace unbinds, Tab moves on.

use std::rc::Rc;

use quadra_design::{dark, key_input};
use quadra_platform::{Key, Mods};
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::{Action, Role};
use quadra_view::column;

struct Hit;
impl Action for Hit {
    const NAME: &'static str = "test::hit";
}

#[test]
fn a_key_input_records_a_keystroke_and_the_keymap_waits() {
    let (value, other, hits) = (signal("ctrl-t".to_string()), signal(String::new()), signal(0));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    let mut keys = quadra_design::keymap();
    keys.bind("ctrl-k", Hit).unwrap();
    h.tree.set_keymap(Rc::new(keys));
    h.mount(move || {
        let both = column().child(key_input(value).label("New tab")).child(key_input(other).label("Split"));
        quadra_design::root().on_action::<Hit>(move |_| hits.update(|n| *n += 1)).child(both)
    });
    h.frame();
    assert!(h.semantics().contains("Button \"New tab\""), "{}", h.semantics());
    assert!(h.has_text("ctrl-t"));

    // Focused, it records: Ctrl+K is the value, and the binding of Ctrl+K does not run.
    h.get_by_role(Role::Button, "New tab").focus();
    h.key(Key::Shift, Mods::SHIFT); // a modifier alone is not a keystroke
    h.key(Key::letter(10), Mods::CTRL);
    h.frame();
    assert_eq!(value.get_untracked(), "ctrl-k");
    assert_eq!(hits.get_untracked(), 0);
    // Recorded, it records no more: the binding runs.
    h.key(Key::letter(10), Mods::CTRL);
    assert_eq!((value.get_untracked().as_str(), hits.get_untracked()), ("ctrl-k", 1));

    // Enter records again; Escape gives up and keeps the value.
    h.key(Key::Enter, Mods::NONE);
    h.key(Key::Escape, Mods::NONE);
    h.key(Key::letter(10), Mods::CTRL);
    assert_eq!((value.get_untracked().as_str(), hits.get_untracked()), ("ctrl-k", 2));

    // A click records; Backspace alone unbinds.
    h.get_by_role(Role::Button, "New tab").click();
    h.key(Key::Backspace, Mods::NONE);
    h.frame();
    assert_eq!(value.get_untracked(), "");

    // While it records, Tab still moves the focus, and the next field records.
    h.key(Key::Enter, Mods::NONE);
    h.key(Key::Tab, Mods::NONE);
    h.key(Key::function(4), Mods::ALT | Mods::SHIFT);
    h.frame();
    assert_eq!((value.get_untracked().as_str(), other.get_untracked().as_str()), ("", "alt-shift-f5"));
}
