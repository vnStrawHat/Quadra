//! The gutter (specs/18, "The view"): when each row of the primary screen was first written,
//! kept with the engine and stamped by the feed, its line number, and the column that shows
//! both at the left of the grid.

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::time::Instant;

use oneterm_vt::{Cell, RowId, Terminal};
use quadra_prim::{Point, Rect};
use quadra_render::Quad;
use quadra_tree::{PaintCx, paint_glyph};

use crate::widget::{TerminalWidget, color};

/// Room around a label: half on each side.
pub(crate) const GAP: f32 = 8.0;
/// Cells of `[HH:MM:SS] `.
const CLOCK_CELLS: usize = 11;
/// The digits a line number has room for at first: none are kept ahead.
pub(crate) const DIGITS: usize = 1;

/// The stamps of the primary screen's rows: `times[j]` is the row `front + j`, in seconds
/// after `origin`.
#[derive(Debug, Default)]
pub(crate) struct Stamps {
    times: VecDeque<u32>,
    front: Option<RowId>,
    origin: Option<Instant>,
    /// The oldest live row: line 1.
    oldest: RowId,
    /// The engine's lines produced at the last update.
    lines: u64,
}

impl Stamps {
    /// Follows the engine's rows, then stamps the new rows with content at `now`. `cleared`
    /// when the screen was cleared; no `now` follows only.
    pub(crate) fn update(&mut self, term: &Terminal, cleared: bool, now: Option<Instant>) {
        let p = term.grid().primary();
        let (oldest, newest) = (p.oldest(), p.newest());
        let front = *self.front.get_or_insert(oldest);
        let (produced, len) = (term.lines_produced().saturating_sub(self.lines), self.times.len() as u64);
        self.lines = term.lines_produced();
        // The content mark: one past the lowest row with content, or the cursor's.
        let floor = (front + len).max(p.screen_top());
        let mut high = p.row_of_index(p.cursor_row_index()) + 1;
        let mut id = newest;
        while id >= floor && id >= high {
            let row = p.row(id);
            if row.occ() > 0 || row.cell(0) != Cell::EMPTY {
                high = high.max(id + 1);
                break;
            }
            id = id - 1;
        }
        if front + len <= oldest && front < oldest {
            if produced < oldest.distance(front) {
                // A column change: every row has a new id. Kept, aligned to the bottom.
                // ponytail: lines that wrap otherwise than before shift their times by a row
                // or two; anchors on rows if that shows.
                let start = if len == 0 { oldest } else { RowId(high.0.saturating_sub(len)) };
                let lost = oldest.distance(start).min(len);
                self.times.drain(..lost as usize);
                self.front = Some(start + lost);
            } else {
                self.front = Some(oldest);
                self.times.clear();
            }
        } else if front < oldest {
            self.times.drain(..oldest.distance(front).min(len) as usize);
            self.front = Some(oldest);
        }
        self.oldest = oldest;
        let front = self.front.unwrap_or(oldest);
        let keep = |to: RowId| to.distance(front) as usize;
        self.times.truncate(keep(newest + 1));
        if cleared {
            self.times.truncate(keep(p.screen_top()));
        }
        let Some(now) = now.filter(|_| !term.grid().alt_active()) else { return };
        let origin = *self.origin.get_or_insert(now);
        let secs = u32::try_from(now.saturating_duration_since(origin).as_secs()).unwrap_or(u32::MAX);
        let new = keep(high).saturating_sub(self.times.len());
        self.times.extend(std::iter::repeat_n(secs, new));
    }

    /// The line number of row `id`, from the top of the history, and when it was written: a
    /// row past the stamps has the newest, one before them the oldest.
    pub(crate) fn at(&self, id: RowId) -> (u64, Option<Instant>) {
        let front = self.front.unwrap_or(id);
        let ahead = id.0 as i128 - front.0 as i128;
        let line = id.distance(self.oldest) + 1;
        let secs = match usize::try_from(ahead) {
            Ok(j) => self.times.get(j).or(self.times.back()),
            Err(_) => self.times.front(),
        };
        let written = secs.zip(self.origin).map(|(&s, o)| o + std::time::Duration::from_secs(s.into()));
        (line, written)
    }
}

