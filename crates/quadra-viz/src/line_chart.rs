//! The line chart (specs/13, "Charts"): views around a plot a series. They share the top of
//! the scale; the pointer over the plot writes a signal, which a hairline and a box of the
//! values there are bound to.

use std::rc::Rc;

use quadra_kit::{fonts, size, token};
use quadra_layout::{Align, Display, Justify, Length};
use quadra_reactive::{Memo, Signal, memo, signal, untrack};
use quadra_tree::{NodeId, Paint, PointerEvent, PointerKind, Role};
use quadra_ui::{Div, IntoProp, MountCx, Prop, View, column, div, row, text};

use crate::geometry::{hh_mm, max_of, nice_max, short};
use crate::plot::filling;
use crate::series_color;

/// What the values at the left and the times under the plot take, in pixels.
const AXIS_W: f32 = 44.0;
const AXIS_H: f32 = 20.0;

/// Reads the values of a series.
// ponytail: a read copies the values (a property gives no reference); a chart of very many
// samples wants its series as signals read in place.
type Values = Rc<dyn Fn() -> Vec<f64>>;

pub struct Series {
    name: String,
    values: Prop<Vec<f64>>,
    color: Option<Paint>,
}

impl Series {
    /// A line of the chart: `values` is a constant or something reactive.
    pub fn new<M>(name: impl Into<String>, values: impl IntoProp<Vec<f64>, M>) -> Series {
        Series { name: name.into(), values: values.into_prop(), color: None }
    }

    /// A colour of its own, or a role of the theme: its place in the series unless told.
    pub fn color(mut self, color: impl Into<Paint>) -> Self {
        self.color = Some(color.into());
        self
    }
}

pub struct LineChart {
    series: Vec<Series>,
    start_ms: i64,
    step_ms: i64,
    unit: Rc<str>,
    height: f32,
}

/// A chart of lines over the same samples, as wide as the box it is in and 200 px tall.
pub fn line_chart(series: impl IntoIterator<Item = Series>) -> LineChart {
    let series = series.into_iter().collect();
    LineChart { series, start_ms: 0, step_ms: 1000, unit: "".into(), height: 200.0 }
}

impl LineChart {
    /// When the first sample was taken, in milliseconds since 1970, and how far apart
    /// samples are: the times of the first, the middle and the last are under the plot.
    pub fn time(mut self, start_ms: i64, step_ms: i64) -> Self {
        (self.start_ms, self.step_ms) = (start_ms, step_ms);
        self
    }

    /// What follows a value in the box under the pointer: "m", " MB".
    pub fn unit(mut self, unit: &str) -> Self {
        self.unit = unit.into();
        self
    }

    pub fn height(mut self, px: f32) -> Self {
        self.height = px;
        self
    }
}

/// A property as what the plot takes and as something to read again.
fn shared(values: Prop<Vec<f64>>) -> (Prop<Vec<f64>>, Values) {
    match values {
        Prop::Value(values) => {
            let again = values.clone();
            (Prop::Value(values), Rc::new(move || again.clone()))
        }
        Prop::Reactive(read) => {
            let read: Values = Rc::from(read);
            let again = read.clone();
            (Prop::Reactive(Box::new(move || again())), read)
        }
    }
}

fn shown(shown: bool) -> Display {
    if shown { Display::Flex } else { Display::None }
}

/// What the chart knows of the pointer: the sample it is nearest to.
#[derive(Clone)]
struct Hover {
    at: Signal<Option<usize>>,
    /// How many samples the first series has.
    count: Memo<usize>,
    /// The time of a sample as text; nothing for a chart that was given no times.
    time: Option<Rc<dyn Fn(usize) -> String>>,
}

impl Hover {
    /// How far along the plot the sample under the pointer is, from 0 to 100.
    fn along(&self) -> f32 {
        let last = self.count.get().saturating_sub(1).max(1);
        self.at.get().unwrap_or(0) as f32 * 100.0 / last as f32
    }

