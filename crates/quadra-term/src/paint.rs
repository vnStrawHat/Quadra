//! Painting the terminal (specs/18, "The view"): the snapshot is taken under the lock, then
//! each row that shows is drawn from it alone: runs of background, the selection, the cursor,
//! a glyph per cell at its column (or a token's glyphs on their cells, with ligatures),
//! underline and strike. A glyph's colour is its class's when the program left it the default
//! (semantic highlighting), and keeps the look's minimum contrast against its background.

use oneterm_vt::{Attrs, Color as VtColor, CursorShape, Pos, SnapshotContent, SnapshotRow, Style};
use quadra_prim::{Color, Edges, Point, Rect};
use quadra_render::Quad;
use quadra_text::FaceId;
use quadra_tree::{PaintCx, paint_glyph};

use crate::highlight::{Highlight, plain};
use crate::shape::Scratch;
use crate::widget::{TerminalWidget, color};

/// The colours a cell's style paints with: inverse swaps them.
fn colors(w: &TerminalWidget, style: &Style) -> (Color, Color) {
    let (fg, bg) = (color(w.palette.resolve(style.fg)), color(w.palette.resolve(style.bg)));
    if style.attrs.contains(Attrs::INVERSE) { (bg, fg) } else { (fg, bg) }
}

/// A glyph's colour: its class's when it has one (the program left it the default), else its
/// style's; moved to the look's contrast unless the program named it exactly or the character
/// is a drawing (specs/18, "The view"); faint is it at 60 %.
fn glyph_color(w: &mut TerminalWidget, style: &Style, c: Option<char>, class: Option<Highlight>) -> Color {
    let named = if style.attrs.contains(Attrs::INVERSE) { style.bg } else { style.fg };
    let exact = matches!(named, VtColor::Rgb(_) | VtColor::Palette(16..));
    let drawing = c.is_some_and(|c| matches!(c as u32, 0x2500..=0x25FF | 0xE0B0..=0xE0D7));
    let (mut fg, bg) = colors(w, style);
    if let Some(class) = class {
        fg = w.colors.highlights[class.index()].unwrap_or_else(|| color(w.palette.indexed[class.ansi()]));
    }
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
    let cursor = (w.cursor_block(), w.cursor_cell());
    let rows = std::mem::take(&mut w.state);
    let mut marks = std::mem::take(&mut w.marks);
    let tint = if w.highlighting {
        marks.update(rows.rows());
        w.status.filter(|_| rows.scroll_offset() == 0).and_then(|s| marks.tinted(rows.rows(), s))
    } else {
        marks.clear();
        None
    };
    for (r, row) in rows.rows().iter().enumerate() {
        let y = top + r as f32 * w.line;
        under(w, cx, row, &rows, (left, y), r, background);
        let classes = marks.classes(row.id);
        let sign = tint.as_ref().filter(|(rows, _)| rows.contains(&r)).map(|it| it.1);
        let class = |col: usize| match classes.get(col).copied() {
            Some(c) if c == Highlight::PromptSign as u8 => sign.or(Some(c)),
            it => it,
        };
        text(w, cx, row, (left, y), cursor, &class);
    }
    w.marks = marks;
    w.state = rows;
}

/// The cell box of `cols` columns from `col` in the row at `y`.
pub(crate) fn cells(w: &TerminalWidget, (left, y): (f32, f32), col: usize, cols: usize) -> Rect {
    Rect::new(left + col as f32 * w.cell, y, cols as f32 * w.cell, w.line)
}

/// Backgrounds that are not the terminal's, the matches found, the selection, and the cursor.
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
    crate::find::paint(w, cx, row, at);
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

/// A cell: its row and column.
type Cell = (oneterm_vt::RowId, u16);

