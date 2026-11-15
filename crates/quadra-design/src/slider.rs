//! The slider (specs/behaviour/09, "Forms"): a number between two ends, set by dragging a
//! knob along a track, by a press on the track, or by the arrows.

use quadra_headless::slider::{self, Scale};
use quadra_layout::Length;
use quadra_platform::Cursor;
use quadra_reactive::{Signal, memo};
use quadra_tree::{NodeId, Paint};
use quadra_view::{Div, IntoProp, MountCx, Prop, View, div};

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

/// The scale of a slider of `width`: the track is 8 px in from each end of the box, so a
/// knob, 12 px, is whole at both.
pub(crate) fn scale(ends: (f32, f32), step: f32, width: f32) -> Scale {
    Scale::new(ends, step, 8.0, width - 16.0)
}

/// The track, and a knob with no place yet.
pub(crate) fn rail(scale: Scale) -> Div {
    let rail = div().absolute().left(8.0).top(8.0).height(4.0).radius(2.0);
    rail.width(scale.track).bg(token::PRESS)
}

pub(crate) fn knob() -> Div {
    let knob = div().absolute().top(4.0).size(12.0, 12.0).radius(6.0);
    knob.border(1.0).border_color(token::CANVAS)
}

impl View for Slider {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (value, scale) = (self.value, scale(self.ends, self.step, self.width));
        let track = scale.track;
        let off = match self.disabled {
            Prop::Value(off) => memo(move || off),
            Prop::Reactive(read) => memo(read),
        };
        let part = move || scale.part(value.get());
        let tone = move |on: quadra_tree::Token| -> Paint {
            if off.get() { token::TEXT_DISABLED.into() } else { on.into() }
        };
        let fill = div().absolute().left(8.0).top(8.0).height(4.0).radius(2.0);
        let fill = fill.width(move || Length::Px(track * part())).bg(move || tone(token::ACCENT));
        let knob = knob().left(move || Length::Px(track * part() + 2.0));
        let (rail, knob) = (rail(scale), knob.bg(move || tone(token::TEXT)));
        let face: Div = div().size(self.width, 20.0).shrink(0.0).cursor(Cursor::Pointer).label(self.name);
        let face = face.radius(4.0).focus_visible(|s| s.outline(token::BORDER_FOCUSED));
        slider::slider(face, value, scale, off).child(rail).child(fill).child(knob).mount(cx)
    }
}
