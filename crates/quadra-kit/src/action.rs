//! What the keyboard asks of the kit's components (specs/13, "Components"): actions, bound in
//! the kit's keymap, never raw keys.

use quadra_tree::{Action, Keymap};

/// The key context of a control that is activated: a button, a checkbox, a tab.
pub const CONTROL: &str = "control";

/// Do what the control is for: Enter or Space on a button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Confirm;

impl Action for Confirm {
    const NAME: &'static str = "kit::confirm";
}

/// The key bindings of the kit over those every window has. An app adds its own to it.
pub fn keymap() -> Keymap {
    let mut keymap = Keymap::default();
    for keys in ["enter", "space"] {
        keymap.bind_in(CONTROL, keys, Confirm).expect("the kit's bindings are well formed");
    }
    keymap
}
