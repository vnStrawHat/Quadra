//! The slider (specs/behaviour/09, "Forms"): a number between two ends, set by dragging a
//! knob along a track, by a press on the track, or by the arrows.

use std::cell::Cell;
use std::rc::Rc;

use quadra_layout::Length;
use quadra_platform::Cursor;
use quadra_reactive::{Signal, memo};
use quadra_tree::{NodeId, Paint, Role, SemanticAction};
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

impl View for Slider {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (value, (min, max), step) = (self.value, self.ends, self.step);
        // The track is 8 px in from each end: the knob, 12 px, is whole at both.
        let track = (self.width - 16.0).max(1.0);
        let off = match self.disabled {
            Prop::Value(off) => memo(move || off),
            Prop::Reactive(read) => memo(read),
        };
        let part = move || if max > min { ((value.get() - min) / (max - min)).clamp(0.0, 1.0) } else { 0.0 };
        let set = move |to: f32| {
            let stepped = if step > 0.0 { ((to - min) / step).round() * step + min } else { to };
            let to = stepped.clamp(min, max);
            if !off.get() && to != value.get_untracked() {
                value.set(to);
            }
        };
        let by = move |steps: f32| set(value.get_untracked() + steps * if step > 0.0 { step } else { 1.0 });
        let at = move |x: f32| set(min + ((x - 8.0) / track).clamp(0.0, 1.0) * (max - min));
        let tone = move |on: quadra_tree::Token| -> Paint {
            if off.get() { token::TEXT_DISABLED.into() } else { on.into() }
        };

        let bar = || div().absolute().left(8.0).top(8.0).height(4.0).radius(2.0);
        let rail = bar().width(track).bg(token::PRESS);
        let fill = bar().width(move || Length::Px(track * part())).bg(move || tone(token::ACCENT));
        let knob = div().absolute().top(4.0).left(move || Length::Px(track * part() + 2.0));
        let knob = knob.size(12.0, 12.0).radius(6.0).border(1.0).border_color(token::CANVAS);
        let knob = knob.bg(move || tone(token::TEXT));

        let held: Rc<Cell<bool>> = Rc::default();
        let (press, drag) = (held.clone(), held.clone());
        let whole = if step.fract() == 0.0 { 0 } else { 2 };
        let face: Div = div().size(self.width, 20.0).shrink(0.0).cursor(Cursor::Pointer);
        let face =
            face.role(Role::Slider).label(self.name).value(move || format!("{:.*}", whole, value.get()));
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
        let tenth = (max - min) / 10.0;
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
