//! What says that something is under way (specs/behaviour/09, "Data"): the progress bar,
//! the spinner and the skeleton. What moves in them is a layer: each frame writes a constant
//! of it, and nothing is laid out or painted.

use std::f32::consts::TAU;
use std::rc::Rc;

use quadra_layout::{Length, Overflow};
use quadra_prim::Point;
use quadra_tree::{NodeId, RangeValue, Role, Token};
use quadra_view::{Div, IntoProp, MountCx, Prop, View, div, each_frame};

use crate::icon::icon;
use crate::{icons, token};

/// A bar of 6 px, 200 wide unless told, filled with the accent up to `part`, from 0 to 1.
pub fn progress<M>(part: impl IntoProp<f32, M>) -> Div {
    let part: Rc<dyn Fn() -> f32> = match part.into_prop() {
        Prop::Value(part) => Rc::new(move || part),
        Prop::Reactive(read) => Rc::from(read),
    };
    let (width, percent, number) = (part.clone(), part.clone(), part);
    let fill = div().height(6.0).radius(3.0).bg(token::ACCENT);
    let fill = fill.width(move || Length::Pct(width().clamp(0.0, 1.0) * 100.0));
    let says = move || format!("{:.0}%", percent().clamp(0.0, 1.0) * 100.0);
    let range =
        move || RangeValue { value: number().clamp(0.0, 1.0) as f64, max: 1.0, ..RangeValue::default() };
    track().role(Role::ProgressBar).value(says).range(range).child(fill)
}

fn track() -> Div {
    let track = div().size(200.0, 6.0).shrink(0.0).radius(3.0).bg(token::CHROME);
    track.overflow_x(Overflow::Hidden)
}

/// How long a turn of each takes, in seconds.
const SWEEP: f32 = 1.4;
const TURN: f32 = 0.9;

pub struct Busy {
    width: f32,
    height: f32,
    track: Token,
}

/// A progress bar for what has no end in sight: a third of it, in the accent, crosses the
/// track again and again.
pub fn progress_busy() -> Busy {
    Busy { width: 200.0, height: 6.0, track: token::CHROME }
}

impl Busy {
    pub fn width(mut self, px: f32) -> Self {
        self.width = px;
        self
    }

    /// Thinner: its corners are half its height, none at 2 px (a seam).
    pub fn height(mut self, px: f32) -> Self {
        self.height = px;
        self
    }

    /// The colour of its track: `chrome` unless told.
    pub fn track(mut self, color: Token) -> Self {
        self.track = color;
        self
    }
}

impl View for Busy {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (whole, part, high) = (self.width, self.width * 0.35, self.height);
        let round = if high > 2.0 { high / 2.0 } else { 0.0 };
        let piece = div().size(part, high).radius(round).bg(token::ACCENT).mount(cx);
        let bar = track().size(whole, high).radius(round).bg(self.track);
        let bar = bar.role(Role::ProgressBar).label("Busy".to_string()).mount(cx);
        cx.tree.set_children(bar, &[piece]);
        cx.tree.make_layer(piece, true);
        let (handle, since) = (cx.handle.clone(), cx.handle.now());
        each_frame(move || {
            let phase = (handle.now() - since).as_secs_f32() % SWEEP / SWEEP;
            let x = -part + (whole + part) * phase;
            handle.with(move |tree| tree.set_layer_offset(piece, Point::new(x, 0.0)));
            true
        });
        bar
    }
}

pub struct Spinner {
    side: f32,
    label: String,
}

/// An open ring that turns, 16 px unless told, in the colour of the text around it.
pub fn spinner() -> Spinner {
    Spinner { side: 16.0, label: "Loading".into() }
}

impl Spinner {
    /// What a reader hears of it: `Loading` unless told.
    pub fn label(mut self, words: impl Into<String>) -> Self {
        self.label = words.into();
        self
    }

    pub fn size(mut self, px: f32) -> Self {
        self.side = px;
        self
    }
}

impl View for Spinner {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        // The ring is drawn once; its layer is turned to the angle of each frame.
        let ring = icon(icons::LOADER_CIRCLE).size(self.side).mount(cx);
        cx.tree.make_layer(ring, false);
        let (handle, since) = (cx.handle.clone(), cx.handle.now());
        each_frame(move || {
            let degrees = (handle.now() - since).as_secs_f32() % TURN / TURN * 360.0;
            handle.with(move |tree| tree.set_layer_turn(ring, degrees));
            true
        });
        let around = div().shrink(0.0).role(Role::ProgressBar).label(self.label).mount(cx);
        cx.tree.set_children(around, &[ring]);
        around
    }
}

pub struct Skeleton {
    width: Length,
    height: f32,
}

/// A block that stands for what is still on its way, and pulses. A width of 0 is all the
/// width there is.
pub fn skeleton(width: f32, height: f32) -> Skeleton {
    Skeleton { width: if width > 0.0 { Length::Px(width) } else { Length::Fill }, height }
}

impl View for Skeleton {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        // Two tones, the second over the first in a layer that fades in and out.
        let over = div().absolute().left(0.0).top(0.0).right(0.0).bottom(0.0).radius(6.0).bg(token::PRESS);
        let over = over.mount(cx);
        let block = div().width(self.width).height(self.height).shrink(0.0).radius(6.0);
        let block = block.bg(token::RAISED_HOVER).mount(cx);
        cx.tree.set_children(block, &[over]);
        cx.tree.make_layer(over, false);
        let (handle, since) = (cx.handle.clone(), cx.handle.now());
        each_frame(move || {
            let seconds = (handle.now() - since).as_secs_f32();
            let pulse = 0.55 + 0.45 * (seconds * TAU * 0.7).sin();
            handle.with(move |tree| tree.set_layer_opacity(over, pulse));
            true
        });
        block
    }
}
