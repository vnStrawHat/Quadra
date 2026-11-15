//! A formula on the screen: the widget that lays the nodes out at the size of the text
//! around it and paints them in its colour, and the view that mounts it.

use quadra_design::{fonts, token};
use quadra_layout::Known;
use quadra_prim::{Color, Point, Rect, Size};
use quadra_render::{Quad, stroke};
use quadra_text::TextStyle;
use quadra_tree::{
    GlyphTile, Inherited, InlineBox, MeasureCx, NodeId, PaintCx, Role, Semantics, Style, Widget, paint_glyph,
};
use quadra_ui::{MountCx, Span, View, div, inline, span, text};

use super::layout::{Laid, Lay, Part};
use super::{List, parse};

pub struct Formula {
    source: String,
    nodes: List,
    /// Set as a line of its own: larger operators, limits above and below them.
    display: bool,
    /// The size of the text around it, and its colour.
    size: f32,
    color: Color,
    /// The box, and the size it was laid out at.
    laid: Option<Laid>,
    laid_at: f32,
    /// The tiles last painted: holding them keeps them in the atlas.
    tiles: Vec<GlyphTile>,
}

impl Formula {
    pub fn new(source: &str, nodes: List, display: bool) -> Formula {
        let (source, tiles) = (source.to_string(), Vec::new());
        Formula { source, nodes, display, size: 16.0, color: Color::BLACK, laid: None, laid_at: 0.0, tiles }
    }
}

impl Widget for Formula {
    fn is_leaf(&self) -> bool {
        true
    }

    fn measure(&mut self, cx: &mut MeasureCx, _known: Known, _avail: Size) -> Size {
        let laid = self.lay(cx, self.size);
        Size::new(laid.width, laid.height + laid.depth)
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        let height = self.laid.as_ref().map_or(0.0, |laid| laid.height);
        self.paint_at(cx, Point::new(cx.rect.x, cx.rect.y + height), self.color);
    }

    fn inherit(&mut self, inherited: &Inherited) {
        (self.color, self.size) = (inherited.color, inherited.style.size);
    }

    fn semantics(&self, says: &mut Semantics) {
        if says.role == Role::None {
            says.role = Role::Text;
        }
        if says.name.is_empty() {
            says.name.clone_from(&self.source);
        }
    }
}

/// A formula in a line of rich text: on the baseline of the line, at the size of its span.
impl InlineBox for Formula {
    fn measure(&mut self, cx: &mut MeasureCx, style: &TextStyle) -> (f32, f32, f32) {
        let laid = self.lay(cx, style.size);
        (laid.width, laid.height, laid.depth)
    }

    fn paint(&mut self, cx: &mut PaintCx, origin: Point, color: Color) {
        self.paint_at(cx, origin, color);
    }

    fn says(&self) -> &str {
        &self.source
    }
}

impl Formula {
    /// The formula as a box at a size of text: laid out when the size is another.
    fn lay(&mut self, cx: &mut MeasureCx, size: f32) -> &Laid {
        if self.laid_at != size {
            (self.laid_at, self.laid) = (size, None);
        }
        let (nodes, style) = (&self.nodes, usize::from(!self.display));
        self.laid.get_or_insert_with(|| Lay::new(cx.text, size).list(nodes, style))
    }

    /// Paints the box last laid out, the left end of its baseline at `origin`.
    fn paint_at(&mut self, cx: &mut PaintCx, origin: Point, color: Color) {
        let Formula { laid: Some(laid), tiles, .. } = self else { return };
        let scale = cx.scale;
        tiles.clear();
        // Baselines and rules are on whole pixels; a rule that exists is a pixel at least.
        laid.each(origin.x, origin.y, &mut |x, y, part| match part {
            Part::Glyphs { word, size } => {
                let baseline = (y * scale).round();
                for g in &word.glyphs {
                    let pen = Point::new((x + g.x) * scale, baseline - (g.y * scale).round());
                    tiles.extend(paint_glyph(cx, (g.face, g.id), size * scale, pen, color));
                }
            }
            Part::Rule { width, height } => {
                let (left, top) = ((x * scale).round(), (y * scale).round());
                let bounds =
                    Rect::new(left, top, (width * scale).round().max(1.0), (height * scale).round().max(1.0));
                cx.push(Quad { bounds, background: color, ..Quad::default() }.instance(&cx.clip));
            }
            Part::Stroke { points, width } => {
                let points: Vec<Point> =
                    points.iter().map(|p| Point::new((x + p.x) * scale, (y + p.y) * scale)).collect();
                let clip = cx.clip;
                stroke(&points, (width * scale).max(1.0), color, false, &clip, |instance| cx.push(instance));
            }
            Part::Box(_) => {}
        });
    }
}

pub struct FormulaView {
    source: String,
    nodes: List,
    fault: Option<String>,
    display: bool,
}

/// A formula, from its source without the `$` around it, set as in a line of text. The
/// source is read here, once.
pub fn formula(source: &str) -> FormulaView {
    let (nodes, fault) = parse(source);
    FormulaView { source: source.to_string(), nodes, fault, display: false }
}

impl FormulaView {
    /// Set as a line of its own.
    pub fn display(mut self) -> Self {
        self.display = true;
        self
    }

    /// The formula as a span of rich text: in a line, on its baseline, at the size of the
    /// span. One with a fault is its source, in the colour of what is wrong.
    pub fn span(self) -> Span {
        match self.fault {
            None => inline(Formula::new(&self.source, self.nodes, false)),
            Some(_) => span(self.source).family(fonts::MONO).color(token::ERROR),
        }
    }

    /// What is wrong with the source, if anything is: the view then shows the source.
    pub fn fault(&self) -> Option<&str> {
        self.fault.as_deref()
    }
}

impl View for FormulaView {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        match self.fault {
            None => {
                let widget = Formula::new(&self.source, self.nodes, self.display);
                cx.tree.insert_widget(Style::default(), Box::new(widget))
            }
            // A formula with a fault shows its source, in the colour of what is wrong, and
            // says the fault to who asks what it is.
            Some(fault) => {
                let wrong = div().text_color(token::ERROR).font_family(fonts::MONO);
                wrong.label(format!("Formula: {fault}")).child(text(self.source)).mount(cx)
            }
        }
    }
}
