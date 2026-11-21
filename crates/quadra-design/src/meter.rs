//! The meter (specs/13, "What is under way"): how full something is, a measure and not a task
//! under way. A track filled as far as a value, a tick at a second one over the fill, and a
//! fill that warns as it nears the top.

use std::rc::Rc;

use quadra_layout::Length;
use quadra_tree::{NodeId, Paint, RangeValue, Role};
use quadra_view::{IntoProp, MountCx, Prop, View, div};

use crate::token;

type Read = Rc<dyn Fn() -> f32>;

fn reader(prop: Prop<f32>) -> Read {
    match prop {
        Prop::Value(value) => Rc::new(move || value),
        Prop::Reactive(read) => Rc::from(read),
    }
}

pub struct Meter {
    used: Read,
    mark: Option<Read>,
    warn: Option<f32>,
    width: Length,
    height: f32,
    names: (String, String),
}

/// How full something is: `used` from 0 to 1 (past 1 when over), a constant or reactive. A
/// bar of 6 px, 200 wide unless told, filled with the accent.
pub fn meter<M>(used: impl IntoProp<f32, M>) -> Meter {
    let names = ("used".to_string(), "mark".to_string());
    let width = Length::Px(200.0);
    Meter { used: reader(used.into_prop()), mark: None, warn: None, width, height: 6.0, names }
}

impl Meter {
    /// A tick at a second value over the fill (what is requested of what is used): 2 px in
    /// the text's colour, with a ring of the panel's so it reads over the fill.
    pub fn mark<M>(mut self, at: impl IntoProp<f32, M>) -> Self {
        self.mark = Some(reader(at.into_prop()));
        self
    }

    /// The fill in the colour of warnings from `part` up, and of errors past 1.
    pub fn warn_at(mut self, part: f32) -> Self {
        self.warn = Some(part);
        self
    }

    /// What a reader calls the value and the mark: "used", "requested".
    pub fn names(mut self, used: impl Into<String>, mark: impl Into<String>) -> Self {
        self.names = (used.into(), mark.into());
        self
    }

    pub fn width(mut self, px: f32) -> Self {
        self.width = Length::Px(px);
        self
    }

    /// As wide as the box it is in, following it as the box changes.
    pub fn fill(mut self) -> Self {
        self.width = Length::Fill;
        self
    }

    /// 6 px unless told: 4 in a row of a table.
    pub fn height(mut self, px: f32) -> Self {
        self.height = px;
        self
    }
}

impl View for Meter {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let Meter { used, mark, warn, width, height, names } = self;
        let (round, part) = (height / 2.0, used.clone());
        let tone = move || -> Paint {
            match (warn, part()) {
                (Some(_), over) if over > 1.0 => token::ERROR.into(),
                (Some(from), at) if at >= from => token::WARN.into(),
                _ => token::ACCENT.into(),
            }
        };
        let far = used.clone();
        let fill = div().height(height).radius(round).bg(tone);
        let fill = fill.width(move || Length::Pct(far().clamp(0.0, 1.0) * 100.0));
        let percent = |part: f32| (part * 100.0).round();
        let (said, marked) = (used.clone(), mark.clone());
        let says = move || match &marked {
            Some(at) => format!("{} {}%, {} {}%", names.0, percent(said()), names.1, percent(at())),
            None => format!("{} {}%", names.0, percent(said())),
        };
        let range = move || RangeValue { value: used() as f64, max: 1.0, ..RangeValue::default() };
        let track = div().width(width).height(height).radius(round).bg(token::LINE_STRONG);
        let track = if matches!(width, Length::Px(_)) { track.shrink(0.0) } else { track };
        let track = track.role(Role::ProgressBar).value(says).range(range).child(fill);
        let track = match mark {
            Some(at) => {
                // A holder of no width at the value, the tick centred on it, past the track by
                // half its height above and below.
                let left = move || Length::Pct(at().clamp(0.0, 1.0) * 100.0);
                let tick = div().absolute().left(-2.0).top(-round).bottom(-round).width(4.0);
                let tick = tick.bg(token::TEXT).border(1.0).border_color(token::PANEL);
                track.child(div().absolute().top(0.0).bottom(0.0).width(0.0).left(left).child(tick))
            }
            None => track,
        };
        track.mount(cx)
    }
}
