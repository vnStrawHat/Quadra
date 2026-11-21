//! The face of a row of the palette, and the heading of a group above its first row.

use std::rc::Rc;

use quadra_headless::pick::{Parts, fuzzy_hits};
use quadra_layout::Align;
use quadra_reactive::Signal;
use quadra_tree::{Role, Side, State};
use quadra_view::{Div, by_text, column, div, rich_text, row, show, span, text};

use super::{Command, HEAD, ROW};
use crate::display::{chosen_mark, key_label};
use crate::size::TEXT_SM;
use crate::{icon, icons, status_label, token};

/// The parts of the palette's rows over `all`, `on` the one the arrows are on.
pub(super) fn parts(all: Rc<Vec<Command>>, query: Signal<String>, on: Signal<usize>, none: Rc<str>) -> Parts {
    let known = all.clone();
    let empty =
        move || div().padding_x(12.0).padding_y(4.0).child(text(none.to_string()).color(token::TEXT_2));
    Parts {
        list: Rc::new(|| column().grow(1.0).padding_y(4.0)),
        line: Some(Rc::new(|| div().padding_x(4.0))),
        row: Rc::new(move |k, i| face(&all[i], &query.get_untracked(), k, on)),
        empty: Some(Rc::new(empty)),
        head: Some(Rc::new(move |before, i| heading(&known, before, i).map(head))),
    }
}

/// The group whose heading goes above command `i` when command `before` is above it: when
/// it starts a group.
pub(super) fn heading(all: &[Command], before: Option<usize>, i: usize) -> Option<Rc<str>> {
    let group = all[i].group.clone()?;
    (before.and_then(|b| all[b].group.as_ref()) != Some(&group)).then_some(group)
}

fn head(group: Rc<str>) -> Div {
    let words = text(group.to_string()).color(token::TEXT_3).size(TEXT_SM.size);
    let line = row().align_items(Align::Center).height(by_text(HEAD)).padding_x(12.0);
    line.role(Role::Heading).label(group.to_string()).child(words)
}

/// A row as wide as the palette: the lead, the title with the letters that matched, the
/// detail; at its end the state, the keys, "Enter" while the arrows are on it, the check of
/// the current one. The title gives way first.
fn face(command: &Command, query: &str, k: usize, on: Signal<usize>) -> Div {
    let hits = fuzzy_hits(query, &command.title).unwrap_or_default();
    let line = row().align_items(Align::Center).gap(8.0).grow(1.0).height(by_text(ROW)).padding_x(8.0);
    let line = line.radius(4.0).label(command.title.clone());
    let line = line.hover(|s| s.bg(token::PANEL_HOVER)).when(State::SELECTED, |s| s.bg(token::PRESS));
    let line = line.child(chosen_mark(Side::Left, move || on.get() == k));
    let line = match &command.lead {
        Some(lead) => line.child(lead().shrink(0.0)),
        None => line,
    };
    let line = line.child(marked(&command.title, &hits));
    let line = match &command.detail {
        Some(detail) => line.child(text(detail.clone()).color(token::TEXT_2).ellipsis()),
        None => line,
    };
    // At the end, each part 8 px after the one before: "Enter" takes no room while hidden.
    let part = || row().align_items(Align::Center).shrink(0.0).padding_left(8.0);
    let small = |words: String| text(words).color(token::TEXT_2).size(TEXT_SM.size);
    let mut end = row().align_items(Align::Center).shrink(0.0);
    if let Some((status, words)) = &command.status {
        end = end.child(part().child(status_label(*status, words)));
    }
    if let Some(keys) = &command.keys {
        end = end.child(part().child(small(key_label(keys))));
    }
    let enter = move || part().child(small(key_label("enter")));
    end = end.child(show(move || on.get() == k, enter).container(row().shrink(0.0)));
    if command.current {
        end = end.child(part().child(icon(icons::CHECK).size(14.0).color(token::ACCENT)));
    }
    line.child(div().grow(1.0)).child(end)
}

/// A text with the characters at `hits` in the accent: where a query matched it. One
/// paragraph with a span a run, so a space at the end of a run keeps its width; one line,
/// cut with an ellipsis where its box ends.
pub fn marked(words: &str, hits: &[usize]) -> Div {
    let (mut spans, mut run, mut on) = (Vec::new(), String::new(), false);
    let mut end = |run: String, on: bool| {
        let piece = span(run);
        spans.push(if on { piece.color(token::ACCENT_TEXT) } else { piece });
    };
    for (i, c) in words.chars().enumerate() {
        let hit = hits.contains(&i);
        if hit != on && !run.is_empty() {
            end(std::mem::take(&mut run), on);
        }
        on = hit;
        run.push(c);
    }
    end(run, on);
    row().min_width(0.0).child(rich_text(spans).ellipsis())
}
