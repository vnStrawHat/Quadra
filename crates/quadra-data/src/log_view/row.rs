//! A line of the log view as a row: its source's dot, its time, its level and what it says,
//! the filter's matches marked, and a note at its end.

use std::rc::Rc;

use quadra_design::token;
use quadra_layout::{Align, Overflow};
use quadra_reactive::Signal;
use quadra_tree::{Paint, RichText};
use quadra_view::{Div, by_text, div, rich_text, row, span, text};

use super::LETTER;
use super::look::{COLUMN, DOT, Look, paint, tone, word_tone};
use crate::ansi::{plain, runs};
use crate::log::Log;

/// The colour of a source's dot.
pub(super) type Colors = Rc<dyn Fn(u8) -> Paint>;

/// The row of the line at a place of the listing; what it says moved left by `sx`.
pub(super) fn line(
    log: &Log,
    place: usize,
    look: Look,
    sx: Signal<f32>,
    note: Option<String>,
    colors: &Colors,
) -> Div {
    let Some((_, at_ms, raw, faint)) = log.line(place) else { return div() };
    let raw = look.shown(raw);
    let (fade, (said, pens), level) = (if faint { 0.4 } else { 1.0 }, runs(&raw), log.level(place));
    let base = if look.levels { tone(level) } else { token::TEXT };
    let mut from = 0;
    let spans = pens.into_iter().map(|(end, pen)| {
        let run = span(&said[std::mem::replace(&mut from, end)..end]).color(paint(pen, fade, base));
        if pen.bold { run.bold() } else { run }
    });
    let mut spans: Vec<_> = spans.collect();
    let letter = by_text(LETTER * look.size)();
    let (wrap, (pad, time), (dot, column)) = (look.wrap.get(), look.lead(letter), look.parts(log));
    let tall = by_text(look.row);
    let line = row().grow(1.0).padding_left(pad);
    let line = match wrap {
        true => line.min_height(tall.clone()).align_items(Align::Start),
        false => line.height(tall.clone()).align_items(Align::Center),
    };
    let line = match dot {
        true => {
            let spot = div().width(8.0).height(8.0).radius(4.0).bg(colors(log.source(place)));
            line.child(
                row().width(DOT).height(tall.clone()).shrink(0.0).align_items(Align::Center).child(spot),
            )
        }
        false => line,
    };
    let line = match (look.timestamps.get(), at_ms) {
        (false, _) => line,
        (true, 0) => line.child(div().width(time).shrink(0.0)),
        (true, at) => {
            let said = text(look.when(at)).color(look.time.alpha(fade));
            line.child(div().width(time).shrink(0.0).child(said))
        }
    };
    let line = match column {
        true => {
            let word = text(level.word().to_string()).color(word_tone(level).alpha(fade));
            line.child(div().width(COLUMN * letter).shrink(0.0).child(word))
        }
        false => line,
    };
    let noted = note.as_ref().map_or(0, |note| note.len() + 2);
    if let Some(note) = note {
        spans.push(span(format!("  {note}")).size(12.0).color(token::ACCENT));
    }
    if spans.is_empty() {
        return line;
    }
    let marks = log.marks_in(&said);
    // ponytail: the accent is read as the row is built; a theme that changes shows it on
    // the rows built after.
    let marked = move |tree: &mut quadra_tree::Tree, id| {
        let Some(&words) = tree.children(id).first() else { return };
        let fill = tree.theme().color(token::ACCENT).alpha(0.32);
        if let Some(text) = tree.widget_mut::<RichText>(words) {
            text.round_marks(2.0);
            text.mark(marks, None, fill, fill);
        }
    };
    if wrap {
        return line.child(div().width(0.0).grow(1.0).on_mount(marked).child(rich_text(spans)));
    }
    // A long line is cut at the edge and scrolls sideways; the times stay.
    // As wide as the line at least (a byte is a cell or more of it): it never wraps.
    let wide = (said.len() + noted) as f32 * letter.max(by_text(LETTER * 12.0)()) + 8.0;
    let words = div().absolute().top(0.0).left(move || -sx.get()).width(wide).cut_to_parent();
    let words = words.on_mount(marked).child(rich_text(spans));
    line.child(div().width(0.0).grow(1.0).height(tall).overflow_x(Overflow::Hidden).child(words))
}

/// A line as copying gives it: its time, when they show, and what it says.
pub(super) fn as_text(log: &Log, place: usize, look: Look) -> String {
    let Some((_, at_ms, raw, _)) = log.line(place) else { return String::new() };
    let raw = quadra_reactive::untrack(|| look.shown(raw));
    match (look.timestamps.get_untracked(), at_ms) {
        (true, at) if at != 0 => format!("{} {}", look.when(at), plain(&raw)),
        _ => plain(&raw).into_owned(),
    }
}
