//! A formula on the screen: the widget that lays the nodes out at the size of the text
//! around it and paints them in its colour, and the view that mounts it.

use std::hash::{DefaultHasher, Hash, Hasher};

use quadra_kit::{fonts, token};
use quadra_layout::Known;
use quadra_prim::{Color, Corners, Point, Rect, Size};
use quadra_render::{Kind, Quad, Sprite};
use quadra_tree::{
    GlyphTile, Inherited, MeasureCx, NodeId, PaintCx, Role, Semantics, Style, Widget, paint_glyph,
};
use quadra_ui::{MountCx, View, div, text};
use quadra_vector::{Coverage, Polyline};

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
    laid: Option<Laid>,
    /// The tiles last painted: holding them keeps them in the atlas.
    tiles: Vec<GlyphTile>,
}

impl Formula {
    pub fn new(source: &str, nodes: List, display: bool) -> Formula {
        let (source, tiles) = (source.to_string(), Vec::new());
        Formula { source, nodes, display, size: 16.0, color: Color::BLACK, laid: None, tiles }
    }
}

/// A line through points, in device pixels, as a mask in the atlas: the points from the
/// corner of the mask, to a sixteenth of a pixel, are what the atlas knows it by.
fn stroke(cx: &mut PaintCx, points: &[Point], width: f32, color: Color) -> Option<GlyphTile> {
    let reach = width / 2.0 + 1.0;
    let low = |of: fn(&Point) -> f32| points.iter().map(of).fold(f32::MAX, f32::min) - reach;
    let high = |of: fn(&Point) -> f32| points.iter().map(of).fold(f32::MIN, f32::max) + reach;
    let (left, top) = (low(|p| p.x).floor(), low(|p| p.y).floor());
    let (wide, tall) = ((high(|p| p.x) - left).ceil(), (high(|p| p.y) - top).ceil());
    if points.len() < 2 || !(wide * tall).is_finite() {
        return None;
    }
    let line = Polyline {
        points: points.iter().map(|p| Point::new(p.x - left, p.y - top)).collect(),
        closed: false,
    };
    let mut hasher = DefaultHasher::new();
    line.points.iter().for_each(|p| ((p.x * 16.0) as i32, (p.y * 16.0) as i32).hash(&mut hasher));
    ((width * 16.0) as i32).hash(&mut hasher);
    let (wide, tall) = (wide as u32, tall as u32);
    // ponytail: a stroke taller than a slice of the atlas is not painted; in pieces when a
    // formula is ever that tall.
    let tile = cx.fonts.mask(hasher.finish(), wide, tall, || {
        let mut coverage = Coverage::new(wide as usize, tall as usize);
        coverage.stroke(std::slice::from_ref(&line), width);
        coverage.to_bytes()
    })?;
    let sprite = Sprite {
        bounds: Rect::new(left, top, tile.rect.w as f32, tile.rect.h as f32),
        uv: tile.uv,
        slice: tile.rect.slice,
        color,
        radii: Corners::default(),
        glyph: true,
    };
    cx.push(sprite.instance(Kind::Mono, &cx.clip));
    Some(tile)
}

impl Widget for Formula {
    fn is_leaf(&self) -> bool {
        true
    }

    fn measure(&mut self, cx: &mut MeasureCx, _known: Known, _avail: Size) -> Size {
        let (nodes, style, size) = (&self.nodes, usize::from(!self.display), self.size);
        let laid = self.laid.get_or_insert_with(|| Lay::new(cx.text, size).list(nodes, style));
        Size::new(laid.width, laid.height + laid.depth)
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        let Formula { laid: Some(laid), tiles, color, .. } = self else { return };
        let (scale, color) = (cx.scale, *color);
        tiles.clear();
        // Baselines and rules are on whole pixels; a rule that exists is a pixel at least.
        laid.each(cx.rect.x, cx.rect.y + laid.height, &mut |x, y, part| match part {
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
                tiles.extend(stroke(cx, &points, (width * scale).max(1.0), color));
            }
            Part::Box(_) => {}
        });
    }

    fn inherit(&mut self, inherited: &Inherited) {
        self.color = inherited.color;
        if self.size != inherited.style.size {
            (self.size, self.laid) = (inherited.style.size, None);
        }
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
