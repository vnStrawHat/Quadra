//! Widgets: a node's behaviour (specs/04, "Widgets"), and the contexts its methods get.

use std::any::Any;

use quadra_a11y::Semantics;
use quadra_layout::{Holds, Known};
use quadra_prim::{Edges, Rect, Size};
use quadra_render::{Clip, Instance, Quad, Shadow, push_quad};
use quadra_text::TextSystem;

use crate::fonts::Fonts;
use crate::inherit::Inherited;
use crate::theme::Theme;

/// A node's behaviour. Every method has a default: a widget implements what it does. Most
/// nodes have no widget at all: a box is its style.
pub trait Widget: Any {
    /// True for a widget sized by its content (text, an image) and not by children.
    fn is_leaf(&self) -> bool {
        false
    }

    /// The content size of a leaf. `known` is what is already fixed per axis, `avail` the
    /// space left on each axis (infinite: no limit).
    fn measure(&mut self, _cx: &mut MeasureCx, _known: Known, _avail: Size) -> Size {
        Size::ZERO
    }

    /// For which available space the size of the last measure stays the answer (specs/06,
    /// "Measuring leaves"): `None`, the default, for only the space it was asked at.
    fn holds(&self) -> Option<Holds> {
        None
    }

    /// Emits the node's own instances, on top of the background the tree painted.
    fn paint(&mut self, _cx: &mut PaintCx) {}

    /// Takes the text values that hold at the node: what a widget lays out and paints its
    /// text with. The tree calls it before the widget is next measured or painted after
    /// those values may have changed, and not otherwise.
    fn inherit(&mut self, _inherited: &Inherited) {}

    /// Takes the theme: when the node is added, and when the tree gets another. A widget
    /// that paints with roles of the theme keeps their colours from it, and returns true
    /// when it must be painted again.
    fn theme(&mut self, _theme: &Theme) -> bool {
        false
    }

    /// Completes what the node says of itself to a user: a text names itself by its text.
    fn semantics(&self, _says: &mut Semantics) {}

    /// Where text would be typed, from the node's top-left corner, for a widget that takes
    /// text: an input method opens beside it.
    fn caret(&self) -> Option<Rect> {
        None
    }
}

pub struct MeasureCx<'a> {
    pub text: &'a mut TextSystem,
}

/// Where a widget paints. Rectangles are logical pixels in the layer's space; the context
/// turns them into device pixels, the only place that happens (specs/07).
pub struct PaintCx<'a> {
    /// The node's border box.
    pub rect: Rect,
    /// Device pixels per logical pixel.
    pub scale: f32,
    /// The part of the layer that is painted, when it is not all of it: what lies outside
    /// need not be emitted (a text skips its lines there).
    pub visible: Option<Rect>,
    pub clip: Clip,
    /// A quad that cannot be seen is emitted all the same: a node whose look follows its
    /// states keeps its place in the layer, and a state that fills it overwrites it there.
    pub(crate) keep: bool,
    /// Where glyph tiles come from.
    pub fonts: &'a mut Fonts,
    pub(crate) out: &'a mut Vec<Instance>,
}

fn snap(v: f32) -> f32 {
    v.round()
}

/// A stroke that exists is at least one device pixel wide.
fn snap_stroke(v: f32) -> f32 {
    if v > 0.0 { v.round().max(1.0) } else { 0.0 }
}

impl PaintCx<'_> {
    /// A logical rectangle in device pixels, its edges on whole pixels: a fractional layout
    /// never draws a blurred edge or leaves a seam between neighbours.
    pub fn device(&self, r: Rect) -> Rect {
        let (x, y) = (snap(r.x * self.scale), snap(r.y * self.scale));
        let (right, bottom) = (snap(r.right() * self.scale), snap(r.bottom() * self.scale));
        Rect::new(x, y, (right - x).max(0.0), (bottom - y).max(0.0))
    }

    /// Paints a quad given in logical pixels.
    pub fn quad(&mut self, q: Quad) {
        // Most boxes are only there to hold others: nothing is worked out for them.
        if !self.keep && !q.is_visible() {
            return;
        }
        let b = q.border_widths.scale(self.scale);
        let border_widths = Edges {
            top: snap_stroke(b.top),
            right: snap_stroke(b.right),
            bottom: snap_stroke(b.bottom),
            left: snap_stroke(b.left),
        };
        let (bounds, radii, backdrop) =
            (self.device(q.bounds), q.radii.scale(self.scale), q.backdrop * self.scale);
        let quad = Quad { bounds, border_widths, radii, backdrop, ..q };
        let before = self.out.len();
        push_quad(self.out, &quad, &self.clip);
        if self.keep && self.out.len() == before {
            self.out.push(quad.instance(&self.clip)); // nothing to see, and a place kept
        }
    }

    /// Paints a shadow given in logical pixels.
    pub fn shadow(&mut self, shadow: Shadow) {
        let (bounds, radii) = (self.device(shadow.bounds), shadow.radii.scale(self.scale));
        let shadow = Shadow { bounds, radii, blur: shadow.blur * self.scale, ..shadow };
        self.out.push(shadow.instance(&self.clip));
    }

    /// Appends an instance that is already in device pixels.
    pub fn push(&mut self, instance: Instance) {
        self.out.push(instance);
    }
}
