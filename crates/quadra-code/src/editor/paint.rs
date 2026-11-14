//! Painting the editor (specs/13, "Code"): the rows of the band its layer paints, and no
//! other. Under the text the lines of the cursors, the selections, the matches and the
//! brackets; the text a glyph at the column of its character; the cursors; the gutter.

use std::ops::Range;

use quadra_prim::{Color, Point, Rect};
use quadra_render::Quad;
use quadra_text::cell_step;
use quadra_tree::{PaintCx, paint_glyph};

use super::widget::EditorWidget;

/// The rows `first..last` that show, and where things go.
struct Frame {
    first: usize,
    last: usize,
    /// The x of the column 0, and the top of the row 0.
    left: f32,
    top: f32,
    /// The text's area, right of the gutter.
    area: Rect,
}

impl Frame {
    fn shows(&self, row: usize) -> bool {
        (self.first..self.last).contains(&row)
    }
}

pub(super) fn paint(w: &mut EditorWidget, cx: &mut PaintCx) {
    w.highlight.sync(&mut w.doc);
    w.sync_rows();
    let rect = cx.rect;
    let rows = w.row_count();
    let band = cx.visible.unwrap_or(rect);
    let row_of_y = |y: f32| ((y - rect.y) / w.line_px).max(0.0);
    let first = (row_of_y(band.y).floor() as usize).min(rows);
    let last = (row_of_y(band.bottom()).ceil() as usize).min(rows);
    let area = Rect::new(rect.x + w.gutter, rect.y, w.text_width(), rect.height);
    let f = Frame { first, last, left: area.x - w.scroll_x, top: rect.y, area };
    let outer = cx.clip;
    cx.clip = outer.then(cx.device(area), 0.0);
    under(w, cx, &f);
    w.tiles.clear();
    if first < last {
        for n in w.row_line(first)..=w.row_line(last - 1) {
            line(w, cx, &f, n);
        }
    }
    if w.focused {
        for c in w.doc.cursors.clone() {
            let (row, x) = w.place(c.head);
            if f.shows(row) {
                let x = f.left + x + preedit_cols(w, c.head) as f32 * w.cell;
                let color = if w.caret_on { w.colors.cursor } else { Color::TRANSPARENT };
                let bounds = cx.device(Rect::new(x, f.top + row as f32 * w.line_px, 2.0, w.line_px));
                cx.push(Quad { bounds, background: color, ..Quad::default() }.instance(&cx.clip));
            }
        }
    }
    cx.clip = outer;
    gutter(w, cx, &f);
}

/// How many columns the composition shows before the caret it is at, at the primary cursor.
fn preedit_cols(w: &EditorWidget, head: usize) -> usize {
    match &w.preedit {
        Some((text, caret)) if head == w.doc.primary().head => w.columns(&text[..*caret]),
        _ => 0,
    }
}

/// A box over a range of the text in each of its rows that show; a range that goes on past
/// the end of a line shows half a cell of its line break.
fn boxes(w: &EditorWidget, f: &Frame, range: Range<usize>, mut each: impl FnMut(Rect)) {
    let ((from, x0), (to, x1)) = (w.place(range.start), w.place(range.end));
    for row in from.max(f.first)..=to.min(f.last.saturating_sub(1)) {
        if row >= f.last {
            break;
        }
        let left = if row == from { x0 } else { 0.0 };
        let right = if row == to {
            x1
        } else {
            // As far as the row goes, and half a cell of the line break after the last.
            let span = w.row_span(row);
            let width = (w.doc.col_of(span.end) - w.doc.col_of(span.start)) as f32 * w.cell;
            let last_of_line = row + 1 == w.line_rows(w.row_line(row)).end;
            width + if last_of_line { w.cell / 2.0 } else { 0.0 }
        };
        each(Rect::new(f.left + left, f.top + row as f32 * w.line_px, right - left, w.line_px));
    }
}

fn under(w: &EditorWidget, cx: &mut PaintCx, f: &Frame) {
    let fill = |cx: &mut PaintCx, bounds: Rect, background: Color| {
        cx.quad(Quad { bounds, background, ..Quad::default() });
    };
    if w.focused {
        for c in &w.doc.cursors {
            for row in w.line_rows(w.doc.buffer.line_of(c.head)).filter(|&row| f.shows(row)) {
                let band = Rect::new(f.area.x, f.top + row as f32 * w.line_px, f.area.width, w.line_px);
                fill(cx, band, w.colors.line);
            }
        }
    }
    for c in w.doc.cursors.iter().filter(|c| !c.is_empty()) {
        boxes(w, f, c.range(), |r| fill(cx, r, w.colors.selection));
    }
    // The matches from the first that ends in a row that shows.
    let start = if f.first < w.row_count() { w.row_span(f.first).start } else { usize::MAX };
    let end = if f.last < w.row_count() { w.row_span(f.last).start } else { usize::MAX };
    let from = w.matches.partition_point(|m| m.end < start);
    for (i, m) in w.matches.iter().enumerate().skip(from).take_while(|(_, m)| m.start < end) {
        let color = if w.current == Some(i) { w.colors.find_current } else { w.colors.find };
        boxes(w, f, m.clone(), |r| fill(cx, r, color));
    }
    // The link Ctrl would open, underlined.
    if let Some((range, _)) = w.link.as_ref().filter(|_| w.link_hot) {
        boxes(w, f, range.clone(), |r| {
            let under = Rect::new(r.x, r.y + w.line_px - 4.0, r.width, 1.0);
            fill(cx, under, w.colors.cursor)
        });
    }
    if w.focused
        && let Some((a, b)) = w.doc.bracket_pair()
    {
        for at in [a, b] {
            boxes(w, f, at..at + 1, |r| fill(cx, r, w.colors.bracket));
        }
    }
}

