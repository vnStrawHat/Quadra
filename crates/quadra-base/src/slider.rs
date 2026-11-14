//! The slider (specs/17, "The components"): a number between two ends by steps, set by a
//! press on the track and a drag from it, by the arrows (Page Up and Page Down a tenth of the
//! way, Home and End the ends), or by a reader; and two knobs on one track for the ends of a
//! range, each a slider of its own that never passes the other. The look gives the face, the
//! track, the fill and the knobs, and places them by the share of the value.

use std::cell::Cell;
use std::rc::Rc;

use quadra_reactive::{Memo, Signal};
use quadra_tree::{PointerKind, RangeValue, Role, SemanticAction};
use quadra_ui::Div;

use crate::action::{Decrease, DecreaseMore, Increase, IncreaseMore, MoveFirst, MoveLast, SLIDER};

/// The scale of a slider: its ends, its step, and where its track is along the face.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scale {
    pub min: f32,
    pub max: f32,
    /// The step the value keeps to; 0: none.
    pub step: f32,
    /// Where the track starts along the face, and how long it is.
    pub start: f32,
    pub track: f32,
}

impl Scale {
    pub fn new((min, max): (f32, f32), step: f32, start: f32, track: f32) -> Scale {
        Scale { min, max, step, start, track: track.max(1.0) }
    }

    /// The nearest value the slider can have.
    pub fn snap(self, to: f32) -> f32 {
        let (min, step) = (self.min, self.step);
        let stepped = if step > 0.0 { ((to - min) / step).round() * step + min } else { to };
        stepped.clamp(min, self.max)
    }

    /// The value under a point of the face.
    pub fn at(self, x: f32) -> f32 {
        self.snap(self.min + ((x - self.start) / self.track).clamp(0.0, 1.0) * (self.max - self.min))
    }

    /// How far along the track a value is, from 0 to 1.
    pub fn part(self, value: f32) -> f32 {
        let span = self.max - self.min;
        if span > 0.0 { ((value - self.min) / span).clamp(0.0, 1.0) } else { 0.0 }
    }

    /// What an arrow steps by.
    pub fn one(self) -> f32 {
        if self.step > 0.0 { self.step } else { 1.0 }
    }

    /// What a page key steps by.
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
}

/// The keys and a reader's steps on what holds one value: `set` takes a value, `now` reads it.
fn keys(
    face: Div,
    scale: Scale,
    now: impl Fn() -> f32 + Copy + 'static,
    set: impl Fn(f32) + Copy + 'static,
) -> Div {
    let by = move |change: f32| set(now() + change);
    let face = face.focusable().key_context(SLIDER);
    let face =
        face.on_action::<Increase>(move |_| by(scale.one())).on_action::<Decrease>(move |_| by(-scale.one()));
    let face = face.on_action::<IncreaseMore>(move |_| by(scale.tenth()));
    let face = face.on_action::<DecreaseMore>(move |_| by(-scale.tenth()));
    let face =
        face.on_action::<MoveFirst>(move |_| set(scale.min)).on_action::<MoveLast>(move |_| set(scale.max));
    face.on_semantic(move |action| {
        match action {
            SemanticAction::Increment => by(scale.one()),
            SemanticAction::Decrement => by(-scale.one()),
            SemanticAction::SetValue(text) => return text.trim().parse().map(set).is_ok(),
            _ => return false,
        }
        true
    })
}

/// A press on `face` and a drag from it give `to` the point along the face.
fn dragged(face: Div, to: impl Fn(f32) + Copy + 'static) -> Div {
    let held: Rc<Cell<bool>> = Rc::default();
    let (press, drag) = (held.clone(), held.clone());
    let face = face.on_pointer(PointerKind::Down, move |e| {
        press.set(true);
        e.capture_pointer();
        to(e.local.x);
    });
    let face = face.on_pointer(PointerKind::Move, move |e| {
        if drag.get() {
            to(e.local.x);
        }
    });
    face.on_pointer(PointerKind::Up, move |_| held.set(false))
}

/// A slider of `value` on its face: a slider to a reader, with its value and its range. While
/// `off` is true it is disabled and nothing sets it.
pub fn slider(face: Div, value: Signal<f32>, scale: Scale, off: Memo<bool>) -> Div {
    let set = move |to: f32| {
        let to = scale.snap(to);
        if !off.get() && to != value.get_untracked() {
            value.set(to);
        }
    };
    let face = face.role(Role::Slider).value(move || scale.says(value.get()));
    let face = face.range(move || scale.range(value.get())).disabled(move || off.get());
    let face = dragged(face, move |x| set(scale.at(x)));
    keys(face, scale, move || value.get_untracked(), set)
}

/// The face of a range slider of `low` to `high`: a group to a reader. A press or a drag
/// moves the knob nearer to the pointer; between two at one place, the one on its side.
pub fn pair(face: Div, low: Signal<f32>, high: Signal<f32>, scale: Scale) -> Div {
    let near = move |x: f32| {
        let (to, at_low, at_high) = (scale.at(x), low.get_untracked(), high.get_untracked());
        let (from_low, from_high) = ((to - at_low).abs(), (to - at_high).abs());
        set_end(low, high, scale, from_low < from_high || (from_low == from_high && to < at_low), to);
    };
    dragged(face.role(Role::Group), near)
}

/// An end of a range, as far as the other end lets it.
fn set_end(low: Signal<f32>, high: Signal<f32>, scale: Scale, of_low: bool, to: f32) {
    let to = scale.snap(to);
    let (end, to) = match of_low {
        true => (low, to.min(high.get_untracked())),
        false => (high, to.max(low.get_untracked())),
    };
    if to != end.get_untracked() {
        end.set(to);
    }
}

/// A knob of a range slider on its face, of the low end or the high one: a slider of its own
/// to the keys and to a reader ("Minimum", "Maximum").
pub fn end(face: Div, low: Signal<f32>, high: Signal<f32>, scale: Scale, of_low: bool) -> Div {
    let (end, name) = if of_low { (low, "Minimum") } else { (high, "Maximum") };
    let face = face.role(Role::Slider).label(name.to_string()).value(move || scale.says(end.get()));
    let face = face.range(move || scale.range(end.get()));
    keys(face, scale, move || end.get_untracked(), move |to| set_end(low, high, scale, of_low, to))
}
