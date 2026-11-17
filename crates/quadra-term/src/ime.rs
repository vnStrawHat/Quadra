//! What an input method composes (specs/18, "Input"): drawn at the cursor over the cells it
//! covers, underlined, the clause being converted thicker; nothing goes to the program until
//! it is committed, as typed text.

use oneterm_vt::{SnapshotState, scalar_width};
use quadra_prim::{Point, Rect};
use quadra_render::Quad;
use quadra_tree::{NodeId, PaintCx, Tree, paint_glyph_with};
use quadra_view::MountCx;

use crate::paint::{cells, colors};
use crate::widget::{TerminalWidget, color};

/// Follows the composition: kept in the widget and painted, until it ends or is committed.
pub(crate) fn compose(cx: &mut MountCx, id: NodeId) {
    let writer = cx.handle.clone();
    cx.tree.on_composition(id, move |text, _, target| {
        let preedit = (!text.is_empty()).then(|| (text.to_string(), target));
        writer.with(move |tree| set(tree, id, preedit));
    });
    // What was composed is committed (the view's own handler sends it), or dropped.
    let writer = cx.handle.clone();
    cx.tree.on_text(id, move |_| writer.with(move |tree| set(tree, id, None)));
}

fn set(tree: &mut Tree, id: NodeId, preedit: Option<(String, std::ops::Range<usize>)>) {
    let Some(w) = tree.widget_mut::<TerminalWidget>(id) else { return };
    if w.preedit != preedit {
        w.preedit = preedit;
        tree.mark_paint(id); // the snapshot is as it was
    }
}

/// The composition at the cursor of `state`, the row's left at `left` and its top at `top`:
/// a cell for each character (two for a wide one; one of no width joins the one before), as
/// far as the row goes.
pub(crate) fn paint(
    w: &mut TerminalWidget,
    cx: &mut PaintCx,
    state: &SnapshotState,
    (left, top): (f32, f32),
) {
    let cursor = state.cursor();
    let Some(r) = cursor.row.filter(|_| w.focused && w.preedit.is_some()) else { return };
    let Some(row) = state.rows().get(usize::from(r)) else { return };
    let Some((text, target)) = w.preedit.take() else { return };
    let (first, end) = (usize::from(cursor.col), row.cells.len());
    let mut placed: Vec<(std::ops::Range<usize>, usize, usize)> = Vec::new();
    let mut col = first;
    for (i, c) in text.char_indices() {
        let width = usize::from(scalar_width(c));
        match placed.last_mut() {
            Some(last) if width == 0 => last.0.end = i + c.len_utf8(),
            _ if width == 0 => {}
            _ if col + width > end => break,
            _ => {
                placed.push((i..i + c.len_utf8(), col, width));
                col += width;
            }
        }
    }
    let at = (left, top + f32::from(r) * w.line);
    let behind =
        row.cells.get(first).map_or(color(w.palette.background), |cell| colors(w, row.style_of(cell)).1);
    cx.quad(Quad { bounds: cells(w, at, first, col - first), background: behind, ..Quad::default() });
    let (fg, scale) = (color(w.palette.foreground), cx.scale);
    let face = cx.fonts.text.face_for(&w.style);
    let (ascent, descent) = cx.fonts.text.line_metrics(&w.style);
    let baseline = ((at.1 + (w.line - ascent - descent) / 2.0 + ascent) * scale).round();
    for (bytes, col, cols) in placed {
        let b = cells(w, at, col, cols);
        let thick = if bytes.start < target.end && target.start < bytes.end { 2.0 } else { 1.0 };
        let underline = Rect::new(b.x, b.bottom() - 2.0, b.width, thick);
        cx.quad(Quad { bounds: underline, background: fg, ..Quad::default() });
        let word = cx.fonts.text.word(&text[bytes], face, w.style.size);
        let shift = (b.width - word.width) / 2.0;
        for g in &word.glyphs {
            let pen = Point::new((b.x + shift + g.x) * scale, baseline - (g.y * scale).round());
            w.tiles.extend(paint_glyph_with(cx, (g.face, g.id), w.style.size * scale, pen, fg, w.grayscale));
        }
    }
    w.preedit = Some((text, target));
}
