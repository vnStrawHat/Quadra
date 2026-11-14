//! The slider (specs/behaviour/09, "Forms"): a number between two ends, set by dragging a
//! knob along a track, by a press on the track, or by the arrows.

use std::cell::Cell;
use std::rc::Rc;

use quadra_layout::Length;
use quadra_platform::Cursor;
use quadra_reactive::{Signal, memo};
use quadra_tree::{NodeId, Paint, RangeValue, Role, SemanticAction};
use quadra_ui::{Div, IntoProp, MountCx, Prop, View, div};

use crate::action::{Decrease, DecreaseMore, Increase, IncreaseMore, MoveFirst, MoveLast, SLIDER};
use crate::token;

pub struct Slider {
    value: Signal<f32>,
    ends: (f32, f32),
    step: f32,
    width: f32,
    name: String,
    disabled: Prop<bool>,
}

/// A slider of `value`, from 0 to 100 in steps of 1, 200 px wide, unless told otherwise.
pub fn slider(value: Signal<f32>) -> Slider {
    let (ends, name, disabled) = ((0.0, 100.0), String::new(), Prop::Value(false));
    Slider { value, ends, step: 1.0, width: 200.0, name, disabled }
}

impl Slider {
    /// The two ends, and the step the value keeps to (0: none).
    pub fn range(mut self, min: f32, max: f32, step: f32) -> Self {
        (self.ends, self.step) = ((min, max), step);
        self
    }

    pub fn width(mut self, px: f32) -> Self {
        self.width = px;
        self
    }

    /// What a reader calls it.
    pub fn label(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn disabled<M>(mut self, disabled: impl IntoProp<bool, M>) -> Self {
        self.disabled = disabled.into_prop();
        self
    }
}

/// The scale of a slider: its ends, its step, and how long its track is. The track is 8 px
/// in from each end of the box: a knob, 12 px, is whole at both.
#[derive(Clone, Copy)]
pub(crate) struct Scale {
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub track: f32,
}

impl Scale {
    pub fn new((min, max): (f32, f32), step: f32, width: f32) -> Scale {
        Scale { min, max, step, track: (width - 16.0).max(1.0) }
    }

    /// The nearest value the slider can have.
    pub fn snap(self, to: f32) -> f32 {
        let (min, step) = (self.min, self.step);
        let stepped = if step > 0.0 { ((to - min) / step).round() * step + min } else { to };
        stepped.clamp(min, self.max)
    }

    /// The value under a point of the box, and how far along the track a value is.
    pub fn at(self, x: f32) -> f32 {
        self.snap(self.min + ((x - 8.0) / self.track).clamp(0.0, 1.0) * (self.max - self.min))
    }

    pub fn part(self, value: f32) -> f32 {
        let span = self.max - self.min;
        if span > 0.0 { ((value - self.min) / span).clamp(0.0, 1.0) } else { 0.0 }
    }

    /// What an arrow steps by, and the page keys.
    pub fn one(self) -> f32 {
        if self.step > 0.0 { self.step } else { 1.0 }
    }

    pub fn tenth(self) -> f32 {
        (self.max - self.min) / 10.0
    }

    /// A value as a number in the range, for a reader.
    pub fn range(self, value: f32) -> RangeValue {
        let (value, min, max, step) = (value as f64, self.min as f64, self.max as f64, self.step as f64);
        RangeValue { value, min, max, step }
    }

    /// A value as a reader is told it.
    pub fn says(self, value: f32) -> String {
        format!("{:.*}", if self.step.fract() == 0.0 { 0 } else { 2 }, value)
    }

    /// The track, and a knob with no place yet.
    pub fn rail(self) -> Div {
        let rail = div().absolute().left(8.0).top(8.0).height(4.0).radius(2.0);
        rail.width(self.track).bg(token::PRESS)
    }

    pub fn knob() -> Div {
        let knob = div().absolute().top(4.0).size(12.0, 12.0).radius(6.0);
        knob.border(1.0).border_color(token::CANVAS)
    }
}

impl View for Slider {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (value, scale) = (self.value, Scale::new(self.ends, self.step, self.width));
        let (min, max, track) = (scale.min, scale.max, scale.track);
        let off = match self.disabled {
            Prop::Value(off) => memo(move || off),
            Prop::Reactive(read) => memo(read),
        };
        let part = move || scale.part(value.get());
        let set = move |to: f32| {
            let to = scale.snap(to);
            if !off.get() && to != value.get_untracked() {
                value.set(to);
            }
        };
        let by = move |steps: f32| set(value.get_untracked() + steps * scale.one());
        let at = move |x: f32| set(scale.at(x));
        let tone = move |on: quadra_tree::Token| -> Paint {
            if off.get() { token::TEXT_DISABLED.into() } else { on.into() }
        };

        let fill = div().absolute().left(8.0).top(8.0).height(4.0).radius(2.0);
        let fill = fill.width(move || Length::Px(track * part())).bg(move || tone(token::ACCENT));
        let knob = Scale::knob().left(move || Length::Px(track * part() + 2.0));
        let (rail, knob) = (scale.rail(), knob.bg(move || tone(token::TEXT)));

        let held: Rc<Cell<bool>> = Rc::default();
        let (press, drag) = (held.clone(), held.clone());
        let face: Div = div().size(self.width, 20.0).shrink(0.0).cursor(Cursor::Pointer);
        let face = face.role(Role::Slider).label(self.name).value(move || scale.says(value.get()));
        let face = face.range(move || scale.range(value.get()));
        let face = face.focusable().key_context(SLIDER).disabled(move || off.get());
        let face = face.radius(4.0).focus_visible(|s| s.outline(token::BORDER_FOCUSED));
        let face = face.on_pointer(quadra_tree::PointerKind::Down, move |e| {
            press.set(true);
            e.capture_pointer();
            at(e.local.x);
        });
        let face = face.on_pointer(quadra_tree::PointerKind::Move, move |e| {
            if drag.get() {
                at(e.local.x);
            }
        });
        let face = face.on_pointer(quadra_tree::PointerKind::Up, move |_| held.set(false));
        // The arrows step; Page Up and Page Down go a tenth of the way; Home and End to the ends.
        let tenth = scale.tenth();
        let face = face.on_action::<Increase>(move |_| by(1.0)).on_action::<Decrease>(move |_| by(-1.0));
        let face = face.on_action::<IncreaseMore>(move |_| set(value.get_untracked() + tenth));
        let face = face.on_action::<DecreaseMore>(move |_| set(value.get_untracked() - tenth));
        let face = face.on_action::<MoveFirst>(move |_| set(min)).on_action::<MoveLast>(move |_| set(max));
        let face = face.on_semantic(move |action| {
            match action {
                SemanticAction::Increment => by(1.0),
                SemanticAction::Decrement => by(-1.0),
                SemanticAction::SetValue(text) => return text.trim().parse().map(set).is_ok(),
                _ => return false,
            }
            true
        });
        face.child(rail).child(fill).child(knob).mount(cx)
    }
}
