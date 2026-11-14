//! What a table shows: its header, where a column is sorted and made wider, and its rows.

use std::cell::Cell;
use std::rc::Rc;

use quadra_layout::{Align, Justify, Length, Overflow};
use quadra_platform::{Clipboard, Cursor};
use quadra_reactive::{Signal, signal};
use quadra_tree::{Paint, PointerKind, Role, State as On};
use quadra_ui::{Div, column, div, dynamic, row, text, uniform_list};

use super::{CellKind, State, age_text};
use crate::action::{Copy, ExtendNext, ExtendPrevious, MoveFirst, MoveLast, MoveNext, MovePrevious, TABLE};
use crate::action::{PageNext, PagePrevious};
use crate::icon::icon;
use crate::status::{Status, status_label};
use crate::{fonts, icons, token};

/// The width of the strip at the edge of a title that makes its column wider, and how many
/// rows a page key moves by.
const GRIP: f32 = 5.0;
const PAGE: usize = 20;

/// The header: the titles on the plane of table heads over a line; the arrow of the column
/// the rows are sorted by is in the accent.
fn header(state: &State) -> Div {
    let mut head = row().height(28.0).shrink(0.0).bg(token::TABLE_HEAD_BG).text_color(token::TABLE_HEAD_FG);
    head = head.border_bottom(1.0).border_color(token::LINE).role(Role::Row);
    for (c, at) in state.columns.iter().enumerate() {
        let (sort, widths, least, sorts) = (state.sort, state.widths, at.least, state.clone());
        let arrow = dynamic(move || match sort.get() {
            Some((by, down)) if by == c => {
                let body = if down { icons::ARROW_DOWN } else { icons::ARROW_UP };
                div().child(icon(body).size(12.0).color(token::ACCENT_TEXT))
            }
            _ => div(),
        });
        let title = row().align_items(Align::Center).gap(4.0).grow(1.0).padding_left(8.0);
        let title = title.overflow_x(Overflow::Hidden).role(Role::Cell);
        let title = if at.counts() { title.justify(Justify::End).padding_right(16.0 - GRIP) } else { title };
        let title = title.child(text(at.title.clone())).child(arrow);
        let title = match at.sortable {
            true => title
                .cursor(Cursor::Pointer)
                .hover(|s| s.bg(token::PANEL_HOVER).text_color(token::TEXT))
                .on_click(move |_| sorts.sort_by(c)),
            false => title,
        };
        // The edge is dragged with the pointer held: the column is as wide as it was and the
        // pointer has moved, and never under its least.
        let held: Rc<Cell<Option<(f32, f32)>>> = Rc::default();
        let (press, drag) = (held.clone(), held.clone());
        let grip = div().width(GRIP).shrink(0.0).cursor(Cursor::ResizeHorizontal);
        let grip = grip.hover(|s| s.bg(token::BORDER_FOCUSED));
        let grip = grip.on_pointer(PointerKind::Down, move |e| {
            press.set(Some((e.at.x, widths.with_untracked(|all| all[c]))));
            e.capture_pointer();
            e.stop();
        });
        let grip = grip.on_pointer(PointerKind::Move, move |e| {
            if let Some((from, was)) = drag.get() {
                widths.update(|all| all[c] = (was + e.at.x - from).max(least));
            }
        });
        let grip = grip.on_pointer(PointerKind::Up, move |_| held.set(None));
        let whole = row().shrink(0.0).width(move || Length::Px(widths.with(|all| all[c])));
        head = head.child(whole.child(title).child(grip));
    }
    head
}

