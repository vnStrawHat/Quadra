//! Painting the terminal (specs/18, "The view"): the snapshot is taken under the lock, then
//! each row that shows is drawn from it alone: runs of background, the selection, the cursor,
//! a glyph per cell at its column, underline and strike. A glyph's colour keeps the look's
//! minimum contrast against its cell's background.

use oneterm_vt::{Attrs, Color as VtColor, CursorShape, Pos, SnapshotContent, SnapshotRow, Style};
use quadra_prim::{Color, Edges, Point, Rect};
use quadra_render::Quad;
use quadra_tree::{PaintCx, paint_glyph};

use crate::widget::{TerminalWidget, color};

/// The colours a cell's style paints with: inverse swaps them.
fn colors(w: &TerminalWidget, style: &Style) -> (Color, Color) {
    let (fg, bg) = (color(w.palette.resolve(style.fg)), color(w.palette.resolve(style.bg)));
    if style.attrs.contains(Attrs::INVERSE) { (bg, fg) } else { (fg, bg) }
}

/// A glyph's colour: its style's, moved to the look's contrast unless the program named it
/// exactly or the character is a drawing (specs/18, "The view"); faint is it at 60 %.
fn glyph_color(w: &mut TerminalWidget, style: &Style, c: Option<char>) -> Color {
    let named = if style.attrs.contains(Attrs::INVERSE) { style.bg } else { style.fg };
    let exact = matches!(named, VtColor::Rgb(_) | VtColor::Palette(16..));
    let drawing = c.is_some_and(|c| matches!(c as u32, 0x2500..=0x25FF | 0xE0B0..=0xE0D7));
    let (fg, bg) = colors(w, style);
    let fg = if exact || drawing { fg } else { w.contrast.fg(fg, bg) };
    if style.attrs.contains(Attrs::DIM) { fg.alpha(0.6) } else { fg }
}

pub(crate) fn paint(w: &mut TerminalWidget, cx: &mut PaintCx) {
    w.scale = cx.scale;
    let background = color(w.palette.background);
    cx.quad(Quad { bounds: cx.rect, background, ..Quad::default() });
    let (left, top) = (cx.rect.x + w.gutter_width() + w.pad.left, cx.rect.y + w.pad.top);
    w.tiles.clear();
    crate::gutter::paint(w, cx, top);
    let block = w.cursor_block();
    let rows = std::mem::take(&mut w.state);
    for (r, row) in rows.rows().iter().enumerate() {
        let y = top + r as f32 * w.line;
        under(w, cx, row, &rows, (left, y), r, background);
        text(w, cx, row, (left, y), block);
    }
    w.state = rows;
}

/// The cell box of `cols` columns from `col` in the row at `y`.
fn cells(w: &TerminalWidget, (left, y): (f32, f32), col: usize, cols: usize) -> Rect {
    Rect::new(left + col as f32 * w.cell, y, cols as f32 * w.cell, w.line)
}

/// Backgrounds that are not the terminal's, the selection, and the cursor.
fn under(
    w: &TerminalWidget,
    cx: &mut PaintCx,
    row: &SnapshotRow,
    state: &oneterm_vt::SnapshotState,
    at: (f32, f32),
    r: usize,
    background: Color,
) {
    for run in &row.runs {
        let (_, bg) = colors(w, &run.style);
        if bg != background {
            let (from, to) = (run.cols.start as usize, run.cols.end as usize);
            cx.quad(Quad { bounds: cells(w, at, from, to - from), background: bg, ..Quad::default() });
        }
    }
    if let Some(selection) = state.selection() {
        let chosen = |col: usize| selection.contains(Pos { row: row.id, col: col as u16 });
        let mut col = 0;
        while col < row.cells.len() {
            let start = col;
            while col < row.cells.len() && chosen(col) {
                col += 1;
            }
            if col > start {
                let bounds = cells(w, at, start, col - start);
                cx.quad(Quad { bounds, background: w.selection, ..Quad::default() });
            }
            col += 1;
        }
    }
    let cursor = state.cursor();
    let shown = cursor.visible && cursor.row == Some(r as u16) && (w.caret_on || !w.cursor_style.blinking);
    if !shown {
        return;
    }
    let wide = row.cells.get(cursor.col as usize).is_some_and(|c| c.width == oneterm_vt::CellWidth::Wide);
    let b = cells(w, at, cursor.col as usize, if wide { 2 } else { 1 });
    let ink = color(w.palette.cursor);
    let quad = match w.cursor_style.shape {
        CursorShape::Hidden => return,
        CursorShape::Block if w.focused => Quad { bounds: b, background: ink, ..Quad::default() },
        CursorShape::Block | CursorShape::HollowBlock => {
            Quad { bounds: b, border_widths: Edges::all(1.0), border_color: ink, ..Quad::default() }
        }
        CursorShape::Beam => {
            Quad { bounds: Rect::new(b.x, b.y, 2.0, b.height), background: ink, ..Quad::default() }
        }
        _ => Quad {
            bounds: Rect::new(b.x, b.bottom() - 2.0, b.width, 2.0),
            background: ink,
            ..Quad::default()
        },
    };
    cx.quad(quad);
}

