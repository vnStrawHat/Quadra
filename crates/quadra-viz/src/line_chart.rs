//! The line chart (specs/13, "Charts"): views around a plot a series. They share the top of
//! the scale; the pointer over the plot writes a signal, which a hairline and a box of the
//! values there are bound to.

mod hover;

use std::rc::Rc;

use quadra_design::{fonts, token};
use quadra_layout::{Align, Display, Justify, Length};
use quadra_reactive::{memo, signal, untrack};
use quadra_tree::{BorderStyle, NodeId, Paint, PointerEvent, PointerKind, Role};
use quadra_view::{Div, IntoProp, MountCx, Prop, View, column, div, row, text};

use self::hover::Hover;
use crate::geometry::{hh_mm, max_of, nice_max, short};
use crate::plot::{PlotView, Read, Values, filling, reader, shared};
use crate::series_color;

/// What the values at the left and the times under the plot take, in pixels.
const AXIS_W: f32 = 44.0;
const AXIS_H: f32 = 20.0;

/// A limit: its name and its value.
type Limit = (String, Read<f64>);

pub struct Series {
    name: String,
    values: Prop<Vec<f64>>,
    color: Option<Paint>,
    dashed: bool,
}

impl Series {
    /// A line of the chart: `values` is a constant or something reactive.
    pub fn new<M>(name: impl Into<String>, values: impl IntoProp<Vec<f64>, M>) -> Series {
        Series { name: name.into(), values: values.into_prop(), color: None, dashed: false }
    }

    /// A colour of its own, or a role of the theme: its place in the series unless told.
    pub fn color(mut self, color: impl Into<Paint>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Its line in dashes, so it reads apart from a solid one without colour.
    pub fn dashed(mut self) -> Self {
        self.dashed = true;
        self
    }
}

pub struct LineChart {
    series: Vec<Series>,
    start_ms: Option<Prop<i64>>,
    step_ms: i64,
    limit: Option<(String, Prop<f64>)>,
    unit: Rc<str>,
    height: f32,
    legend: bool,
    note: Option<Read<String>>,
    bare: bool,
    offset_ms: i64,
    top: Option<f64>,
}

/// A chart of lines over the same samples, as wide as the box it is in and 200 px tall.
pub fn line_chart(series: impl IntoIterator<Item = Series>) -> LineChart {
    let series = series.into_iter().collect();
    let (start_ms, limit, unit, note, top) = (None, None, "".into(), None, None);
    let (step_ms, height, legend, bare, offset_ms) = (1000, 200.0, false, false, 0);
    LineChart { series, start_ms, step_ms, limit, unit, height, legend, note, bare, offset_ms, top }
}

impl LineChart {
    /// When the first sample was taken, in milliseconds since 1970, and how far apart
    /// samples are: the times of the first, the middle and the last are under the plot.
    /// `start_ms` is a constant or something reactive: a live chart's first sample.
    pub fn time<M>(mut self, start_ms: impl IntoProp<i64, M>, step_ms: i64) -> Self {
        (self.start_ms, self.step_ms) = (Some(start_ms.into_prop()), step_ms);
        self
    }

