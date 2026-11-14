//! The range slider (specs/behaviour/09, "Forms"): two knobs on one track, for the low end
//! and the high end of a range. A press or a drag moves the knob nearer to it; each knob has
//! the focus and the arrows of its own, and never passes the other.

use std::cell::Cell;
use std::rc::Rc;

use quadra_layout::Length;
use quadra_platform::Cursor;
use quadra_reactive::Signal;
use quadra_tree::{NodeId, PointerKind, Role, SemanticAction};
use quadra_ui::{Div, MountCx, View, div};

use crate::action::{Decrease, DecreaseMore, Increase, IncreaseMore, MoveFirst, MoveLast, SLIDER};
use crate::slider::Scale;
use crate::token;

pub struct RangeSlider {
    low: Signal<f32>,
    high: Signal<f32>,
    ends: (f32, f32),
    step: f32,
    width: f32,
    name: String,
}

/// A slider of the range from `low` to `high`, between 0 and 100 in steps of 1, 200 px wide,
/// unless told otherwise.
pub fn range_slider(low: Signal<f32>, high: Signal<f32>) -> RangeSlider {
    RangeSlider { low, high, ends: (0.0, 100.0), step: 1.0, width: 200.0, name: String::new() }
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

    /// What a reader calls the pair.
    pub fn label(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

impl View for RangeSlider {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (low, high, scale) = (self.low, self.high, Scale::new(self.ends, self.step, self.width));
        let track = scale.track;
        // An end is set as far as the other end lets it.
        let set = move |of_low: bool, to: f32| {
            let to = scale.snap(to);
            let (end, to) = match of_low {
                true => (low, to.min(high.get_untracked())),
                false => (high, to.max(low.get_untracked())),
            };
            if to != end.get_untracked() {
                end.set(to);
            }
        };
        // The pointer moves the knob nearer to it; between two at one place, the one on its side.
        let near = move |x: f32| {
            let (to, low, high) = (scale.at(x), low.get_untracked(), high.get_untracked());
            let (from_low, from_high) = ((to - low).abs(), (to - high).abs());
            set(from_low < from_high || (from_low == from_high && to < low), to);
        };
        let knob = move |of_low: bool| {
            let (end, name) = if of_low { (low, "Minimum") } else { (high, "Maximum") };
            let knob = Scale::knob().left(move || Length::Px(track * scale.part(end.get()) + 2.0));
            let knob = knob.bg(token::TEXT).role(Role::Slider).label(name.to_string());
            let knob = knob.value(move || scale.says(end.get())).focusable().key_context(SLIDER);
            let knob = knob.range(move || scale.range(end.get()));
            let knob = knob.focus_visible(|s| s.outline(token::BORDER_FOCUSED));
            let by = move |change: f32| set(of_low, end.get_untracked() + change);
            let knob = knob.on_action::<Increase>(move |_| by(scale.one()));
            let knob = knob.on_action::<Decrease>(move |_| by(-scale.one()));
            let knob = knob.on_action::<IncreaseMore>(move |_| by(scale.tenth()));
            let knob = knob.on_action::<DecreaseMore>(move |_| by(-scale.tenth()));
            let knob = knob.on_action::<MoveFirst>(move |_| set(of_low, scale.min));
            let knob = knob.on_action::<MoveLast>(move |_| set(of_low, scale.max));
            // A reader steps it as the arrows do.
            knob.on_semantic(move |action| match action {
                SemanticAction::Increment => {
                    by(scale.one());
                    true
                }
                SemanticAction::Decrement => {
                    by(-scale.one());
                    true
                }
                _ => false,
            })
        };
        let from = move || track * scale.part(low.get());
        let fill = div().absolute().top(8.0).height(4.0).radius(2.0).bg(token::ACCENT);
        let fill = fill.left(move || Length::Px(8.0 + from()));
        let fill = fill.width(move || Length::Px(track * scale.part(high.get()) - from()));

        let held: Rc<Cell<bool>> = Rc::default();
        let (press, drag) = (held.clone(), held.clone());
        let face: Div = div().size(self.width, 20.0).shrink(0.0).cursor(Cursor::Pointer);
        let face = face.role(Role::Group).label(self.name);
        let face = face.on_pointer(PointerKind::Down, move |e| {
            press.set(true);
            e.capture_pointer();
            near(e.local.x);
        });
        let face = face.on_pointer(PointerKind::Move, move |e| {
            if drag.get() {
                near(e.local.x);
            }
        });
        let face = face.on_pointer(PointerKind::Up, move |_| held.set(false));
        face.child(scale.rail()).child(fill).child(knob(true)).child(knob(false)).mount(cx)
    }
}
