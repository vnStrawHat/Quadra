//! What one node paints of its own (specs/07): its box from its style, what its widget
//! emits, and the line along its edge that marks the focus.

use quadra_layout::{Display, LayoutTree};
use quadra_prim::{Color, Edges, Rect};
use quadra_render::{Clip, Instance, Quad, Shadow};

use crate::fonts::Fonts;
use crate::inherit::{TextAt, refresh};
use crate::node::Node;
use crate::widget::PaintCx;

/// The shadows of the levels above the window (specs/behaviour/15, "Elevation"): how far
/// down each one is, how blurred, and how black.
const SHADOWS: [&[(f32, f32, f32)]; 3] = [
    &[],
    &[(2.0, 3.0, 0.12), (1.0, 0.0, 0.06)],
    &[(2.0, 3.0, 0.12), (3.0, 6.0, 0.08), (6.0, 12.0, 0.04), (1.0, 0.0, 0.12)],
];

/// Paints one node's own instances: the background and border from its style, then its
/// widget. A node that is not displayed paints nothing. The caller has checked that the node
/// is in the band its layer paints.
pub(crate) fn paint_own(
    nodes: &mut [Node],
    index: usize,
    layout: &LayoutTree,
    rect: Rect,
    (scale, band, text_at): (f32, Option<Rect>, TextAt),
    fonts: &mut Fonts,
    out: &mut Vec<Instance>,
) {
    let style = layout.style(nodes[index].layout);
    if style.display == Display::None {
        return;
    }
    refresh(nodes, index, text_at);
    let node = &mut nodes[index];
    let keep = node.styled.as_ref().is_some_and(|styled| styled.has_variants());
    let mut cx = PaintCx { rect, scale, visible: band, clip: Clip::NONE, keep, fonts, out };
    let (background, border_color, radii, outline) =
        (node.paint.background, node.paint.border_color, node.paint.radii, node.paint.outline);
    // What is above the window casts its shadows first: they are under its own box.
    for &(down, blur, alpha) in SHADOWS[(node.paint.elevation as usize).min(2)] {
        let bounds = rect.translate(0.0, down);
        cx.shadow(Shadow { bounds, radii, blur, color: Color::BLACK.alpha(alpha) });
    }
    cx.quad(Quad {
        bounds: rect,
        background,
        border_widths: style.border,
        border_color,
        radii,
        ..Quad::default()
    });
    cx.keep = false;
    if let Some(widget) = &mut node.widget {
        widget.paint(&mut cx);
    }
    let edge = Quad {
        bounds: rect,
        border_widths: Edges::all(1.0),
        border_color: outline,
        radii,
        ..Quad::default()
    };
    cx.quad(edge);
}

/// Whether a node lies outside the part of its layer that is painted: it emits nothing.
pub(crate) fn outside(band: Option<Rect>, rect: Rect) -> bool {
    band.is_some_and(|band| !band.intersects(rect))
}