    /// A dashed line at `value` across the plot, named `name` where the values are said; the
    /// scale reaches it. `value` is a constant or something reactive.
    pub fn limit<M>(mut self, name: impl Into<String>, value: impl IntoProp<f64, M>) -> Self {
        self.limit = Some((name.into(), value.into_prop()));
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

    /// The name of each series beside a swatch of its colour, in a row under the chart; then
    /// the limit's, beside a short dashed line.
    pub fn legend(mut self) -> Self {
        self.legend = true;
        self
    }

    /// Muted mono words at the right end of the legend's row ("since 09:12"), which shows.
    pub fn legend_note(mut self, words: impl Fn() -> String + 'static) -> Self {
        (self.note, self.legend) = (Some(Rc::new(words) as Read<String>), true);
        self
    }

    /// No values at the left and no times under it: the plot fills the box, and what is
    /// said of it is in the box under the pointer (a small chart beside its numbers).
    pub fn bare(mut self) -> Self {
        self.bare = true;
        self
    }

    /// The seconds the user's clock is ahead of UTC (`quadra_app::utc_offset()`): the times
    /// are written in local time.
    pub fn offset(mut self, seconds: i64) -> Self {
        self.offset_ms = seconds * 1000;
        self
    }

    /// A fixed top of the scale (100 for percentages) instead of one rounded up from the
    /// values.
    pub fn top(mut self, value: f64) -> Self {
        self.top = Some(value);
        self
    }
}

/// A swatch of a colour and a name after it.
fn named(name: &str, paint: Paint) -> Div {
    let swatch = div().size(8.0, 8.0).shrink(0.0).radius(2.0).bg(paint);
    row().shrink(0.0).align_items(Align::Center).gap(6.0).child(swatch).child(text(name.to_string()))
}

/// The swatch of a limit or a dashed series, a short dashed line, and its name after it.
fn dashed(name: &str, paint: Paint) -> Div {
    let swatch = div().width(12.0).height(1.0).shrink(0.0).border_top(1.0).border_color(paint);
    let swatch = swatch.border_style(BorderStyle::Dashed);
    row().shrink(0.0).align_items(Align::Center).gap(6.0).child(swatch).child(text(name.to_string()))
}

/// What a reader is told of the lines: each series, its least and its most, and its last;
/// then the limit.
fn summary(lines: &[(String, Paint, Values)], limit: &Option<Limit>, unit: &str) -> String {
    let says = |(name, _, read): &(String, Paint, Values)| {
        let values = read();
        let least = values.iter().copied().fold(f64::INFINITY, f64::min);
        match values.last() {
            Some(last) => {
                let (least, most, last) = (short(least), short(max_of(&values)), short(*last));
                format!("{name}: {least} to {most}{unit}, last {last}{unit}")
            }
            None => format!("{name}: no values"),
        }
    };
    let limit = limit.iter().map(|(name, value)| format!("{name} {}{unit}", short(value())));
    lines.iter().map(says).chain(limit).collect::<Vec<_>>().join("; ")
}

fn shown(shown: bool) -> Display {
    if shown { Display::Flex } else { Display::None }
}

impl View for LineChart {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (start, step, alone) = (self.start_ms.map(reader), self.step_ms, self.series.len() == 1);
        let limit: Option<Limit> = self.limit.map(|(name, value)| (name, reader(value)));
        let (mut plots, mut lines) = (Vec::new(), Vec::new());
        for (i, series) in self.series.into_iter().enumerate() {
            let paint = series.color.unwrap_or(series_color(i));
            let (values, read) = shared(series.values);
            plots.push((values, paint, series.dashed));
            lines.push((series.name, paint, read));
        }
        let legend = self.legend.then(|| {
            let legend = row().wrap(true).gap(16.0).padding_top(8.0).align_items(Align::Center);
            let swatch = |(l, p): (&(String, Paint, Values), &(_, _, bool))| match p.2 {
                true => dashed(&l.0, l.1),
                false => named(&l.0, l.1),
            };
            let legend = legend.children(lines.iter().zip(&plots).map(swatch));
            let legend = legend.children(limit.iter().map(|(name, _)| dashed(name, token::TEXT_3.into())));
            let note = self
                .note
                .map(|note| row().font_family(fonts::MONO).child(text(move || note()).color(token::TEXT_3)));
            match note {
                Some(note) => legend.child(div().grow(1.0)).child(note),
                None => legend,
            }
        });
        let (said, limited, unit) = (lines.clone(), limit.clone(), self.unit.clone());
        let names = move || summary(&said, &limited, &unit);
        let all: Vec<Values> = lines.iter().map(|line| line.2.clone()).collect();
        let first = all.first().cloned();
        let count = memo(move || first.as_ref().map_or(0, |read| read().len()));
        // The top reaches the limit, however far under it the series are.
        let (reach, top) = (limit.as_ref().map(|(_, value)| value.clone()), self.top);
        let max = memo(move || {
            let most = all.iter().map(|read| max_of(&read())).fold(0.0, f64::max);
            top.unwrap_or_else(|| nice_max(reach.as_ref().map_or(most, |value| most.max(value()))))
        });
        // The time of a sample, from the first sample's as it is now.
        let offset = self.offset_ms;
        let time = start.map(|start| {
            Rc::new(move |i: usize| hh_mm(start() + i as i64 * step + offset)) as Rc<dyn Fn(usize) -> String>
        });
        let hover = Hover { at: signal(None), count, time: time.clone() };