/// A row: its cells, each as its column shows them; tinted with the accent's mark at its left
/// when selected, and outlined when the keys are on it and the table has the focus.
fn line(state: &State, place: Signal<usize>) -> Div {
    let (s, on, mark, pick) = (state.clone(), state.clone(), state.clone(), state.clone());
    let data = move || s.data(place.get());
    let chosen = move || on.chosen.with(|rows| rows.binary_search(&on.data(place.get())).is_ok());
    let shown = chosen.clone();
    let ring = move || -> Paint {
        let here = mark.focused.get() && mark.cursor.get() == Some(place.get());
        if here { token::BORDER_FOCUSED.into() } else { Paint::default() }
    };
    let line = row().align_items(Align::Center).grow(1.0).height(state.row_height).role(Role::Row);
    let line = line.border_bottom(1.0).border_color(token::TABLE_ROW_BORDER).selected(chosen).outline(ring);
    let line = line
        .hover(|s| s.bg(token::RAISED_HOVER.alpha(0.6)))
        .when(On::SELECTED, |s| s.bg(token::ACCENT.alpha(0.1)));
    let mut line = line.on_click(move |e| pick.pick(place.get_untracked(), e.mods));
    if let Some(live) = state.live {
        line = line.child(live.tints(state, place));
    }
    for (c, at) in state.columns.iter().enumerate() {
        let (widths, says, row_of) = (state.widths, state.cell.clone(), data.clone());
        let value = move || says(row_of(), c);
        let cell = row().align_items(Align::Center).shrink(0.0).padding_x(8.0).role(Role::Cell);
        let cell = cell.overflow_x(Overflow::Hidden).width(move || Length::Px(widths.with(|all| all[c])));
        // Numbers are in the mono face at the right of their cells, so columns line up.
        let cell = match at.counts() {
            true => cell.justify(Justify::End).padding_right(16.0).font_family(fonts::MONO),
            false => cell,
        };
        line = line.child(match at.kind {
            CellKind::Text => cell.child(text(value)),
            CellKind::Age => cell.child(text(move || age_text(value().trim().parse().unwrap_or(0)))),
            CellKind::Status => cell.child(dynamic(move || {
                let word = value();
                status_label(Status::of(&word), &word)
            })),
        });
    }
    // After the cells: what reads a row by its children finds them where they were.
    line.child(crate::display::chosen_mark(quadra_tree::Side::Left, shown))
}

/// The table: its header over its rows, with the focus and the keys on the whole.
pub(crate) fn build(state: State, clipboard: Option<Rc<dyn Clipboard>>) -> Div {
    let (count, rows, cursor) = (state.rows.clone(), state.clone(), state.cursor);
    let body = uniform_list(move || count(), state.row_height, signal(0.0), move |place| line(&rows, place));
    let body = body.container(div().grow(1.0)).reveal(move || cursor.get());
    let focused = state.focused;
    let all = column().grow(1.0).overflow_x(Overflow::Hidden).role(Role::Table);
    let all = all.focusable().key_context(TABLE).on_focus(move |has| focused.set(has));
    // The arrows move the row the keys are on and select it; with Shift, from where it was.
    let moves = |extend: bool, to: fn(usize, usize) -> usize| {
        let state = state.clone();
        move || state.step(to, extend)
    };
    let (next, previous) = (moves(false, |at, _| at + 1), moves(false, |at, _| at.saturating_sub(1)));
    let (more, less) = (moves(true, |at, _| at + 1), moves(true, |at, _| at.saturating_sub(1)));
    let (first, last) = (moves(false, |_, _| 0), moves(false, |_, last| last));
    let (down, up) = (moves(false, |at, _| at + PAGE), moves(false, |at, _| at.saturating_sub(PAGE)));
    let all = all.on_action::<MoveNext>(move |_| next()).on_action::<MovePrevious>(move |_| previous());
    let all = all.on_action::<ExtendNext>(move |_| more()).on_action::<ExtendPrevious>(move |_| less());
    let all = all.on_action::<MoveFirst>(move |_| first()).on_action::<MoveLast>(move |_| last());
    let all = all.on_action::<PageNext>(move |_| down()).on_action::<PagePrevious>(move |_| up());
    let copies = state.clone();
    let all = all.on_action::<Copy>(move |_| {
        let text = copies.copy_text();
        if let (false, Some(clipboard)) = (text.is_empty(), &clipboard) {
            clipboard.write_text(&text);
        }
    });
    all.child(header(&state)).child(body)
}
