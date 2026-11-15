//! The range slider (specs/behaviour/09, "Forms"): two knobs on one track, for the low end
//! and the high end of a range. A press or a drag moves the knob nearer to it; each knob has
//! the focus and the arrows of its own, and never passes the other.

use quadra_headless::slider;
use quadra_layout::Length;
use quadra_platform::Cursor;
use quadra_reactive::Signal;
use quadra_tree::NodeId;
use quadra_view::{Div, IntoProp, MountCx, Prop, View, div};

use crate::slider::{knob, rail, scale};
use crate::token;

pub struct RangeSlider {
    low: Signal<f32>,
    high: Signal<f32>,
    ends: (f32, f32),
    step: f32,
    width: f32,
    name: Prop<String>,
}

/// A slider of the range from `low` to `high`, between 0 and 100 in steps of 1, 200 px wide,
/// unless told otherwise.
pub fn range_slider(low: Signal<f32>, high: Signal<f32>) -> RangeSlider {
    RangeSlider { low, high, ends: (0.0, 100.0), step: 1.0, width: 200.0, name: Prop::Value(String::new()) }
}

impl RangeSlider {
    /// The two ends, and the step the values keep to (0: none).
    pub fn range(mut self, min: f32, max: f32, step: f32) -> Self {
        (self.ends, self.step) = ((min, max), step);
        self
    }

    pub fn width(mut self, px: f32) -> Self {
        self.width = px;
        self
    }

    /// What a reader calls the pair, a constant or words that change.
    pub fn label<M>(mut self, name: impl IntoProp<String, M>) -> Self {
        self.name = name.into_prop();
        self
    }
}

impl View for RangeSlider {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (low, high, scale) = (self.low, self.high, scale(self.ends, self.step, self.width));
        let track = scale.track;
        let knob = move |of_low: bool| {
            let end = if of_low { low } else { high };
            let face = knob().left(move || Length::Px(track * scale.part(end.get()) + 2.0)).bg(token::TEXT);
            let face = face.focus_visible(|s| s.outline(token::BORDER_FOCUSED));
            slider::end(face, low, high, scale, of_low)
        };
        let from = move || track * scale.part(low.get());
        let fill = div().absolute().top(8.0).height(4.0).radius(2.0).bg(token::ACCENT);
        let fill = fill.left(move || Length::Px(8.0 + from()));
        let fill = fill.width(move || Length::Px(track * scale.part(high.get()) - from()));
        let face: Div = div().size(self.width, 20.0).shrink(0.0).cursor(Cursor::Pointer).label(self.name);
        let face = slider::pair(face, low, high, scale);
        face.child(rail(scale)).child(fill).child(knob(true)).child(knob(false)).mount(cx)
    }
}