        // The plot's box: the grid, a plot a series over it, and what follows the pointer.
        // Bare, it has no grid: one rule, faint, along its bottom.
        let bare = self.bare;
        let grid = (0..5).filter(|&k| !bare || k == 4).map(move |k| {
            let rule = div().absolute().left(0.0).right(0.0).top(Length::Pct(k as f32 * 25.0)).height(1.0);
            rule.bg(if k == 4 && !bare { token::LINE_STRONG } else { token::LINE })
        });
        // A unit that is a sign follows the scale's values; a word would crowd them.
        let sign: Rc<str> =
            if self.unit.chars().any(char::is_alphabetic) { "".into() } else { self.unit.clone() };
        let scale = (0..5).map(|k| {
            let sign = sign.clone();
            let value = text(move || format!("{}{sign}", short(max.get() * (4 - k) as f64 / 4.0)));
            let label = row().absolute().right(Length::Pct(100.0)).top(Length::Pct(k as f32 * 25.0));
            label.height(0.0).padding_right(6.0).align_items(Align::Center).child(value)
        });
        // The limit: a dashed line at its height, over the grid and under the plots.
        let rule = limit.as_ref().map(|(_, value)| {
            let value = value.clone();
            let top =
                move || Length::Pct(((1.0 - value() / max.get().max(f64::MIN_POSITIVE)) * 100.0) as f32);
            let rule = div().absolute().left(0.0).right(0.0).top(top).height(1.0).border_top(1.0);
            rule.border_color(token::TEXT_3).border_style(BorderStyle::Dashed)
        });
        let area = if alone { 0.12 } else { 0.0 };
        let plots = plots.into_iter().map(|(values, paint, dashed)| PlotView {
            dashed,
            ..filling(values, max.into_prop(), paint, area)
        });
        let at = hover.at;
        // Bare, the plot fills the box: no scale, no times.
        let (left, top, right, bottom) = if bare { (0.0, 0.0, 0.0, 0.0) } else { (AXIS_W, 6.0, 8.0, AXIS_H) };
        let field = div().absolute().left(left).top(top).right(right).bottom(bottom);
        let scale: Vec<Div> = if bare { Vec::new() } else { scale.collect() };
        let field = field.role(Role::Image).label(names).children(grid).children(scale).children(rule);
        let field = field.children(plots).child(hover.hair()).child(hover.values(lines, limit, self.unit));
        let field = field.on_pointer(PointerKind::Leave, move |_| at.set(None));
        let field = field.on_pointer(PointerKind::Move, move |e: &mut PointerEvent| {
            // The nearest sample: the same one again changes nothing, and costs no frame.
            let count = untrack(|| count.get());
            let near = (e.local.x / e.size.width.max(1.0)).clamp(0.0, 1.0) * count.saturating_sub(1) as f32;
            at.set((count > 0).then_some(near.round() as usize));
        });

        // The times of the first, the middle and the last sample.
        let times = time.into_iter().filter(|_| !bare).flat_map(|time| {
            (0..3).map(move |k| {
                let time = time.clone();
                text(move || match count.get() {
                    0 | 1 => String::new(),
                    n => time((n - 1) * k / 2),
                })
            })
        });
        let times = row().absolute().left(AXIS_W).right(8.0).bottom(0.0).height(AXIS_H).children(times);
        let times = times.justify(Justify::Between).align_items(Align::Center);

        let chart = div().width(Length::Fill).height(self.height).shrink(0.0).font_family(fonts::MONO);
        let chart = chart.text_size(10.0).text_color(token::TEXT_3).child(field).child(times);
        match legend {
            Some(legend) => column().width(Length::Fill).child(chart).child(legend.text_size(12.0)).mount(cx),
            None => chart.mount(cx),
        }
    }
}
