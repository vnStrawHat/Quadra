//! What a table shows: its header, where a column is sorted and made wider, and its rows.

use std::rc::Rc;

use quadra_headless::table::Sheet;
use quadra_layout::{Align, Display, Justify, Length, Overflow};
use quadra_platform::{Clipboard, Cursor};
use quadra_reactive::{Signal, memo, signal};
use quadra_tree::PointerKind;
use quadra_tree::{Paint, Role, State as On};
use quadra_view::{
    AnyView, Div, View, by_text, column, div, dynamic, row, scroll_x, sideways_bar, text, uniform_list,
};

use super::{CellKind, Column, age_text};
use crate::icon::icon;
use crate::menu::{MenuItem, context_menu};
use crate::status::{Status, status_label};
use crate::{fonts, icons, token};

/// The width of the strip at the edge of a title that makes its column wider.
const GRIP: f32 = 5.0;

/// The metrics of a table: the header's height, the room at each side of a cell's words and
/// at each end of a row; dense, the header's words are small and muted.
#[derive(Clone, Copy)]
pub(crate) struct Look {
    head: f32,
    pad: f32,
    edge: f32,
    dense: bool,
}

impl Look {
    /// Dense: a header of 24 px in 12 px text, 12 px from a row's start to its first words
    /// and between the columns' words.
    pub(crate) fn of(dense: bool) -> Look {
        match dense {
            true => Look { head: 24.0, pad: 6.0, edge: 6.0, dense },
            false => Look { head: 28.0, pad: 8.0, edge: 0.0, dense },
        }
    }
}

/// The header: the titles on the plane of table heads over a line; the arrow of the column
/// the rows are sorted by is in the accent.
fn header(state: &Sheet, columns: &[Column], (pinned, sx, look): (usize, Signal<f32>, Look)) -> Div {
    let head = row().height(by_text(look.head)).shrink(0.0).padding_x(look.edge).bg(token::TABLE_HEAD_BG);
    let head = match look.dense {
        true => head.text_color(token::TEXT_2).text_size(crate::size::TEXT_SM.size),
        false => head.text_color(token::TABLE_HEAD_FG),
    };
    let head = head.border_bottom(1.0).border_color(token::LINE).role(Role::Row);
    let mut titles = Vec::new();
    for (c, at) in columns.iter().enumerate() {
        let (sort, widths) = (state.sort, state.widths);
        let arrow = dynamic(move || match sort.get() {
            Some((by, down)) if by == c => {
                let body = if down { icons::ARROW_DOWN } else { icons::ARROW_UP };
                div().child(icon(body).size(12.0).color(token::ACCENT_TEXT))
            }
            _ => div(),
        });
        let title = row().align_items(Align::Center).gap(4.0).grow(1.0).padding_left(look.pad);
        let title = title.overflow_x(Overflow::Hidden);
        let title = match at.counts() {
            true => title.justify(Justify::End).padding_right(2.0 * look.pad - GRIP),
            false => title,
        };
        let title = title.child(text(at.title.clone())).child(arrow);
        let title = match at.sortable {
            true => title.cursor(Cursor::Pointer).hover(|s| s.bg(token::PANEL_HOVER).text_color(token::TEXT)),
            false => title,
        };
        let title = state.title(title, c);
        let grip = div().width(GRIP).shrink(0.0).cursor(Cursor::ResizeHorizontal);
        let grip = state.grip(grip.hover(|s| s.bg(token::BORDER_FOCUSED)), c);
        let (fills, whole) =
            (state.fills, row().shrink(0.0).width(move || Length::Px(widths.with(|all| all[c]))));
        // A press on the edge of a column that fills makes it fixed at the width it has, which
        // the drag starts from: before the grip's handler, while the press goes down to it.
        let whole = whole.grow(move || if pinned > 0 { 0.0 } else { fills.with(|all| all[c]) }).on_mount(
            move |tree, id| {
                tree.on_pointer_capture(id, PointerKind::Down, move |e| {
                    if fills.with_untracked(|all| all[c] > 0.0) && e.local.x >= e.size.width - GRIP {
                        widths.update(|all| all[c] = e.size.width);
                        fills.update(|all| all[c] = 0.0);
                    }
                })
            },
        );
        titles.push(shown(state, c, whole.child(title).child(grip)));
    }
    split(head, titles, pinned, sx)
}

