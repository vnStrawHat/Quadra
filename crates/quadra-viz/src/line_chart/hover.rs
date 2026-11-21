//! What a line chart knows of the pointer (specs/13, "Charts"): the sample it is nearest to,
//! the hairline there and the box of the values.

use std::rc::Rc;

use quadra_design::token;
use quadra_layout::{Align, Length};
use quadra_reactive::{Memo, Signal};
use quadra_tree::Paint;
use quadra_view::{Div, column, div, row, text};

use super::{Limit, Values, shown};
use crate::geometry::short;

/// What the chart knows of the pointer: the sample it is nearest to.
#[derive(Clone)]
pub(super) struct Hover {
    pub at: Signal<Option<usize>>,
    /// How many samples the first series has.
    pub count: Memo<usize>,
    /// The time of a sample as text; nothing for a chart that was given no times.
    pub time: Option<Rc<dyn Fn(usize) -> String>>,
}

impl Hover {
    /// How far along the plot the sample under the pointer is, from 0 to 100.
    pub fn along(&self) -> f32 {
        let last = self.count.get().saturating_sub(1).max(1);
        self.at.get().unwrap_or(0) as f32 * 100.0 / last as f32
    }

    /// The hairline at the sample.
    pub fn hair(&self) -> Div {
        let (at, along) = (self.at, self.clone());
        let hair = div().absolute().top(0.0).bottom(0.0).width(1.0).bg(token::TEXT_3);
        hair.left(move || Length::Pct(along.along())).display(move || shown(at.get().is_some()))
    }

    /// The box of the values at the sample, and the limit after them: at the right of the
    /// hairline in the left half of the plot, at its left in the right half.
    pub fn values(&self, lines: Vec<(String, Paint, Values)>, limit: Option<Limit>, unit: Rc<str>) -> Div {
        let at = self.at;
        let plane = column().padding_y(5.0).padding_right(12.0).radius(token::RADIUS_MD).elevation(1);
        let plane =
            plane.bg(token::RAISED).border(1.0).border_color(token::LINE_STRONG).text_color(token::TEXT);
        let plane = match self.time.clone() {
            Some(time) => {
                let time = text(move || at.get().map(|i| time(i)).unwrap_or_default());
                plane.child(div().padding_left(18.0).child(time))
            }
            None => plane,
        };
        let rows = lines.into_iter().map(|(name, paint, read)| {
            let unit = unit.clone();
            let says = move || {
                let value = at.get().map(|i| read().get(i).copied().unwrap_or(0.0));
                value.map(|value| format!("{name} {}{unit}", short(value))).unwrap_or_default()
            };
            let swatch = div().size(6.0, 6.0).shrink(0.0).radius(1.0).bg(paint);
            row().align_items(Align::Center).gap(4.0).padding_left(8.0).child(swatch).child(text(says))
        });
        let tail = unit.clone();
        let limit = limit.map(|(name, value)| {
            let unit = tail;
            let says = move || format!("{name} {}{unit}", short(value()));
            let dash = div().size(6.0, 2.0).shrink(0.0).bg(token::TEXT_2);
            row().align_items(Align::Center).gap(4.0).padding_left(8.0).child(dash).child(text(says))
        });
        let (left, right) = (self.clone(), self.clone());
        let place = row().absolute().top(4.0).padding_x(12.0);
        let place = place.left(move || match left.along() {
            along if along <= 50.0 => Length::Pct(along),
            _ => Length::Auto,
        });
        let place = place.right(move || match right.along() {
            along if along > 50.0 => Length::Pct(100.0 - along),
            _ => Length::Auto,
        });
        place.display(move || shown(at.get().is_some())).child(plane.children(rows).children(limit))
    }
}
