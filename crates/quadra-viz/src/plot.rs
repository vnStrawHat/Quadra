//! The plot (specs/13, "Charts"): a line over a box, drawn a device pixel column at a time,
//! and the sparkline, which is a plot by itself.

use quadra_design::token;
use quadra_layout::{Known, Length, Position};
use quadra_prim::{Color, Rect, Size};
use quadra_render::{Line, LineMode, Quad};
use quadra_tree::{MeasureCx, NodeId, Paint, PaintCx, Role, Style, Theme, Tree, Widget};
use quadra_ui::{IntoProp, MountCx, Prop, View};

use crate::geometry::{line_spans, max_of};

/// How thick a line is, in logical pixels.
const THICK: f32 = 1.5;

/// The widget: values spread over the width of its box, the value `top` at the box's top.
pub struct Plot {
    values: Vec<f64>,
    /// 0: 1.1 times the largest value.
    top: f64,
    paint: Paint,
    color: Color,
    /// How opaque the area under the line is; 0 is no area.
    area: f32,
}

impl Plot {
    pub fn new(values: Vec<f64>, paint: Paint, area: f32) -> Plot {
        Plot { values, top: 0.0, paint, color: Color::TRANSPARENT, area }
    }

    /// Replaces the values; false when they are equal. The caller has the node painted.
    pub fn set_values(&mut self, values: Vec<f64>) -> bool {
        std::mem::replace(&mut self.values, values) != self.values
    }

    /// Says which value is at the top of the box; false when it was.
    pub fn set_top(&mut self, top: f64) -> bool {
        std::mem::replace(&mut self.top, top) != self.top
    }
}

impl Widget for Plot {
    fn is_leaf(&self) -> bool {
        true
    }

    fn measure(&mut self, _cx: &mut MeasureCx, known: Known, _avail: Size) -> Size {
        Size::new(known.width.unwrap_or(0.0), known.height.unwrap_or(0.0))
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        // Whole device pixel columns on the pixel grid, and each tells the shader how tall
        // the line is in it and how far it falls across it: the shader works out how much
        // of each pixel the line covers, so a slope has no steps. The line's own heights
        // stay fractions of a pixel: rounding them is what makes steps.
        let (s, rect) = (cx.scale, cx.rect);
        let (left, top) = ((rect.x * s).round(), rect.y * s);
        let columns = ((rect.right() * s).round() - left).max(0.0) as usize;
        let (height, floor) = (rect.height * s, (rect.bottom() * s).round());
        let max = if self.top > 0.0 { self.top } else { max_of(&self.values) * 1.1 };
        let spans = line_spans(&self.values, columns, height, max, THICK * s);
        // Every column emits its instances whatever it holds: values that change overwrite
        // them in place.
        let area = self.color.alpha(self.color.a * self.area);
        for (x, span) in spans.iter().enumerate().filter(|_| self.area > 0.0) {
            let from = (top + span.y).round().min(floor);
            let bounds = Rect::new(left + x as f32, from, 1.0, floor - from);
            cx.push(Quad { bounds, background: area, ..Quad::default() }.instance(&cx.clip));
        }
        for (x, span) in spans.iter().enumerate() {
            // A pixel more above and below: the edges of the line are in it.
            let bounds = Rect::new(left + x as f32, top + span.top - 1.0, 1.0, span.bottom - span.top + 2.0);
            let mode = LineMode::Column { rise: span.bottom - span.top - span.thick };
            cx.push(Line { bounds, color: self.color, thickness: span.thick, mode }.instance(&cx.clip));
        }
    }

    fn theme(&mut self, theme: &Theme) -> bool {
        std::mem::replace(&mut self.color, theme.resolve(self.paint)) != self.color
    }
}

/// A plot as a view: where it is laid out, and what its widget is given and follows.
pub(crate) struct PlotView {
    pub layout: quadra_layout::Style,
    pub values: Prop<Vec<f64>>,
    pub top: Prop<f64>,
    pub paint: Paint,
    pub area: f32,
}

/// Writes something into a mounted plot, and has it painted when that changed it.
fn write(tree: &mut Tree, id: NodeId, f: impl FnOnce(&mut Plot) -> bool) {
    if tree.widget_mut::<Plot>(id).is_some_and(f) {
        tree.mark_paint(id);
    }
}

impl View for PlotView {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let style = Style { layout: self.layout, ..Style::default() };
        let id = cx.tree.insert_widget(style, Box::new(Plot::new(Vec::new(), self.paint, self.area)));
        match self.values {
            Prop::Value(values) => write(cx.tree, id, |plot| plot.set_values(values)),
            Prop::Reactive(read) => {
                let writer = cx.handle.clone();
                cx.handle.bind(move || {
                    let values = read();
                    writer.with(move |tree| write(tree, id, |plot| plot.set_values(values)));
                });
            }
        }
        match self.top {
            Prop::Value(top) => write(cx.tree, id, |plot| plot.set_top(top)),
            Prop::Reactive(read) => {
                let writer = cx.handle.clone();
                cx.handle.bind(move || {
                    let top = read();
                    writer.with(move |tree| write(tree, id, |plot| plot.set_top(top)));
                });
            }
        }
        id
    }
}

/// A plot that fills the box it is in, over what else is there.
pub(crate) fn filling(values: Prop<Vec<f64>>, top: Prop<f64>, paint: Paint, area: f32) -> PlotView {
    let layout = quadra_layout::Style {
        position: Position::Absolute,
        width: Length::Fill,
        height: Length::Fill,
        ..Default::default()
    };
    PlotView { layout, values, top, paint, area }
}

/// A trend in a line of text: a line over a faint area.
pub struct Sparkline(PlotView, String);

/// A sparkline of values, 120 by 24 px unless told, in the first colour of the series.
pub fn sparkline<M>(values: impl IntoProp<Vec<f64>, M>) -> Sparkline {
    let (width, height) = (Length::Px(120.0), Length::Px(24.0));
    let layout = quadra_layout::Style { width, height, shrink: 0.0, ..Default::default() };
    let (values, top, paint) = (values.into_prop(), Prop::Value(0.0), token::SERIES[0].into());
    Sparkline(PlotView { layout, values, top, paint, area: 0.12 }, String::new())
}

impl Sparkline {
    pub fn size(mut self, width: f32, height: f32) -> Self {
        (self.0.layout.width, self.0.layout.height) = (Length::Px(width), Length::Px(height));
        self
    }

    /// A colour, or a role of the theme.
    pub fn color(mut self, color: impl Into<Paint>) -> Self {
        self.0.paint = color.into();
        self
    }

    /// The value at the top of the box: 1.1 times the largest one unless told.
    pub fn max<M>(mut self, top: impl IntoProp<f64, M>) -> Self {
        self.0.top = top.into_prop();
        self
    }

    /// What a reader calls the picture: "CPU of api".
    pub fn label(mut self, name: impl Into<String>) -> Self {
        self.1 = name.into();
        self
    }
}

impl View for Sparkline {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let id = self.0.mount(cx);
        cx.tree.update_semantics(id, |says| (says.role, says.name) = (Role::Image, self.1));
        id
    }
}
