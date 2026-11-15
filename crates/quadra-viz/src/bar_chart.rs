//! The bar chart (specs/13, "Charts"): a row a bar, its label, a track with the bar in it
//! and its value. Views alone.

use std::rc::Rc;

use quadra_design::{fonts, token};
use quadra_layout::{Align, Justify, Length};
use quadra_reactive::{Signal, memo};
use quadra_tree::{NodeId, Paint};
use quadra_view::{Div, IntoProp, MountCx, Prop, View, column, div, each, row, text};

use crate::geometry::{max_of, nice_max, short};

/// What the label of a bar and its value take of the chart's width, in pixels.
const LABEL: f32 = 160.0;
const VALUE: f32 = 64.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Bar {
    pub label: String,
    pub value: f64,
    color: Option<Paint>,
}

impl Bar {
    pub fn new(label: impl Into<String>, value: f64) -> Bar {
        Bar { label: label.into(), value, color: None }
    }

    /// A colour of its own, or a role of the theme: the first of the series unless told.
    pub fn color(mut self, color: impl Into<Paint>) -> Self {
        self.color = Some(color.into());
        self
    }

    fn paint(&self) -> Paint {
        self.color.unwrap_or(token::SERIES[0].into())
    }

    /// How much of the track the bar takes under a top of `max`.
    fn share(&self, max: f64) -> Length {
        Length::Pct(if max > 0.0 { (self.value.max(0.0) / max).min(1.0) as f32 * 100.0 } else { 0.0 })
    }
}

pub struct BarChart {
    bars: Prop<Vec<Bar>>,
    unit: Rc<str>,
    /// Upright, this tall: a column a bar, side by side.
    upright: Option<f32>,
}

/// Bars one under the other, each as long as its share of the largest value rounded up.
/// `bars` is a constant or something reactive; a row is kept by its label.
pub fn bar_chart<M>(bars: impl IntoProp<Vec<Bar>, M>) -> BarChart {
    BarChart { bars: bars.into_prop(), unit: "".into(), upright: None }
}

impl BarChart {
    /// What follows a value: "m", " MB".
    pub fn unit(mut self, unit: &str) -> Self {
        self.unit = unit.into();
        self
    }

    /// Bars upright side by side, `height` px with their values over them and their labels
    /// under them: a chart of categories.
    pub fn upright(mut self, height: f32) -> Self {
        self.upright = Some(height);
        self
    }
}

fn top(bars: &[Bar]) -> f64 {
    nice_max(max_of(bars.iter().map(|bar| &bar.value)))
}

/// A row: the label, the track with the bar, the value; upright, a column: the value, the
/// track with the bar standing at its bottom, the label.
fn line(upright: bool, label: String, share: Prop<Length>, paint: Prop<Paint>, says: Prop<String>) -> Div {
    if upright {
        let bar = div().width(Length::Fill).min_height(2.0).radius(2.0).height(share).bg(paint);
        let track = column().grow(1.0).width(Length::Fill).justify(Justify::End).child(bar);
        let label = div().height(18.0).text_size(12.0).text_color(token::TEXT_2).child(text(label));
        let column = column().width(0.0).grow(1.0).align_items(Align::Center).gap(4.0);
        return column.child(text(says)).child(track.max_width(48.0)).child(label);
    }
    let label = div().width(LABEL).shrink(0.0).text_size(12.0).text_color(token::TEXT_2).child(text(label));
    let bar = div().height(Length::Fill).min_width(2.0).radius(2.0).width(share).bg(paint);
    let track = div().grow(1.0).height(14.0).radius(2.0).bg(token::LINE).child(bar);
    let value = div().width(VALUE).shrink(0.0).padding_left(8.0).child(text(says));
    row().height(24.0).align_items(Align::Center).child(label).child(track).child(value)
}

impl View for BarChart {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (unit, upright) = (self.unit, self.upright.is_some());
        let chart = match self.upright {
            Some(height) => row().height(height).gap(8.0).align_items(Align::Stretch),
            None => column(),
        };
        let chart = chart.width(Length::Fill).padding_bottom(4.0).font_family(fonts::MONO);
        let chart = chart.text_size(10.0).text_color(token::TEXT_3);
        let says = move |bar: &Bar| format!("{}{unit}", short(bar.value));
        match self.bars {
            Prop::Value(bars) => {
                let max = top(&bars);
                let rows = bars.into_iter().map(|bar| {
                    let (share, paint, says) = (bar.share(max), bar.paint(), says(&bar));
                    line(upright, bar.label, Prop::Value(share), Prop::Value(paint), Prop::Value(says))
                });
                chart.children(rows).mount(cx)
            }
            Prop::Reactive(read) => {
                let read: Rc<dyn Fn() -> Vec<Bar>> = Rc::from(read);
                let all = read.clone();
                let max = memo(move || top(&all()));
                let rows = each(
                    move || read(),
                    |bar: &Bar| bar.label.clone(),
                    move |bar: Signal<Bar>| {
                        let says = says.clone();
                        line(
                            upright,
                            bar.with_untracked(|bar| bar.label.clone()),
                            Prop::Reactive(Box::new(move || bar.with(|bar| bar.share(max.get())))),
                            Prop::Reactive(Box::new(move || bar.with(Bar::paint))),
                            Prop::Reactive(Box::new(move || bar.with(&says))),
                        )
                    },
                );
                rows.container(chart).mount(cx)
            }
        }
    }
}