/// A glyph a cell, in the face of its style, in the middle of its cells; then its lines.
fn text(
    w: &mut TerminalWidget,
    cx: &mut PaintCx,
    row: &SnapshotRow,
    at: (f32, f32),
    cursor: Option<(oneterm_vt::RowId, u16)>,
) {
    let scale = cx.scale;
    let (ascent, descent) = cx.fonts.text.line_metrics(&w.style);
    let baseline = ((at.1 + (w.line - ascent - descent) / 2.0 + ascent) * scale).round();
    let mut buf = String::new();
    for (col, cell) in row.cells.iter().enumerate() {
        if cell.width.is_spacer() {
            continue;
        }
        let style = row.style_of(cell);
        let blank = matches!(cell.content, SnapshotContent::Scalar(' '));
        if blank && !style.attrs.intersects(Attrs::ALL_UNDERLINES | Attrs::STRIKEOUT) {
            continue; // nothing to draw, so no colour to work out
        }
        let first = match cell.content {
            SnapshotContent::Scalar(c) => Some(c),
            SnapshotContent::Cluster { .. } => None,
        };
        let mut fg = glyph_color(w, style, first);
        if cursor == Some((row.id, col as u16)) {
            fg = color(w.palette.background); // on the block, the text is the background's
        }
        let cols = if cell.width == oneterm_vt::CellWidth::Wide { 2 } else { 1 };
        let b = cells(w, at, col, cols);
        let lines =
            [(Attrs::ALL_UNDERLINES, b.bottom() - 2.0), (Attrs::STRIKEOUT, b.y + (w.line / 2.0).round())];
        for (attr, y) in lines {
            if style.attrs.intersects(attr) {
                let bounds = Rect::new(b.x, y, b.width, 1.0);
                cx.quad(Quad { bounds, background: fg, ..Quad::default() });
            }
        }
        buf.clear();
        match cell.content {
            SnapshotContent::Scalar(' ') => continue,
            SnapshotContent::Scalar(c) => buf.push(c),
            SnapshotContent::Cluster { start, len } => buf.extend(row.cluster(start, len)),
        }
        if style.attrs.contains(Attrs::HIDDEN) {
            continue;
        }
        let mut face_style = w.style.clone();
        face_style.weight = if style.attrs.contains(Attrs::BOLD) { 700 } else { face_style.weight };
        face_style.italic = style.attrs.contains(Attrs::ITALIC);
        let face = cx.fonts.text.face_for(&face_style);
        let word = cx.fonts.text.word(&buf, face, w.style.size);
        let shift = (b.width - word.width) / 2.0;
        for g in &word.glyphs {
            let pen = Point::new((b.x + shift + g.x) * scale, baseline - (g.y * scale).round());
            w.tiles.extend(paint_glyph(cx, (g.face, g.id), w.style.size * scale, pen, fg));
        }
    }
}

impl TerminalWidget {
    /// The cell a filled block cursor covers, when one shows.
    fn cursor_block(&self) -> Option<(oneterm_vt::RowId, u16)> {
        let cursor = self.state.cursor();
        let on = self.focused && cursor.visible && (self.caret_on || !self.cursor_style.blinking);
        (on && cursor.row.is_some() && self.cursor_style.shape == CursorShape::Block)
            .then_some((cursor.id, cursor.col))
    }
}