/// A glyph a cell, in the face of its style, in the middle of its cells (or, with ligatures, a
/// token's glyphs on their cells); then its lines. `class` is a column's class; `cursor` the
/// cell the block cursor fills now, and the one it is on in either phase of its blink (a token
/// stops there, so a ligature does not come and go as it blinks).
fn text(
    w: &mut TerminalWidget,
    cx: &mut PaintCx,
    row: &SnapshotRow,
    at: (f32, f32),
    cursor: (Option<Cell>, Option<Cell>),
    class: &dyn Fn(usize) -> Option<u8>,
) {
    let scale = cx.scale;
    let (ascent, descent) = cx.fonts.text.line_metrics(&w.style);
    let baseline = ((at.1 + (w.line - ascent - descent) / 2.0 + ascent) * scale).round();
    let on_row = |c: Option<Cell>| c.filter(|c| c.0 == row.id).map(|c| usize::from(c.1));
    let (stop, split) = (on_row(cursor.0), on_row(cursor.1));
    let (mut buf, mut token_to) = (String::new(), 0);
    let hovered = w.hover.iter().find(|it| it.0 == row.id).map(|it| it.1.clone()).unwrap_or_default();
    for (col, cell) in row.cells.iter().enumerate() {
        if cell.width.is_spacer() {
            continue;
        }
        let style = row.style_of(cell);
        let meant = class(col).filter(|_| plain(style)).and_then(Highlight::of);
        let underlined = meant.is_some_and(Highlight::underlined) || hovered.contains(&(col as u16));
        let blank = matches!(cell.content, SnapshotContent::Scalar(c) if crate::shape::blank(c));
        if blank && !underlined && !style.attrs.intersects(Attrs::ALL_UNDERLINES | Attrs::STRIKEOUT) {
            continue; // nothing to draw, so no colour to work out
        }
        let first = match cell.content {
            SnapshotContent::Scalar(c) => Some(c),
            SnapshotContent::Cluster { .. } => None,
        };
        let mut fg = glyph_color(w, style, first, meant);
        if stop == Some(col) {
            fg = color(w.palette.background); // on the block, the text is the background's
        }
        let cols = if cell.width == oneterm_vt::CellWidth::Wide { 2 } else { 1 };
        let b = cells(w, at, col, cols);
        let lines =
            [(Attrs::ALL_UNDERLINES, b.bottom() - 2.0), (Attrs::STRIKEOUT, b.y + (w.line / 2.0).round())];
        for (attr, y) in lines {
            if style.attrs.intersects(attr) || (underlined && attr == Attrs::ALL_UNDERLINES) {
                let bounds = Rect::new(b.x, y, b.width, 1.0);
                cx.quad(Quad { bounds, background: fg, ..Quad::default() });
            }
        }
        if blank || style.attrs.contains(Attrs::HIDDEN) || col < token_to {
            continue;
        }
        let face = face(w, cx, style);
        if w.ligatures {
            let end = crate::shape::token_end(row, col, split);
            if end > col + 1 {
                token_to = end;
                token(w, cx, row, (col, end), (b.x, baseline), face, class);
                continue;
            }
        }
        buf.clear();
        match cell.content {
            SnapshotContent::Scalar(c) => buf.push(c),
            SnapshotContent::Cluster { start, len } => buf.extend(row.cluster(start, len)),
        }
        let word = cx.fonts.text.word(&buf, face, w.style.size);
        let shift = (b.width - word.width) / 2.0;
        for g in &word.glyphs {
            let pen = Point::new((b.x + shift + g.x) * scale, baseline - (g.y * scale).round());
            w.tiles.extend(paint_glyph(cx, (g.face, g.id), w.style.size * scale, pen, fg));
        }
    }
}

/// The face of a cell's style: the look's, bold and italic as the style says.
fn face(w: &TerminalWidget, cx: &mut PaintCx, style: &Style) -> FaceId {
    let mut face_style = w.style.clone();
    face_style.weight = if style.attrs.contains(Attrs::BOLD) { 700 } else { face_style.weight };
    face_style.italic = style.attrs.contains(Attrs::ITALIC);
    cx.fonts.text.face_for(&face_style)
}

/// The cells `from..to` shaped as one word, each glyph on the cell its character is in, in
/// that cell's colour; `x` is the first cell's left.
fn token(
    w: &mut TerminalWidget,
    cx: &mut PaintCx,
    row: &SnapshotRow,
    (from, to): (usize, usize),
    (x, baseline): (f32, f32),
    face: FaceId,
    class: &dyn Fn(usize) -> Option<u8>,
) {
    let mut s = std::mem::take(&mut w.shaping);
    let Scratch { text, starts, placed, inks } = &mut s;
    text.clear();
    starts.clear();
    inks.clear();
    for cell in &row.cells[from..to] {
        starts.push(text.len() as u32);
        if let SnapshotContent::Scalar(c) = cell.content {
            text.push(c);
        }
    }
    let word = cx.fonts.text.word(text, face, w.style.size);
    crate::shape::place(&word.glyphs, starts, w.cell, placed);
    inks.resize(to - from, None);
    for &(g, at, dx) in placed.iter() {
        let col = from + at;
        let style = row.style_of(&row.cells[col]);
        let fg = match inks[at] {
            Some(fg) => fg,
            None => {
                let first = text[starts[at] as usize..].chars().next();
                let meant = class(col).filter(|_| plain(style)).and_then(Highlight::of);
                *inks[at].insert(glyph_color(w, style, first, meant))
            }
        };
        let glyph = &word.glyphs[g];
        let pen = Point::new((x + dx) * cx.scale, baseline - (glyph.y * cx.scale).round());
        w.tiles.extend(paint_glyph(cx, (glyph.face, glyph.id), w.style.size * cx.scale, pen, fg));
    }
    w.shaping = s;
}

impl TerminalWidget {
    /// The cell a focused block cursor is on, in either phase of its blink.
    fn cursor_cell(&self) -> Option<(oneterm_vt::RowId, u16)> {
        let cursor = self.state.cursor();
        (self.focused
            && cursor.visible
            && cursor.row.is_some()
            && self.cursor_style.shape == CursorShape::Block)
            .then_some((cursor.id, cursor.col))
    }

    /// The cell a filled block cursor covers, when one shows.
    fn cursor_block(&self) -> Option<(oneterm_vt::RowId, u16)> {
        let cursor = self.state.cursor();
        let on = self.focused && cursor.visible && (self.caret_on || !self.cursor_style.blinking);
        (on && cursor.row.is_some() && self.cursor_style.shape == CursorShape::Block)
            .then_some((cursor.id, cursor.col))
    }
}
