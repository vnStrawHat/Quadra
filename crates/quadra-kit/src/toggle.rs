//! The controls that are on or off (specs/behaviour/15, "Components"): the checkbox and the
//! switch. Each takes the signal that holds its value, and writes it.

use std::rc::Rc;

use quadra_layout::{Align, Justify};
use quadra_platform::Cursor;
use quadra_reactive::Signal;
use quadra_tree::{NodeId, Paint, Role, State};
use quadra_ui::{Div, IntoProp, MountCx, Prop, View, div, dynamic, row, show, text};

use crate::action::{CONTROL, Confirm};
use crate::icon::icon;
use crate::{icons, size, token};

pub struct Toggle {
    on: Signal<bool>,
    is_switch: bool,
    label: Option<Prop<String>>,
    disabled: Prop<bool>,
    mixed: Prop<bool>,
    test_id: Option<&'static str>,
}

/// A box that is checked or not: `checked` holds which, and a click, Space or Enter turns it.
pub fn checkbox(checked: Signal<bool>) -> Toggle {
    let (disabled, mixed) = (Prop::Value(false), Prop::Value(false));
    Toggle { on: checked, is_switch: false, label: None, disabled, mixed, test_id: None }
}

/// A switch that is on or off: `on` holds which.
pub fn switch(on: Signal<bool>) -> Toggle {
    Toggle { is_switch: true, ..checkbox(on) }
}

/// Whether a control is disabled, for each part of it that looks different then.
type Off = Rc<dyn Fn() -> bool>;

/// A box 16 px square with a hairline border; the check is the accent, or the colour of
/// what is off.
fn check_box(on: Signal<bool>, off: Off, mixed: Off) -> Div {
    let mark = move || Paint::from(if off() { token::TEXT_DISABLED } else { token::ACCENT_TEXT });
    let edge = div().size(16.0, 16.0).shrink(0.0).align_items(Align::Center).justify(Justify::Center);
    let edge = edge.radius(size::RADIUS_SM).border(1.0).border_color(token::LINE_STRONG);
    let edge = edge.text_color(mark).hover(|s| s.border_color(token::LINE_STRONG.alpha(0.7)));
    // A check when it is checked; a dash while it is neither.
    let dash = mixed.clone();
    let mark = move || icon(if mixed() { icons::MINUS } else { icons::CHECK });
    edge.child(show(move || on.get() || dash(), move || dynamic(mark.clone())))
}

/// A track of 32 by 20 px inside a ring one pixel off it, 38 by 26 in all, with a knob of
/// 12 px in the text colour: at the end and the accent behind it when on.
fn switch_track(on: Signal<bool>, off: Off) -> Div {
    let fill = move || if on.get() { token::ACCENT.alpha(0.4) } else { Paint::from(token::RAISED) };
    let edge = move || if on.get() { token::ACCENT_TEXT.alpha(0.2) } else { Paint::from(token::LINE_STRONG) };
    // The knob is the text colour: whole when on, half when off, faint when disabled.
    let knob = move || {
        token::TEXT.alpha(if off() {
            0.2
        } else if on.get() {
            1.0
        } else {
            0.5
        })
    };
    let track = row().size(32.0, 20.0).shrink(0.0).radius(10.0).padding_x(2.0).align_items(Align::Center);
    let track = track.bg(fill).border(1.0).border_color(edge);
    let track = track.justify(move || if on.get() { Justify::End } else { Justify::Start });
    let track = track.child(div().size(12.0, 12.0).radius(6.0).bg(knob));
    div().padding(1.0).shrink(0.0).radius(13.0).border(2.0).border_color(token::LINE_STRONG).child(track)
}

impl Toggle {
    /// The text beside the control: a click on it turns the control too.
    pub fn label<M>(mut self, label: impl IntoProp<String, M>) -> Self {
        self.label = Some(label.into_prop());
        self
    }

    pub fn disabled<M>(mut self, disabled: impl IntoProp<bool, M>) -> Self {
        self.disabled = disabled.into_prop();
        self
    }

    /// While `mixed` is true a checkbox shows a dash: some of what it stands for is
    /// checked, and some is not.
    pub fn indeterminate<M>(mut self, mixed: impl IntoProp<bool, M>) -> Self {
        self.mixed = mixed.into_prop();
        self
    }

    pub fn test_id(mut self, id: &'static str) -> Self {
        self.test_id = Some(id);
        self
    }

    fn build(self) -> Div {
        let on = self.on;
        let mixed: Off = match self.mixed {
            Prop::Value(mixed) => Rc::new(move || mixed),
            Prop::Reactive(read) => Rc::new(read),
        };
        let turn = move || on.update(|on| *on = !*on);
        let off: Off = match self.disabled {
            Prop::Value(off) => Rc::new(move || off),
            Prop::Reactive(read) => Rc::new(read),
        };
        let part = off.clone();
        let control = if self.is_switch { switch_track(on, part) } else { check_box(on, part, mixed) };
        let face = row().align_items(Align::Center).gap(6.0).shrink(0.0).radius(size::RADIUS_SM);
        let face = face.role(if self.is_switch { Role::Switch } else { Role::Checkbox }).checked(on);
        let face = face.focusable().key_context(CONTROL).cursor(Cursor::Pointer);
        // The keyboard's focus is a line around the control and its label.
        let face = face.focus_visible(|s| s.outline(token::BORDER_FOCUSED));
        let face = face.disabled(move || off()).when(State::DISABLED, |s| s.text_color(token::TEXT_DISABLED));
        let face = face.on_click(move |_| turn()).on_action::<Confirm>(move |_| turn());
        let face = match self.test_id {
            Some(id) => face.test_id(id),
            None => face,
        };
        match self.label {
            Some(label) => face.child(control).child(text(label)),
            None => face.child(control),
        }
    }
}

impl View for Toggle {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        self.build().mount(cx)
    }
}
