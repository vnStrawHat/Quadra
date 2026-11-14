//! One of a few (specs/17, "The components"): a radio group, a segmented control, tabs. The
//! group is one stop of Tab and the arrows move the choice round the ends; an option is
//! chosen by a click. The look gives the group's box and each option's face.

use quadra_reactive::Signal;
use quadra_tree::Role;
use quadra_ui::Div;

use crate::action::{CHOICE, MoveNext, MovePrevious};

/// The box around `count` options of which `chosen` holds the one chosen: a group of the
/// `role` to a reader (`Role::Group` for radios, `Role::TabList` for tabs).
pub fn group(group: Div, role: Role, count: usize, chosen: Signal<usize>) -> Div {
    let step = move |by: usize| chosen.update(|at| *at = (*at + by) % count.max(1));
    let group = group.role(role).set_size(count).focusable().key_context(CHOICE);
    let group = group.on_action::<MoveNext>(move |_| step(1));
    group.on_action::<MovePrevious>(move |_| step(count.saturating_sub(1)))
}

/// The option at `i` on its `face`: a radio, or with `Role::Tab` a tab. A radio is checked
/// while it is the one chosen, a tab selected: the states the look follows.
pub fn option(face: Div, role: Role, i: usize, chosen: Signal<usize>) -> Div {
    let on = move || chosen.get() == i;
    let face = face.role(role).place_in_set(i).on_click(move |_| chosen.set(i));
    match role {
        Role::Tab => face.selected(on),
        _ => face.checked(on),
    }
}