    /// The hairline at the sample.
    fn hair(&self) -> Div {
        let (at, along) = (self.at, self.clone());
        let hair = div().absolute().top(0.0).bottom(0.0).width(1.0).bg(token::TEXT_3);
        hair.left(move || Length::Pct(along.along())).display(move || shown(at.get().is_some()))
    }

    /// The box of the values at the sample: at the right of the hairline in the left half
    /// of the plot, at its left in the right half.
    fn values(&self, lines: Vec<(String, Paint, Values)>, unit: Rc<str>) -> Div {
        let at = self.at;
        let plane = column().padding_y(5.0).padding_right(12.0).radius(size::RADIUS_MD).elevation(1);
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
        place.display(move || shown(at.get().is_some())).child(plane.children(rows))
    }
}

impl View for LineChart {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (start, step, alone) = (self.start_ms, self.step_ms, self.series.len() == 1);
        let names: Vec<&str> = self.series.iter().map(|series| series.name.as_str()).collect();
        let names = names.join(", ");
        let (mut plots, mut lines) = (Vec::new(), Vec::new());
        for (i, series) in self.series.into_iter().enumerate() {
            let paint = series.color.unwrap_or(series_color(i).into());
            let (values, read) = shared(series.values);
            plots.push((values, paint));
            lines.push((series.name, paint, read));
        }
        let all: Vec<Values> = lines.iter().map(|line| line.2.clone()).collect();
        let first = all.first().cloned();
        let count = memo(move || first.as_ref().map_or(0, |read| read().len()));
        let max = memo(move || nice_max(all.iter().map(|read| max_of(&read())).fold(0.0, f64::max)));
        let time = move |i: usize| hh_mm(start + i as i64 * step);
        let hover = Hover { at: signal(None), count, time: (start > 0).then(|| Rc::new(time) as _) };

        // The plot's box: the grid, a plot a series over it, and what follows the pointer.
        let grid = (0..5).map(|k| {
            let rule = div().absolute().left(0.0).right(0.0).top(Length::Pct(k as f32 * 25.0)).height(1.0);
            rule.bg(if k == 4 { token::LINE_STRONG } else { token::LINE })
        });
        let scale = (0..5).map(|k| {
            let value = text(move || short(max.get() * (4 - k) as f64 / 4.0));
            let label = row().absolute().right(Length::Pct(100.0)).top(Length::Pct(k as f32 * 25.0));
            label.height(0.0).padding_right(6.0).align_items(Align::Center).child(value)
        });
        let area = if alone { 0.12 } else { 0.0 };
        let plots = plots.into_iter().map(|(values, paint)| filling(values, max.into_prop(), paint, area));
        let at = hover.at;
        let field = div().absolute().left(AXIS_W).top(6.0).right(8.0).bottom(AXIS_H);
        let field = field.role(Role::Image).label(names).children(grid).children(scale).children(plots);
        let field = field.child(hover.hair()).child(hover.values(lines, self.unit));
        let field = field.on_pointer(PointerKind::Leave, move |_| at.set(None));
        let field = field.on_pointer(PointerKind::Move, move |e: &mut PointerEvent| {
            // The nearest sample: the same one again changes nothing, and costs no frame.
            let count = untrack(|| count.get());
            let near = (e.local.x / e.size.width.max(1.0)).clamp(0.0, 1.0) * count.saturating_sub(1) as f32;
            at.set((count > 0).then_some(near.round() as usize));
        });

        // The times of the first, the middle and the last sample.
        let times = (0..3).filter(|_| start > 0).map(|k| {
            text(move || match count.get() {
                0 | 1 => String::new(),
                n => time((n - 1) * k / 2),
            })
        });
        let times = row().absolute().left(AXIS_W).right(8.0).bottom(0.0).height(AXIS_H).children(times);
        let times = times.justify(Justify::Between).align_items(Align::Center);

        let chart = div().width(Length::Fill).height(self.height).shrink(0.0).font_family(fonts::MONO);
        let chart = chart.text_size(10.0).text_color(token::TEXT_3);
        chart.child(field).child(times).mount(cx)
    }
}
