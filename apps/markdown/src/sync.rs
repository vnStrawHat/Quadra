//! Side by side (specs/21, "Side by side"): the panes scroll together by their headings, and
//! a heading chosen on one side is shown on the other.

use std::time::Duration;

use quadra::{Memo, Scope, Signal, effect, memo, timeout, untrack};
use quadra_md::{Heading, Place, outline, parse};

use crate::model::{Model, SIDE_BY_SIDE};

/// The room above the preview's text: its padding.
pub(crate) const PAD: f32 = 24.0;
/// How long a heading chosen is tinted.
const TINT: Duration = Duration::from_millis(600);

/// Which pane the user moved last: it drives, the other follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Driver {
    Editor,
    Preview,
}

/// Where the editor stands (a fractional line) for where the preview does (its scroll), by
/// pairs of the two: linear between the pairs around, the ends where they are.
pub(crate) fn y_of(pairs: &[(f64, f32)], line: f64) -> f32 {
    let at = pairs.partition_point(|pair| pair.0 <= line);
    match at {
        0 => pairs.first().map_or(0.0, |pair| pair.1),
        n if n == pairs.len() => pairs[n - 1].1,
        n => {
            let ((l0, y0), (l1, y1)) = (pairs[n - 1], pairs[n]);
            let t = if l1 > l0 { ((line - l0) / (l1 - l0)) as f32 } else { 0.0 };
            y0 + (y1 - y0) * t
        }
    }
}

/// The other way: the line for a scroll of the preview.
pub(crate) fn line_of(pairs: &[(f64, f32)], y: f32) -> f64 {
    let at = pairs.partition_point(|pair| pair.1 <= y);
    match at {
        0 => pairs.first().map_or(0.0, |pair| pair.0),
        n if n == pairs.len() => pairs[n - 1].0,
        n => {
            let ((l0, y0), (l1, y1)) = (pairs[n - 1], pairs[n]);
            let t = if y1 > y0 { f64::from((y - y0) / (y1 - y0)) } else { 0.0 };
            l0 + (l1 - l0) * t
        }
    }
}

/// The pairs of a document: its start, each heading laid out (its line, its block's top),
/// and its end; a pair that would go back up is left out.
pub(crate) fn pairs(headings: &[usize], places: &[Place], lines: usize) -> Vec<(f64, f32)> {
    let mut out = vec![(0.0, 0.0)];
    for &line in headings {
        if let Some(place) = places.iter().find(|place| place.line == line) {
            out.push((line as f64, place.top + PAD));
        }
    }
    let end = places.last().map_or(0.0, |place| place.top + place.height + 2.0 * PAD);
    out.push((lines as f64, end));
    let mut kept: Vec<(f64, f32)> = Vec::with_capacity(out.len());
    for pair in out {
        if kept.last().is_none_or(|last| pair.0 > last.0 && pair.1 >= last.1) {
            kept.push(pair);
        }
    }
    kept
}

/// The document's headings, read again when the text the preview shows changes.
pub(crate) fn headings(text: Signal<String>) -> Memo<Vec<Heading>> {
    memo(move || text.with(|text| outline(&parse(text))))
}

/// The pairs of where a line is in the editor and in the preview, as they change.
pub(crate) fn mapping(
    m: &Model,
    headings: Memo<Vec<Heading>>,
) -> impl Fn() -> Vec<(f64, f32)> + Copy + use<> {
    let (text, places) = (m.text, m.places);
    let lines = memo(move || text.with(|text| text.matches('\n').count() + 1));
    let at = memo(move || headings.with(|h| h.iter().map(|it| it.line).collect::<Vec<_>>()));
    move || at.with(|h| places.with(|p| pairs(h, p, lines.get())))
}