/// The menu of the columns, on the header with the other button: a row a column, checked
/// while it shows; choosing one hides or shows it. The last that shows stays.
fn columns_menu(head: Div, columns: Rc<Vec<Column>>, hidden: Signal<Vec<usize>>) -> impl View {
    context_menu(head, move || {
        let shut = hidden.get_untracked();
        let last = columns.len() - shut.len() == 1;
        let row = |(c, at): (usize, &Column)| {
            let on = !shut.contains(&c);
            let item = MenuItem::new(at.title.clone()).checked(on).disabled(on && last);
            item.on_select(move || {
                hidden.update(|all| match all.iter().position(|&h| h == c) {
                    Some(k) => _ = all.remove(k),
                    None => all.push(c),
                })
            })
        };
        columns.iter().enumerate().map(row).collect()
    })
}

/// A column's part of a row, which shows while the column is not hidden.
fn shown(state: &Sheet, c: usize, part: Div) -> Div {
    let hidden = state.hidden;
    part.display(move || if hidden.with(|all| all.contains(&c)) { Display::None } else { Display::Flex })
}

/// A row of parts: the pinned ones where they are, the others in a box that cuts them (a
/// layer cut to it: a box's overflow cuts its layout, not what is painted), moved left by
/// `sx`. With none pinned, all of them in the row.
fn split(row_of: Div, parts: Vec<Div>, pinned: usize, sx: Signal<f32>) -> Div {
    if pinned == 0 {
        return row_of.children(parts);
    }
    let mut parts = parts.into_iter();
    let fixed: Vec<Div> = parts.by_ref().take(pinned).collect();
    let rest = row().absolute().top(0.0).bottom(0.0).left(move || -sx.get());
    let rest = rest.children(parts.collect::<Vec<_>>()).cut_to_parent();
    let cut = row().width(0.0).grow(1.0).height(Length::Fill).overflow_x(Overflow::Hidden).child(rest);
    row_of.children(fixed).child(cut.border_left(1.0).border_color(token::LINE))
}