/// Writes the label of a row: `[HH:MM:SS]`, a space, the line number right-aligned in
/// `digits` cells (it ends by the grid); `--:--:--` with no time. Returns the length of the
/// clock's part.
pub(crate) fn label(out: &mut String, line: u64, digits: usize, seconds_of_day: Option<u32>) -> usize {
    out.clear();
    match seconds_of_day {
        Some(t) => _ = write!(out, "[{:02}:{:02}:{:02}]", t / 3600 % 24, t / 60 % 60, t % 60),
        None => out.push_str("[--:--:--]"),
    }
    let clock = out.len();
    _ = write!(out, " {line:>digits$}");
    clock
}

/// The digits a line number needs.
pub(crate) fn digits_of(line: u64) -> usize {
    line.checked_ilog10().map_or(1, |d| d as usize + 1)
}

impl TerminalWidget {
    /// The gutter's width in logical pixels; 0 when it is off.
    pub fn gutter_width(&self) -> f32 {
        if self.gutter { (CLOCK_CELLS + self.digits) as f32 * self.cell + GAP } else { 0.0 }
    }

    /// The line number of the row `row` of the viewport and when it was written, as the
    /// gutter shows them; `None` when the gutter is off or the row has no label.
    pub fn gutter_row(&self, row: usize) -> Option<(u64, Option<Instant>)> {
        self.gutter_rows.get(row).copied().filter(|_| self.gutter).flatten()
    }

    /// The local time of day of an instant, by the clock taken at mount.
    pub(crate) fn seconds_of_day(&self, at: Instant) -> u32 {
        let (anchor, local) = self.clock;
        let after = match at.checked_duration_since(anchor) {
            Some(d) => d.as_secs() as i64,
            None => -(anchor.saturating_duration_since(at).as_secs() as i64),
        };
        (local + after).rem_euclid(86_400) as u32
    }
}

/// The gutter's background and a label a row that shows.
pub(crate) fn paint(w: &mut TerminalWidget, cx: &mut PaintCx, top: f32) {
    let width = w.gutter_width();
    if width <= 0.0 {
        return;
    }
    let c = &w.colors;
    let fg = c.gutter_fg.unwrap_or_else(|| color(w.palette.dim(w.palette.foreground)));
    let background = c.gutter_bg.unwrap_or_else(|| color(w.palette.background));
    let (clock_fg, number_fg) = (c.clock_fg.unwrap_or(fg), c.line_number_fg.unwrap_or(fg));
    let bounds = Rect::new(cx.rect.x, cx.rect.y, width, cx.rect.height);
    cx.quad(Quad { bounds, background, ..Quad::default() });
    let scale = cx.scale;
    let (ascent, descent) = cx.fonts.text.line_metrics(&w.style);
    let face = cx.fonts.text.face_for(&w.style);
    let mut text = std::mem::take(&mut w.label);
    for r in 0..w.gutter_rows.len() {
        let Some((line, written)) = w.gutter_rows[r] else { continue };
        let clock = label(&mut text, line, w.digits, written.map(|t| w.seconds_of_day(t)));
        let y = top + r as f32 * w.line;
        let baseline = ((y + (w.line - ascent - descent) / 2.0 + ascent) * scale).round();
        // A glyph a cell, in the middle of it, as the grid draws them; the label is ASCII.
        for (col, ch) in text.char_indices().filter(|it| it.1 != ' ') {
            let ink = if col < clock { clock_fg } else { number_fg };
            let word = cx.fonts.text.word(ch.encode_utf8(&mut [0; 4]), face, w.style.size);
            let x = cx.rect.x + GAP / 2.0 + col as f32 * w.cell + (w.cell - word.width) / 2.0;
            for g in &word.glyphs {
                let pen = Point::new((x + g.x) * scale, baseline - (g.y * scale).round());
                w.tiles.extend(paint_glyph(cx, (g.face, g.id), w.style.size * scale, pen, ink));
            }
        }
    }
    w.label = text;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_label_is_the_clock_a_space_and_the_number_by_the_grid() {
        let mut out = String::new();
        assert_eq!(label(&mut out, 42, 3, Some(13 * 3600 + 5 * 60 + 9)), 10);
        assert_eq!(out, "[13:05:09]  42");
        label(&mut out, 7, 3, None);
        assert_eq!(out, "[--:--:--]   7");
        label(&mut out, 1234, 4, None);
        assert_eq!(out, "[--:--:--] 1234");
        assert_eq!((digits_of(0), digits_of(9), digits_of(10), digits_of(99_999)), (1, 1, 2, 5));
    }
}
