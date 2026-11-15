//! What opens and closes (specs/17, "The components"): the sections of an accordion. The
//! head is a button, expanded or collapsed to a reader; a click, Enter or Space turns the
//! signal that says whether its section is open. The look gives the head's face and the body.

use quadra_reactive::Signal;
use quadra_tree::Role;
use quadra_view::{Div, Region, View, show};

use crate::action::{CONTROL, Confirm};

/// The head of a section on its face: `State::EXPANDED` while `open` is true.
pub fn head(face: Div, open: Signal<bool>) -> Div {
    let toggle = move || open.update(|open| *open = !*open);
    let face = face.role(Role::Button).expanded(move || open.get()).focusable().key_context(CONTROL);
    face.on_action::<Confirm>(move |_| toggle()).on_click(move |_| toggle())
}

/// The body of a section: built by `body` each time it opens, gone while it is closed.
pub fn body<V: View + 'static>(open: Signal<bool>, body: impl Fn() -> V + 'static) -> Region {
    show(move || open.get(), body)
}
