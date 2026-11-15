//! The key input (specs/17, "The components"): a field that records a keystroke. Focused, it
//! records, and again on a click or `Confirm`; the next press that is no modifier, with its
//! modifiers, is the value as `Keystroke` prints it (`ctrl-shift-t`), and recording ends.
//! Escape gives up and keeps the value; Backspace or Delete alone empties it (unbound); Tab
//! and Shift+Tab still move the focus. It sees presses before the keymap, so no binding of
//! the app runs while it records. The look gives the face.

use quadra_platform::{Key, Mods};
use quadra_reactive::Signal;
use quadra_tree::{KeyEvent, Keystroke, Role};
use quadra_view::{Div, NodeRef};

use crate::action::{CONTROL, Confirm};

/// The face the look built, made a field that records a keystroke into `value` while
/// `recording` says so: a button to a reader, its value the keys.
pub fn key_input(face: Div, value: Signal<String>, recording: Signal<bool>) -> Div {
    let me = NodeRef::new();
    let face = face.role(Role::Button).value(move || value.get()).focusable().key_context(CONTROL);
    let face = face.node_ref(&me).on_focus(move |on| recording.set(on));
    // A click on the field that has the focus already brings no focus event.
    let face = face.on_click(move |_| {
        me.focus();
        recording.set(true);
    });
    let face = face.on_action::<Confirm>(move |_| recording.set(true));
    face.on_mount(move |tree, id| tree.on_key_first(id, move |event| record(event, value, recording)))
}

/// What a press does while the field records: the keystroke is the value, or it gives up.
fn record(event: &mut KeyEvent, value: Signal<String>, recording: Signal<bool>) {
    let alone = event.mods == Mods::NONE;
    if !recording.get_untracked() || (event.key == Key::Tab && (alone || event.mods == Mods::SHIFT)) {
        return; // the keymap's: Confirm, or the focus moved
    }
    event.stop();
    let stroke = Keystroke { key: event.key, mods: event.mods };
    match event.key {
        _ if event.repeat => return,
        Key::Escape if alone => {}
        Key::Backspace | Key::Delete if alone => value.set(String::new()),
        _ if stroke.is_bindable() => value.set(stroke.to_string()),
        _ => return, // a key with no name: still recording
    }
    recording.set(false);
}
