//! The controls that are on or off (specs/design, "Components"): the checkbox and the
//! switch. Each takes the signal that holds its value, and writes it.

use std::rc::Rc;

use quadra_layout::{Align, Justify};
use quadra_platform::Cursor;
use quadra_reactive::Signal;
use quadra_tree::{NodeId, Paint, State};
use quadra_view::{Div, IntoProp, MountCx, Prop, View, div, dynamic, row, show, text};

use crate::icon::icon;
use crate::{icons, token};

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

/// A box 16 px square with a hairline border; checked, it is filled with the accent, or the
/// colour of what is off, and the check is dark on it.
fn check_box(on: Signal<bool>, off: Off, mixed: Off) -> Div {
    let (dash, full) = (mixed.clone(), mixed.clone());
    let fill = move || match on.get() || full() {
        true => Paint::from(if off() { token::TEXT_DISABLED } else { token::ACCENT }),
        false => Paint::default(),
    };
    let shown = fill.clone();
    let edge = move || match shown() {
        Paint::Color(_) => Paint::from(token::LINE_STRONG),
        filled => filled,
    };
    let frame = div().size(16.0, 16.0).shrink(0.0).align_items(Align::Center).justify(Justify::Center);
    let frame = frame.radius(token::RADIUS_SM).border(1.0).border_color(edge).bg(fill);
    let frame = frame.text_color(token::TEXT_ON_ACCENT);
    // A check when it is checked; a dash while it is neither.
    let mark = move || icon(if mixed() { icons::MINUS } else { icons::CHECK }).size(12.0);
    frame.child(show(move || on.get() || dash(), move || dynamic(mark.clone())))
}

/// A track of 32 by 20 px inside a ring one pixel off it, 38 by 26 in all, with a knob of
/// 12 px: when on, at the end, dark on a track of the accent.
fn switch_track(on: Signal<bool>, off: Off) -> Div {
    let lit = off.clone();
    let fill = move || match on.get() {
        true => Paint::from(if lit() { token::TEXT_DISABLED } else { token::ACCENT }),
        false => Paint::from(token::RAISED),
    };
    let shown = fill.clone();
    let edge = move || if on.get() { shown() } else { Paint::from(token::LINE_STRONG) };
    let ring = move || if on.get() { token::ACCENT.alpha(0.35) } else { Paint::from(token::LINE_STRONG) };
    // The knob is dark on the accent; off, the text colour at half, faint when disabled.
    let knob = move || match (on.get(), off()) {
        (true, _) => Paint::from(token::TEXT_ON_ACCENT),
        (false, off) => token::TEXT.alpha(if off { 0.2 } else { 0.5 }),
    };
    let track = row().size(32.0, 20.0).shrink(0.0).radius(10.0).padding_x(2.0).align_items(Align::Center);
    let track = track.bg(fill).border(1.0).border_color(edge);
    let track = track.justify(move || if on.get() { Justify::End } else { Justify::Start });
    let track = track.child(div().size(12.0, 12.0).radius(6.0).bg(knob));
    // design: a switch is a pill, its ring half its height
    div().padding(1.0).shrink(0.0).radius(13.0).border(2.0).border_color(ring).child(track)
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
        let off: Off = match self.disabled {
            Prop::Value(off) => Rc::new(move || off),
            Prop::Reactive(read) => Rc::new(read),
        };
        let part = off.clone();
        let control = if self.is_switch { switch_track(on, part) } else { check_box(on, part, mixed) };
        let face = row().align_items(Align::Center).gap(6.0).shrink(0.0).radius(token::RADIUS_SM);
        let face = face.cursor(Cursor::Pointer);
        // The keyboard's focus is a line around the control and its label.
        let face = face.focus_visible(|s| s.outline(token::BORDER_FOCUSED));
        let face = face.when(State::DISABLED, |s| s.text_color(token::TEXT_DISABLED));
        let face = match self.is_switch {
            true => quadra_headless::switch(on, face, move || off()),
            false => quadra_headless::checkbox(on, face, move || off()),
        };
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
