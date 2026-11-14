//! Painting the editor (specs/13, "Code"): the lines of the band its layer paints, and no
//! other. Under the text the lines of the cursors, the selections, the matches and the
//! brackets; the text a glyph at the column of its character; the cursors; the gutter.

use std::ops::Range;

use quadra_prim::{Color, Point, Rect};
use quadra_render::Quad;
use quadra_text::cell_step;
use quadra_tree::{PaintCx, paint_glyph};

use super::widget::EditorWidget;

/// The lines `first..last` that show, and where things go.
struct Frame {
    first: usize,
    last: usize,
    /// The x of the column 0, and the top of the line 0.
    left: f32,
    top: f32,
    /// The text's area, right of the gutter.
    area: Rect,
}

pub(super) fn paint(w: &mut EditorWidget, cx: &mut PaintCx) {
    w.highlight.sync(&mut w.doc);
    let rect = cx.rect;
    let lines = w.doc.buffer.line_count();
    let band = cx.visible.unwrap_or(rect);
    let line_of_y = |y: f32| ((y - rect.y) / w.line_px).max(0.0);
    let first = (line_of_y(band.y).floor() as usize).min(lines);
    let last = (line_of_y(band.bottom()).ceil() as usize).min(lines);
    let area = Rect::new(rect.x + w.gutter, rect.y, w.text_width(), rect.height);
    let f = Frame { first, last, left: area.x - w.scroll_x, top: rect.y, area };
    let outer = cx.clip;
    cx.clip = outer.then(cx.device(area), 0.0);
    under(w, cx, &f);
    w.tiles.clear();
    for n in first..last {
        line(w, cx, &f, n);
    }
    if w.focused {
        for c in w.doc.cursors.clone() {
            let n = w.doc.buffer.line_of(c.head);
            if (first..last).contains(&n) {
                let x = f.left + w.x_of(c.head) + preedit_cols(w, c.head) as f32 * w.cell;
                let color = if w.caret_on { w.colors.cursor } else { Color::TRANSPARENT };
                let bounds = cx.device(Rect::new(x, f.top + n as f32 * w.line_px, 2.0, w.line_px));
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

/// A box over a range of the text in each of its lines that show; a range that goes on
/// past the end of a line shows half a cell of its line break.
fn boxes(w: &EditorWidget, f: &Frame, range: Range<usize>, mut each: impl FnMut(Rect)) {
    let buffer = &w.doc.buffer;
    let (from, to) = (buffer.line_of(range.start), buffer.line_of(range.end));
    for n in from.max(f.first)..=to.min(f.last.saturating_sub(1)) {
        if n >= f.last {
            break;
        }
        let x0 = if n == from { w.x_of(range.start) } else { 0.0 };
        let x1 = if n == to {
            w.x_of(range.end)
        } else {
            w.columns(&buffer.line(n)) as f32 * w.cell + w.cell / 2.0
        };
        each(Rect::new(f.left + x0, f.top + n as f32 * w.line_px, x1 - x0, w.line_px));
    }
}

fn under(w: &EditorWidget, cx: &mut PaintCx, f: &Frame) {
    let fill = |cx: &mut PaintCx, bounds: Rect, background: Color| {
        cx.quad(Quad { bounds, background, ..Quad::default() });
    };
    if w.focused {
        for c in &w.doc.cursors {
            let n = w.doc.buffer.line_of(c.head);
            if (f.first..f.last).contains(&n) {
                let band = Rect::new(f.area.x, f.top + n as f32 * w.line_px, f.area.width, w.line_px);
                fill(cx, band, w.colors.line);
            }
        }
    }
    for c in w.doc.cursors.iter().filter(|c| !c.is_empty()) {
        boxes(w, f, c.range(), |r| fill(cx, r, w.colors.selection));
    }
    // The matches from the first that ends in a line that shows.
    let start = w.doc.buffer.line_start(f.first);
    let end = if f.last < w.doc.buffer.line_count() { w.doc.buffer.line_start(f.last) } else { usize::MAX };
    let from = w.matches.partition_point(|m| m.end < start);
    for (i, m) in w.matches.iter().enumerate().skip(from).take_while(|(_, m)| m.start < end) {
        let color = if w.current == Some(i) { w.colors.find_current } else { w.colors.find };
        boxes(w, f, m.clone(), |r| fill(cx, r, color));
    }
    if w.focused
        && let Some((a, b)) = w.doc.bracket_pair()
    {
        for at in [a, b] {
            boxes(w, f, at..at + 1, |r| fill(cx, r, w.colors.bracket));
        }
    }
}

/// Paints a line's text: each glyph at the column of its character, in the colour of the
/// kind of its token; with the composition at the primary cursor, underlined.
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
    let top = f.top + n as f32 * w.line_px;
    let (size, scale, tab) = (w.style.size, cx.scale, w.doc.tab_size.max(1));
    let face = cx.fonts.text.face_for(&w.style);
    let (ascent, descent) = cx.fonts.text.line_metrics(&w.style);
    let baseline = ((top + (w.line_px - ascent - descent) / 2.0 + ascent) * scale).round();
    let word = cx.fonts.text.word(&shown, face, size);
    // Walking the glyphs, which are in the order of the text: the column of the character
    // a glyph is of, and the token that holds it.
    let (mut at, mut col, mut token) = (0, 0, 0);
    let (mut cluster, mut base) = (usize::MAX, 0.0);
    for g in &word.glyphs {
        let c = g.cluster as usize;
        if c != cluster {
            while at < c {
                let (end, width) = cell_step(&shown, at);
                col = if shown.as_bytes()[at] == b'\t' { (col / tab + 1) * tab } else { col + width };
                at = end;
            }
            (cluster, base) = (c, g.x);
        }
        if shown.as_bytes().get(c) == Some(&b'\t') {
            continue;
        }
        let x = f.left + col as f32 * w.cell + (g.x - base);
        if x > f.area.right() {
            break;
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
        if x + g.advance < f.area.x {
            continue;
        }
        let pen = Point::new(x * scale, baseline - (g.y * scale).round());
        w.tiles.extend(paint_glyph(cx, (g.face, g.id), size * scale, pen, color));
    }
    let cols = w.columns(&shown);
    w.widest = w.widest.max((cols + 4) as f32 * w.cell);
    if !composed.is_empty() {
        let x0 = w.columns(&shown[..composed.start]) as f32 * w.cell;
        let x1 = w.columns(&shown[..composed.end]) as f32 * w.cell;
        let bounds = Rect::new(f.left + x0, top + w.line_px - 2.0, x1 - x0, 1.0);
        cx.quad(Quad { bounds, background: w.colors.text, ..Quad::default() });
    }
}

/// The numbers of the lines, right of at least four digits, brighter on the lines of the
/// cursors while the editor has the focus.
fn gutter(w: &mut EditorWidget, cx: &mut PaintCx, f: &Frame) {
    let (size, scale) = (w.style.size, cx.scale);
    let face = cx.fonts.text.face_for(&w.style);
    let (ascent, descent) = cx.fonts.text.line_metrics(&w.style);
    let right = cx.rect.x + w.gutter - 3.0 * w.cell;
    for n in f.first..f.last {
        let number = (n + 1).to_string();
        let active = w.focused && w.doc.cursors.iter().any(|c| w.doc.buffer.line_of(c.head) == n);
        let color = if active { w.colors.gutter_active } else { w.colors.gutter };
        let top = f.top + n as f32 * w.line_px;
        let baseline = ((top + (w.line_px - ascent - descent) / 2.0 + ascent) * scale).round();
        let word = cx.fonts.text.word(&number, face, size);
        let left = right - word.width;
        for g in &word.glyphs {
            let pen = Point::new((left + g.x) * scale, baseline - (g.y * scale).round());
            w.tiles.extend(paint_glyph(cx, (g.face, g.id), size * scale, pen, color));
        }
    }
}
