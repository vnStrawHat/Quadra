//! What a row of a diff shows: its lines with their numbers and marks, the part of an edited
//! line that changed, the halves side by side, and the row of folded lines.

use quadra_layout::{Align, Justify, Overflow};
use quadra_platform::Cursor;
use quadra_reactive::Signal;
use quadra_tree::Role;
use quadra_ui::{Div, div, rich_text, row, span, text};

use super::lines::changed_part;
use super::rows::{DiffMode, Row};
use super::{ROW, Sides};
use crate::token;

#[derive(Clone, Copy, PartialEq)]
enum Mark {
    Same,
    Removed,
    Added,
}

/// A line on its row: its numbers, its mark and what it says, on the tint of what happened
/// to it, moved left by `sx` past the numbers. `other` is the line it is paired with: what
/// is not common to both is tinted more.
fn line(
    numbers: &[Option<usize>],
    gutter: f32,
    mark: Mark,
    said: &str,
    other: Option<&str>,
    sx: Signal<f32>,
) -> Div {
    let (sign, tint, name) = match mark {
        Mark::Same => (" ", None, said.to_string()),
        Mark::Removed => ("-", Some((token::DIFF_REMOVE, token::ERROR)), format!("Removed: {said}")),
        Mark::Added => ("+", Some((token::DIFF_ADD, token::OK)), format!("Added: {said}")),
    };
    // ponytail: a tab is four spaces; stops when the editor says where they are.
    let shown = |line: &str| line.replace('\t', "    ");
    let (said, other) = (shown(said), other.map(shown));
    let part = other.and_then(|other| match mark {
        Mark::Removed => changed_part(&said, &other).map(|(part, _)| part),
        Mark::Added => changed_part(&other, &said).map(|(_, part)| part),
        Mark::Same => None,
    });
    let part = part.unwrap_or(0..0);
    let before = span(&said[..part.start]);
    let changed = span(&said[part.clone()]).background(tint.map_or(token::CANVAS, |(_, ink)| ink).alpha(0.3));
    let spans =
        [(before, part.start), (changed, part.len()), (span(&said[part.end..]), said.len() - part.end)];
    let spans: Vec<_> = spans.into_iter().filter(|(_, len)| *len > 0).map(|(span, _)| span).collect();

    let number = |n: &Option<usize>| {
        let cell = row().width(gutter).shrink(0.0).justify(Justify::End).padding_right(8.0);
        let cell = cell.text_color(token::TEXT_3);
        match n {
            Some(n) => cell.child(text((n + 1).to_string())),
            None => cell,
        }
    };
    let line = row().height(ROW).align_items(Align::Center).role(Role::Cell).label(name);
    let line = line.children(numbers.iter().map(number).collect::<Vec<_>>());
    let sign = div().width(16.0).shrink(0.0).child(text(sign));
    let line = match tint {
        Some((tint, ink)) => line.bg(tint).child(sign.text_color(ink)),
        None => line.child(sign),
    };
    if spans.is_empty() {
        return line;
    }
    // A long line is cut at the edge and scrolls sideways; the numbers stay.
    let words = div().absolute().top(0.0).left(move || -sx.get()).child(rich_text(spans));
    line.child(div().width(0.0).grow(1.0).height(ROW).overflow_x(Overflow::Hidden).child(words))
}

/// The old text's half of a row beside the new one's, as wide as each other.
fn halves(left: Div, right: Div) -> Div {
    let right = row().border_left(1.0).border_color(token::LINE).child(right.grow(1.0));
    row().height(ROW).child(left.width(0.0).grow(1.0)).child(right.width(0.0).grow(1.0))
}

/// The row of lines that do not show: a button that shows them.
fn fold(old: usize, count: usize, open: Signal<Vec<usize>>) -> Div {
    let show = move || open.update(|open| open.push(old));
    let fold = row().height(ROW).align_items(Align::Center).justify(Justify::Center);
    let fold = fold.bg(token::PANEL).text_color(token::TEXT_3).hover(|s| s.bg(token::PANEL_HOVER));
    let fold = fold.cursor(Cursor::Pointer).focus_visible(|s| s.outline(token::BORDER_FOCUSED));
    quadra_base::press::press(fold.child(text(format!("{count} unchanged lines")))).on_click(show).build()
}

/// What a row of the listing shows.
pub(super) fn build(
    sides: &Sides,
    at: Row,
    mode: DiffMode,
    gutter: f32,
    (open, sx): (Signal<Vec<usize>>, Signal<f32>),
) -> Div {
    let (was, is) = (|i: usize| sides.old.line(i), |j: usize| sides.new.line(j));
    let one = |n: usize, mark: Mark, said: &str, other: Option<&str>| {
        line(&[Some(n)], gutter, mark, said, other, sx)
    };
    let none = || row().height(ROW).bg(token::PANEL);
    match (at, mode) {
        (Row::Same { old, new }, DiffMode::Inline) => {
            line(&[Some(old), Some(new)], gutter, Mark::Same, was(old), None, sx)
        }
        (Row::Same { old, new }, DiffMode::SideBySide) => {
            halves(one(old, Mark::Same, was(old), None), one(new, Mark::Same, is(new), None))
        }
        (Row::Removed { old, with }, _) => {
            line(&[Some(old), None], gutter, Mark::Removed, was(old), with.map(is), sx)
        }
        (Row::Added { new, with }, _) => {
            line(&[None, Some(new)], gutter, Mark::Added, is(new), with.map(was), sx)
        }
        (Row::Pair { old, new }, _) => halves(
            old.map_or_else(none, |i| one(i, Mark::Removed, was(i), new.map(is))),
            new.map_or_else(none, |j| one(j, Mark::Added, is(j), old.map(was))),
        ),
        (Row::Fold { old, count }, _) => fold(old, count, open),
    }
}

/// A row as text, as a unified diff writes it: what copying gives.
pub(super) fn as_text(sides: &Sides, at: Row) -> String {
    let (was, is) = (|i: usize| sides.old.line(i), |j: usize| sides.new.line(j));
    match at {
        Row::Same { old, .. } => format!(" {}", was(old)),
        Row::Removed { old, .. } => format!("-{}", was(old)),
        Row::Added { new, .. } => format!("+{}", is(new)),
        Row::Pair { old, new } => {
            let lines = [old.map(|i| format!("-{}", was(i))), new.map(|j| format!("+{}", is(j)))];
            lines.into_iter().flatten().collect::<Vec<_>>().join("\n")
        }
        Row::Fold { count, .. } => format!("... {count} unchanged lines"),
    }
}
