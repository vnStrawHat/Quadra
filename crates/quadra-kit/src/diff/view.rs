//! What a row of a diff shows: its lines with their numbers and marks, the part of an edited
//! line that changed, the halves side by side, and the row of folded lines.

use quadra_layout::{Align, Justify, Overflow};
use quadra_platform::Cursor;
use quadra_reactive::Signal;
use quadra_tree::{Role, Token};
use quadra_ui::{Div, by_text, div, rich_text, row, span, text};

use super::rows::{DiffMode, Row};
use super::words::changed_parts;
use super::{LineColours, ROW, Sides};
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
    (said, colours): (&str, &[(std::ops::Range<usize>, Token)]),
    other: Option<&str>,
    (sx, wrap): (Signal<f32>, bool),
) -> Div {
    let (sign, tint, name) = match mark {
        Mark::Same => (" ", None, said.to_string()),
        Mark::Removed => ("-", Some((token::DIFF_REMOVE, token::ERROR)), format!("Removed: {said}")),
        Mark::Added => ("+", Some((token::DIFF_ADD, token::OK)), format!("Added: {said}")),
    };
    let part = other.and_then(|other| match mark {
        Mark::Removed => changed_parts(said, other).map(|(parts, _)| parts),
        Mark::Added => changed_parts(other, said).map(|(_, parts)| parts),
        Mark::Same => None,
    });
    let parts = part.unwrap_or_default();
    // Pieces where a colour or a part that changed starts or ends.
    let mut cuts = vec![0, said.len()];
    cuts.extend(parts.iter().flat_map(|part| [part.start, part.end]));
    cuts.extend(colours.iter().flat_map(|(range, _)| [range.start, range.end]));
    cuts.retain(|&cut| said.is_char_boundary(cut));
    cuts.sort_unstable();
    cuts.dedup();
    // ponytail: a tab is four spaces; stops when the editor says where they are.
    let shown = |line: &str| line.replace('\t', "    ");
    let piece = |cut: &[usize]| {
        let piece = span(shown(&said[cut[0]..cut[1]]));
        let piece = match colours.iter().find(|(range, _)| range.contains(&cut[0])) {
            Some((_, role)) => piece.color(*role),
            None => piece,
        };
        match parts.iter().any(|part| part.contains(&cut[0])) {
            true => piece.background(tint.map_or(token::CANVAS, |(_, ink)| ink).alpha(0.3)),
            false => piece,
        }
    };
    let spans: Vec<_> = cuts.windows(2).map(piece).collect();

    let number = |n: &Option<usize>| {
        let cell = row().width(gutter).height(by_text(ROW)).align_items(Align::Center).shrink(0.0);
        let cell = cell.justify(Justify::End).padding_right(8.0);
        let cell = cell.text_color(token::TEXT_3);
        match n {
            Some(n) => cell.child(text((n + 1).to_string())),
            None => cell,
        }
    };
    let line = match wrap {
        true => row().min_height(by_text(ROW)).align_items(Align::Start),
        false => row().height(by_text(ROW)).align_items(Align::Center),
    };
    let line = line.role(Role::Cell).label(name).children(numbers.iter().map(number).collect::<Vec<_>>());
    let sign =
        row().width(16.0).height(by_text(ROW)).align_items(Align::Center).shrink(0.0).child(text(sign));
    let line = match tint {
        Some((tint, ink)) => line.bg(tint).child(sign.text_color(ink)),
        None => line.child(sign),
    };
    if spans.is_empty() {
        return line;
    }
    if wrap {
        // In the flow, as wide as the column less the numbers: long lines wrap.
        return line.child(div().width(0.0).grow(1.0).child(rich_text(spans)));
    }
    // A long line is cut at the edge and scrolls sideways; the numbers stay.
    // As wide as the line at least (a byte is a cell or more of it): it never wraps.
    let wide = (said.len() + 3 * said.matches('\t').count()) as f32 * by_text(super::LETTER)() + 8.0;
    let words = div().absolute().top(0.0).left(move || -sx.get()).width(wide).child(rich_text(spans));
    line.child(div().width(0.0).grow(1.0).height(by_text(ROW)).overflow_x(Overflow::Hidden).child(words))
}

/// The old text's half of a row beside the new one's, as wide as each other; as high as
/// the higher when lines wrap.
fn halves(left: Div, right: Div, wrap: bool) -> Div {
    let right = row().border_left(1.0).border_color(token::LINE).child(right.grow(1.0));
    let pair = if wrap { row().min_height(by_text(ROW)) } else { row().height(by_text(ROW)) };
    pair.child(left.width(0.0).grow(1.0)).child(right.width(0.0).grow(1.0))
}

/// The row of lines that do not show: a button that shows them.
fn fold(old: usize, count: usize, open: Signal<Vec<usize>>) -> Div {
    let show = move || open.update(|open| open.push(old));
    let fold = row().height(by_text(ROW)).align_items(Align::Center).justify(Justify::Center);
    let fold = fold.bg(token::PANEL).text_color(token::TEXT_3).hover(|s| s.bg(token::PANEL_HOVER));
    let fold = fold.cursor(Cursor::Pointer).focus_visible(|s| s.outline(token::BORDER_FOCUSED));
    quadra_base::press::press(fold.child(text(format!("{count} unchanged lines")))).on_click(show).build()
}

/// What a row of the listing shows.
pub(super) fn build(
    sides: &Sides,
    at: Row,
    (mode, gutter, painted): (DiffMode, f32, Option<&(LineColours, LineColours)>),
    (open, sx, wrap): (Signal<Vec<usize>>, Signal<f32>, bool),
) -> Div {
    fn of(colours: Option<&LineColours>, i: usize) -> &[(std::ops::Range<usize>, Token)] {
        colours.and_then(|c| c.get(i)).map_or(&[][..], Vec::as_slice)
    }
    fn words<T>((said, _): (&str, T)) -> &str {
        said
    }
    let was = |i: usize| (sides.old.line(i), of(painted.map(|p| &p.0), i));
    let is = |j: usize| (sides.new.line(j), of(painted.map(|p| &p.1), j));
    let one = |n: usize, mark: Mark, said, other: Option<&str>| {
        line(&[Some(n)], gutter, mark, said, other, (sx, wrap))
    };
    let none = || if wrap { row().min_height(ROW) } else { row().height(ROW) }.bg(token::PANEL);
    match (at, mode) {
        (Row::Same { old, new }, DiffMode::Inline) => {
            line(&[Some(old), Some(new)], gutter, Mark::Same, was(old), None, (sx, wrap))
        }
        (Row::Same { old, new }, DiffMode::SideBySide) => {
            halves(one(old, Mark::Same, was(old), None), one(new, Mark::Same, is(new), None), wrap)
        }
        (Row::Removed { old, with }, _) => {
            let other = with.map(|j| words(is(j)));
            line(&[Some(old), None], gutter, Mark::Removed, was(old), other, (sx, wrap))
        }
        (Row::Added { new, with }, _) => {
            let other = with.map(|i| words(was(i)));
            line(&[None, Some(new)], gutter, Mark::Added, is(new), other, (sx, wrap))
        }
        (Row::Pair { old, new }, _) => halves(
            old.map_or_else(none, |i| one(i, Mark::Removed, was(i), new.map(|j| words(is(j))))),
            new.map_or_else(none, |j| one(j, Mark::Added, is(j), old.map(|i| words(was(i))))),
            wrap,
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