/// Keeps the panes together while they are side by side, and shows a heading chosen on
/// either side on the other. Call it while the view is built.
pub(crate) fn follow(m: &Model, headings: Memo<Vec<Heading>>) {
    let all = mapping(m, headings);
    let (layout, driver, pos, y) = (m.layout, m.driver, m.pos, m.preview_y);
    let side = move || layout.get() == SIDE_BY_SIDE;
    // The editor drives: the preview follows.
    effect(move || {
        if side() && driver.get() == Driver::Editor {
            let now = y_of(&all(), pos.get());
            if (now - untrack(|| y.get())).abs() > 0.5 {
                y.set(now);
            }
        }
    });
    // The preview drives: the editor follows.
    effect(move || {
        if side() && driver.get() == Driver::Preview {
            let now = line_of(&all(), y.get());
            if (now - untrack(|| pos.get())).abs() > 1e-3 {
                pos.set(now);
            }
        }
    });
    // The caret on a heading's line: the preview shows it, tinted.
    let (caret, shown, scope) = (m.caret, m.clone(), Scope::current());
    effect(move || {
        let line = caret.get();
        if side() && untrack(|| headings.with(|h| h.iter().any(|it| it.line == line))) {
            show_heading(&shown, line, scope);
        }
    });
}

/// A heading pressed in the preview: the editor's caret goes to the start of its line, and
/// both panes put it at their top. The focus stays where it was.
pub(crate) fn chosen(m: &Model, line: usize, scope: Scope) {
    let (places, driver, y) = (m.places, m.driver, m.preview_y);
    let top =
        places.with_untracked(|p| p.iter().find(|place| place.line == line).map(|place| place.top + PAD));
    m.editor.edit(move |w| {
        let at = w.doc.buffer.line_start(line);
        w.doc.set_cursor(at, at, false);
        // After the caret is in view: the panes go to the heading.
        if let Some(top) = top {
            driver.set(Driver::Preview);
            y.set(top);
        }
    });
    tint(m, line, scope);
}

/// Brings the heading of `line` into view in the preview, tinted for a moment; then the
/// preview is free again (its blocks laid out anew do not bring it back).
pub(crate) fn show_heading(m: &Model, line: usize, scope: Scope) {
    m.reveal.set(Some(line));
    tint(m, line, scope);
}

/// Tints the heading of `line` in the preview for a moment.
pub(crate) fn tint(m: &Model, line: usize, scope: Scope) {
    let (flash, reveal) = (m.flash, m.reveal);
    flash.set(Some(line));
    scope.run(|| {
        timeout(TINT, move || {
            if flash.get_untracked() == Some(line) {
                flash.set(None);
            }
            if reveal.get_untracked() == Some(line) {
                reveal.set(None);
            }
        })
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_heading_at_one_top_is_at_the_other_and_between_is_linear() {
        let pairs = [(0.0, 0.0), (10.0, 400.0), (30.0, 600.0), (40.0, 1000.0)];
        assert_eq!(y_of(&pairs, 10.0), 400.0);
        assert_eq!(y_of(&pairs, 20.0), 500.0);
        assert_eq!(y_of(&pairs, 99.0), 1000.0, "the end where it is");
        assert_eq!(line_of(&pairs, 400.0), 10.0);
        assert_eq!(line_of(&pairs, 500.0), 20.0);
        for line in [0.0, 3.5, 10.0, 17.25, 39.0] {
            assert!((line_of(&pairs, y_of(&pairs, line)) - line).abs() < 1e-3, "{line}");
        }
    }

    #[test]
    fn the_pairs_are_the_start_the_headings_laid_out_and_the_end() {
        let place = |line, top, height| Place { line, top, height };
        let places = [place(0, 0.0, 40.0), place(2, 56.0, 20.0), place(4, 92.0, 40.0), place(6, 148.0, 20.0)];
        let got = pairs(&[0, 4, 9], &places, 8);
        assert_eq!(got, [(0.0, 0.0), (4.0, 92.0 + PAD), (8.0, 168.0 + 2.0 * PAD)]);
    }
}