/// Paints a line's text: each glyph at the column of its character in its row, in the
/// colour of the kind of its token; with the composition at the primary cursor, underlined.
fn line(w: &mut EditorWidget, cx: &mut PaintCx, f: &Frame, n: usize) {
    w.highlight.tokens(&w.doc.buffer, n, &mut w.tokens);
    let mut shown = w.doc.buffer.line(n).into_owned();
    let start = w.doc.buffer.line_start(n);
    let head = w.doc.primary().head;
    // What is composed goes in at the caret, and what follows it moves along.
    let mut composed = 0..0;
    if let Some((text, _)) = &w.preedit
        && w.doc.buffer.line_of(head) == n
    {
        shown.insert_str(head - start, text);
        composed = head - start..head - start + text.len();
    }
    let rows = w.line_rows(n);
    let tab = w.doc.tab_size.max(1);
    // Where each row starts in what is shown, and its column.
    let starts: Vec<(usize, usize)> = rows
        .clone()
        .map(|row| {
            let at = w.row_span(row).start - start;
            let at = if at > composed.start { at + composed.len() } else { at };
            (at, w.columns(&shown[..at]))
        })
        .collect();
    let (size, scale) = (w.style.size, cx.scale);
    let face = cx.fonts.text.face_for(&w.style);
    let (ascent, descent) = cx.fonts.text.line_metrics(&w.style);
    let baseline = |row: usize| {
        let top = f.top + row as f32 * w.line_px;
        ((top + (w.line_px - ascent - descent) / 2.0 + ascent) * scale).round()
    };
    let word = cx.fonts.text.word(&shown, face, size);
    // Walking the glyphs, which are in the order of the text: the column of the character
    // a glyph is of, its row, and the token that holds it.
    let (mut at, mut col, mut token, mut r) = (0, 0, 0, 0);
    let (mut cluster, mut base) = (usize::MAX, 0.0);
    for g in &word.glyphs {
        let c = g.cluster as usize;
        if c != cluster {
            while at < c {
                let (end, width) = cell_step(&shown, at);
                col = if shown.as_bytes()[at] == b'\t' { (col / tab + 1) * tab } else { col + width };
                at = end;
            }
            while r + 1 < starts.len() && c >= starts[r + 1].0 {
                r += 1;
            }
            (cluster, base) = (c, g.x);
        }
        let row = rows.start + r;
        if shown.as_bytes().get(c) == Some(&b'\t') || !f.shows(row) {
            continue;
        }
        let x = f.left + (col - starts[r].1) as f32 * w.cell + (g.x - base);
        if x > f.area.right() || x + g.advance < f.area.x {
            continue;
        }
        let color = if composed.contains(&c) {
            w.colors.text
        } else {
            let c = if c >= composed.end { c - composed.len() } else { c };
            while w.tokens.get(token).is_some_and(|t| t.end <= c) {
                token += 1;
            }
            match w.tokens.get(token) {
                Some(t) if t.start <= c => w.colors.syntax[t.kind as usize],
                _ => w.colors.text,
            }
        };
        let pen = Point::new(x * scale, baseline(row) - (g.y * scale).round());
        w.tiles.extend(paint_glyph(cx, (g.face, g.id), size * scale, pen, color));
    }
    if !w.wrap {
        w.widest = w.widest.max((w.columns(&shown) + 4) as f32 * w.cell);
    }
    if !composed.is_empty() {
        let x0 = w.columns(&shown[..composed.start]) as f32 * w.cell;
        let x1 = w.columns(&shown[..composed.end]) as f32 * w.cell;
        let (row, _) = w.place(head);
        let from = w.columns(&shown[..starts[row - rows.start].0]) as f32 * w.cell;
        let top = f.top + row as f32 * w.line_px;
        let bounds = Rect::new(f.left + x0 - from, top + w.line_px - 2.0, x1 - x0, 1.0);
        cx.quad(Quad { bounds, background: w.colors.text, ..Quad::default() });
    }
}

/// The numbers of the lines on their first rows, right of at least four digits, brighter on
/// the lines of the cursors while the editor has the focus.
fn gutter(w: &mut EditorWidget, cx: &mut PaintCx, f: &Frame) {
    let (size, scale) = (w.style.size, cx.scale);
    let face = cx.fonts.text.face_for(&w.style);
    let (ascent, descent) = cx.fonts.text.line_metrics(&w.style);
    let right = cx.rect.x + w.gutter - 3.0 * w.cell;
    if f.first >= f.last {
        return;
    }
    for n in w.row_line(f.first)..=w.row_line(f.last - 1) {
        let row = w.line_rows(n).start;
        if !f.shows(row) {
            continue;
        }
        let number = (n + 1).to_string();
        let active = w.focused && w.doc.cursors.iter().any(|c| w.doc.buffer.line_of(c.head) == n);
        let color = if active { w.colors.gutter_active } else { w.colors.gutter };
        let top = f.top + row as f32 * w.line_px;
        w.paint_mark(cx, n, top);
        let baseline = ((top + (w.line_px - ascent - descent) / 2.0 + ascent) * scale).round();
        let word = cx.fonts.text.word(&number, face, size);
        let left = right - word.width;
        for g in &word.glyphs {
            let pen = Point::new((left + g.x) * scale, baseline - (g.y * scale).round());
            w.tiles.extend(paint_glyph(cx, (g.face, g.id), size * scale, pen, color));
        }
    }
}