/// A row: its cells, each as its column shows them; tinted with a bar of the accent along all
/// of its left edge when selected (a row is a band), and outlined when the keys are on it and
/// the table has the focus.
fn line(
    state: &Sheet,
    columns: &[Column],
    (height, pinned, sx, look): (f32, usize, Signal<f32>, Look),
    place: Signal<usize>,
) -> Div {
    let (s, mark) = (state.clone(), state.clone());
    let data = move || s.data(place.get());
    let chosen = state.selected(place);
    let ring = move || -> Paint {
        let here = mark.column.is_none() && mark.focused.get() && mark.cursor.get() == Some(place.get());
        if here { token::BORDER_FOCUSED.into() } else { Paint::default() }
    };
    let line = row().align_items(Align::Center).grow(1.0).height(by_text(height)).padding_x(look.edge);
    let line = line.border_bottom(1.0).border_color(token::TABLE_ROW_BORDER).outline(ring);
    let line = line
        .hover(|s| s.bg(token::RAISED_HOVER.alpha(0.6)))
        .when(On::SELECTED, |s| s.bg(token::ACCENT.alpha(0.1)));
    let line = state.row(line, place);
    let changed = state.live.map(|live| super::live::changed(live, state, place));
    let mut cells = Vec::new();
    for (c, at) in columns.iter().enumerate() {
        let (widths, says, row_of) = (state.widths, state.cell.clone(), data.clone());
        let value = move || says(row_of(), c);
        let cell = row().align_items(Align::Center).shrink(0.0).padding_x(look.pad);
        let (focused, here) = (state.focused, state.here(place, c));
        let ring = move || -> Paint {
            if focused.get() && here() { token::BORDER_FOCUSED.into() } else { Paint::default() }
        };
        let cell = state.cell(cell.height(Length::Fill).outline(ring), place, c);
        let cell = cell.overflow_x(Overflow::Hidden).width(move || Length::Px(widths.with(|all| all[c])));
        let fills = state.fills;
        let cell = cell.grow(move || if pinned > 0 { 0.0 } else { fills.with(|all| all[c]) });
        // Under what it says, the tint of a change.
        let cell = match (changed, state.live) {
            (Some(changed), Some(live)) => cell.child(super::live::tint(live, changed, c)),
            _ => cell,
        };
        // Numbers are in the mono face at the right of their cells, so columns line up.
        let cell = match at.counts() {
            true => cell.justify(Justify::End).padding_right(2.0 * look.pad).font_family(fonts::MONO),
            false => cell,
        };
        let cell = if at.mono { cell.font_family(fonts::MONO) } else { cell };
        let cell = if at.muted { cell.text_color(token::TEXT_3) } else { cell };
        let row_of = data.clone();
        cells.push(shown(
            state,
            c,
            match (&at.shows, at.kind) {
                (Some(shows), _) => cell.child(shows(memo(row_of), memo(value))),
                // One line, cut with "…" where the column ends.
                (None, CellKind::Text) => cell.child(text(value).ellipsis()),
                (None, CellKind::Age) => cell
                    .child(text(move || value().trim().parse().map(age_text).unwrap_or_default()).ellipsis()),
                (None, CellKind::Status) => cell.child(dynamic(move || {
                    let word = value();
                    status_label(Status::of(&word), &word)
                })),
            },
        ));
    }
    // After the cells: what reads a row by its children finds them where they were.
    let shown = move || if chosen() { Display::Flex } else { Display::None };
    let mark = div().absolute().left(0.0).top(0.0).bottom(0.0).width(2.0).bg(token::ACCENT).display(shown);
    split(line, cells, pinned, sx).child(mark)
}

/// The table: its header over its rows, with the focus and the keys on the whole.
pub(crate) fn build(
    state: Sheet,
    columns: Rc<Vec<Column>>,
    (height, pinned, look): (f32, usize, Look),
    clipboard: Option<Rc<dyn Clipboard>>,
) -> AnyView {
    let (count, rows, cursor, of) = (state.rows.clone(), state.clone(), state.cursor, columns.clone());
    let sx = signal(0.0f32);
    // With columns pinned, the others move under the wheel sideways, as far as they go past
    // the room they have.
    let (widths, hidden) = (state.widths, state.hidden);
    let end = move |width: f32| {
        let (all, hidden) = (widths.get(), hidden.get()); // tracked where a bar follows it
        let wide = |range: std::ops::Range<usize>| {
            range.filter(|c| !hidden.contains(c)).map(|c| all[c]).sum::<f32>()
        };
        wide(pinned..all.len()) - (width - wide(0..pinned) - 1.0 - 2.0 * look.edge)
    };
    let body = uniform_list(
        move || count(),
        height,
        signal(0.0),
        move |place| {
            let line = line(&rows, &of, (height, pinned, sx, look), place);
            if pinned == 0 { line } else { crate::rows::sideways(line, sx, end) }
        },
    );
    let body = body.container(div().grow(1.0)).reveal(move || cursor.get());
    let all = state.frame(column().grow(1.0).overflow_x(Overflow::Hidden), clipboard);
    let head = header(&state, &columns, (pinned, sx, look));
    if pinned > 0 {
        let head = columns_menu(crate::rows::sideways(head, sx, end), columns, hidden);
        // A bar along the bottom: what scrolls has no box of its own to have one.
        return AnyView::new(sideways_bar(all.child(head).child(body), sx, end));
    }
    let head = columns_menu(head, columns, hidden);
    // Columns wider than the table scroll sideways, the header with the rows.
    // ponytail: the rows' own scrollbar is at the right end of what scrolls sideways; a bar
    // at the table's edge when tables that wide are common.
    let sheet = column().grow(1.0).child(head).child(body);
    AnyView::new(all.child(scroll_x(signal(0.0), sheet).container(div().grow(1.0))))
}
