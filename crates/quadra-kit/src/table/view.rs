//! What a table shows: its header, where a column is sorted and made wider, and its rows.

use std::rc::Rc;

use quadra_base::table::Sheet;
use quadra_layout::{Align, Justify, Length, Overflow};
use quadra_platform::{Clipboard, Cursor};
use quadra_reactive::{Signal, signal};
use quadra_tree::{Paint, Role, State as On};
use quadra_ui::{Div, column, div, dynamic, row, scroll_x, text, uniform_list};

use super::{CellKind, Column, age_text};
use crate::icon::icon;
use crate::status::{Status, status_label};
use crate::{fonts, icons, token};

/// The width of the strip at the edge of a title that makes its column wider.
const GRIP: f32 = 5.0;

/// The header: the titles on the plane of table heads over a line; the arrow of the column
/// the rows are sorted by is in the accent.
fn header(state: &Sheet, columns: &[Column]) -> Div {
    let mut head = row().height(28.0).shrink(0.0).bg(token::TABLE_HEAD_BG).text_color(token::TABLE_HEAD_FG);
    head = head.border_bottom(1.0).border_color(token::LINE).role(Role::Row);
    for (c, at) in columns.iter().enumerate() {
        let (sort, widths) = (state.sort, state.widths);
        let arrow = dynamic(move || match sort.get() {
            Some((by, down)) if by == c => {
                let body = if down { icons::ARROW_DOWN } else { icons::ARROW_UP };
                div().child(icon(body).size(12.0).color(token::ACCENT_TEXT))
            }
            _ => div(),
        });
        let title = row().align_items(Align::Center).gap(4.0).grow(1.0).padding_left(8.0);
        let title = title.overflow_x(Overflow::Hidden);
        let title = if at.counts() { title.justify(Justify::End).padding_right(16.0 - GRIP) } else { title };
        let title = title.child(text(at.title.clone())).child(arrow);
        let title = match at.sortable {
            true => title.cursor(Cursor::Pointer).hover(|s| s.bg(token::PANEL_HOVER).text_color(token::TEXT)),
            false => title,
        };
        let title = state.title(title, c);
        let grip = div().width(GRIP).shrink(0.0).cursor(Cursor::ResizeHorizontal);
        let grip = state.grip(grip.hover(|s| s.bg(token::BORDER_FOCUSED)), c);
        let whole = row().shrink(0.0).width(move || Length::Px(widths.with(|all| all[c])));
        head = head.child(whole.child(title).child(grip));
    }
    head
}

/// A row: its cells, each as its column shows them; tinted with the accent's mark at its left
/// when selected, and outlined when the keys are on it and the table has the focus.
fn line(state: &Sheet, columns: &[Column], height: f32, place: Signal<usize>) -> Div {
    let (s, mark) = (state.clone(), state.clone());
    let data = move || s.data(place.get());
    let shown = state.selected(place);
    let ring = move || -> Paint {
        let here = mark.focused.get() && mark.cursor.get() == Some(place.get());
        if here { token::BORDER_FOCUSED.into() } else { Paint::default() }
    };
    let line = row().align_items(Align::Center).grow(1.0).height(height);
    let line = line.border_bottom(1.0).border_color(token::TABLE_ROW_BORDER).outline(ring);
    let line = line
        .hover(|s| s.bg(token::RAISED_HOVER.alpha(0.6)))
        .when(On::SELECTED, |s| s.bg(token::ACCENT.alpha(0.1)));
    let mut line = state.row(line, place);
    if let Some(live) = state.live {
        line = line.child(super::live::tints(live, state, place));
    }
    for (c, at) in columns.iter().enumerate() {
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
pub(crate) fn build(
    state: Sheet,
    columns: Rc<Vec<Column>>,
    height: f32,
    clipboard: Option<Rc<dyn Clipboard>>,
) -> Div {
    let (count, rows, cursor, of) = (state.rows.clone(), state.clone(), state.cursor, columns.clone());
    let body =
        uniform_list(move || count(), height, signal(0.0), move |place| line(&rows, &of, height, place));
    let body = body.container(div().grow(1.0)).reveal(move || cursor.get());
    let all = state.frame(column().grow(1.0).overflow_x(Overflow::Hidden), clipboard);
    // Columns wider than the table scroll sideways, the header with the rows.
    // ponytail: the rows' own scrollbar is at the right end of what scrolls sideways; a bar
    // at the table's edge when tables that wide are common.
    let sheet = column().grow(1.0).child(header(&state, &columns)).child(body);
    all.child(scroll_x(signal(0.0), sheet).container(div().grow(1.0)))
}
