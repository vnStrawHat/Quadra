//! What is pressed (specs/17, "The components"): a button and an on or off control. A click
//! and `Confirm` (Enter, Space) do the same thing; a disabled control is not clicked, focused
//! or tabbed to. The look gives the face.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_reactive::{Memo, Signal};
use quadra_tree::{NodeId, Role};
use quadra_view::{Div, IntoProp, MountCx, Prop, View};

use crate::action::{CONTROL, Confirm};

/// A button on its face: what it does, when it does nothing, and whether it is on.
pub struct Press {
    face: Div,
    disabled: Prop<bool>,
    busy: Option<Memo<bool>>,
    toggled: Option<Prop<bool>>,
    click: Option<Box<dyn FnMut()>>,
}

/// The face the look built, made a button: a button to a reader, named by its label (or by
/// `.label(..)` on the face).
pub fn press(face: Div) -> Press {
    Press { face, disabled: Prop::Value(false), busy: None, toggled: None, click: None }
}

impl Press {
    pub fn disabled<M>(mut self, disabled: impl IntoProp<bool, M>) -> Self {
        self.disabled = disabled.into_prop();
        self
    }

    /// While `busy` is true a click does nothing: what it started is under way.
    pub fn busy(mut self, busy: Memo<bool>) -> Self {
        self.busy = Some(busy);
        self
    }

    /// A button that is on or off, which its owner says: pressed or not to a reader, and
    /// `State::CHECKED` while on.
    pub fn toggled<M>(mut self, on: impl IntoProp<bool, M>) -> Self {
        self.toggled = Some(on.into_prop());
        self
    }

    /// What the button does: on a click, and on Enter or Space while it has the focus.
    pub fn on_click(mut self, f: impl FnMut() + 'static) -> Self {
        self.click = Some(Box::new(f));
        self
    }

    pub fn build(self) -> Div {
        let face = self.face.role(Role::Button).focusable().key_context(CONTROL).disabled(self.disabled);
        let face = match self.toggled {
            Some(on) => face.checked(on),
            None => face,
        };
        let Some(click) = self.click else { return face };
        let (busy, click) = (self.busy, Rc::new(RefCell::new(click)));
        let by_key = click.clone();
        let press = move |click: &RefCell<Box<dyn FnMut()>>| {
            if !busy.is_some_and(|busy| busy.get()) {
                (click.borrow_mut())()
            }
        };
        // A press it takes is its own: what it stands in (a row) is not clicked by it.
        let face = face.on_click(move |e| {
            press(&click);
            e.stop();
        });
        face.on_action::<Confirm>(move |_| press(&by_key))
    }
}

impl View for Press {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        self.build().mount(cx)
    }
}

/// A control that is on or off on its face: a checkbox, or with `switch` a switch. A click on
/// the face (its label in it too), `Confirm`, or a reader turns `on`; it is
/// `State::CHECKED` while on.
pub fn toggle<M>(on: Signal<bool>, face: Div, switch: bool, disabled: impl IntoProp<bool, M>) -> Div {
    let turn = move || on.update(|on| *on = !*on);
    let face = face.role(if switch { Role::Switch } else { Role::Checkbox }).checked(on);
    let face = face.focusable().key_context(CONTROL).disabled(disabled);
    face.on_click(move |_| turn()).on_action::<Confirm>(move |_| turn())
}
