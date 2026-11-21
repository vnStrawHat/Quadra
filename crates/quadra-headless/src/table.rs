//! The table (specs/17, "The components"): any number of rows of cells. A click on a title
//! sorts by its column from the least, another from the most, worked out again when the data
//! changes; the edge of a title is dragged to make its column wider, never under its least. A
//! click selects a row, with Ctrl one more or one less, with Shift from the last one clicked;
//! the arrows, the page keys, Home and End move the row the keys are on and select it, Shift
//! with an arrow grows the selection, and the key for copying copies the selected rows as
//! lines of cells with tabs between. The look gives the frame, the titles, the grips, the
//! rows and the cells.

use std::cell::Cell as Shared;
use std::rc::Rc;

use quadra_platform::{Clipboard, Mods};
use quadra_reactive::{Memo, Signal, memo, signal};
use quadra_tree::{PointerKind, Role};
use quadra_view::Div;

use crate::action::{Copy, ExtendNext, ExtendPrevious, MoveFirst, MoveLast, MoveNext, MovePrevious, TABLE};
use crate::action::{MoveInto, MoveOut, PageNext, PagePrevious};
pub use crate::live::{FLASH, Live};

/// How many rows a page key moves by.
pub const PAGE: usize = 20;

/// What the headless crate knows of a column.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Column {
    pub width: f32,
    /// The least it is dragged to.
    pub least: f32,
    /// A click on its title sorts by it.
    pub sortable: bool,
    /// Its cells are numbers, and sorted as numbers.
    pub counts: bool,
    /// Its cells change all the time (an age): a live table does not mark their changes.
    pub restless: bool,
}

pub type Rows = Rc<dyn Fn() -> usize>;
pub type Cells = Rc<dyn Fn(usize, usize) -> String>;

/// A mounted table: its data, and what the user did to it.
#[derive(Clone)]
pub struct Sheet {
    pub columns: Rc<Vec<Column>>,
    pub rows: Rows,
    pub cell: Cells,
    pub widths: Signal<Vec<f32>>,
    /// The column the rows are sorted by, and whether from the most.
    pub sort: Signal<Option<(usize, bool)>>,
    /// The data row at each place of the table; empty: as the data has them.
    pub order: Memo<Rc<Vec<usize>>>,
    pub chosen: Signal<Vec<usize>>,
    /// The place of the row the keys are on, and of the one a range is selected from.
    pub cursor: Signal<Option<usize>>,
    pub anchor: Signal<Option<usize>>,
    pub focused: Signal<bool>,
    /// The columns that do not show, and are not copied.
    pub hidden: Signal<Vec<usize>>,
    /// With a cursor on cells, the column of the cell the keys are on: the left and right
    /// arrows move it, a click on a cell puts it there, and the key for copying copies it.
    pub column: Option<Signal<usize>>,
    /// What a table of live data keeps.
    pub live: Option<Live>,
}

/// A table of `rows()` rows of `columns`: `cell(row, column)` is what a cell says. `chosen`
/// holds the selected rows as the data numbers them, ascending: the caller's, if given.
pub fn sheet(columns: Vec<Column>, rows: Rows, cell: Cells, chosen: Option<Signal<Vec<usize>>>) -> Sheet {
    let (columns, sort) = (Rc::new(columns), signal(None::<(usize, bool)>));
    let (by, count, says) = (columns.clone(), rows.clone(), cell.clone());
    // Sorted again when the rows or what they say change: the memo reads them.
    let order = memo(move || {
        let Some((column, down)) = sort.get() else { return Rc::new(Vec::new()) };
        let mut order: Vec<usize> = (0..count()).collect();
        let mut blank = Vec::new();
        if by[column].counts {
            let keys: Vec<f64> =
                order.iter().map(|&row| says(row, column).trim().parse().unwrap_or(f64::NAN)).collect();
            order.sort_by(|&a, &b| keys[a].total_cmp(&keys[b]));
            blank = keys.iter().map(|key| key.is_nan()).collect();
        } else {
            let keys: Vec<String> = order.iter().map(|&row| says(row, column).to_lowercase()).collect();
            order.sort_by(|&a, &b| keys[a].cmp(&keys[b]));
        }
        if down {
            order.reverse();
        }
        // What says no number comes after every number, whichever way.
        if !blank.is_empty() {
            order.sort_by_key(|&row| blank[row]);
        }
        Rc::new(order)
    });
    let widths = signal(columns.iter().map(|column| column.width).collect());
    let chosen = chosen.unwrap_or_else(|| signal(Vec::new()));
    let (cursor, anchor, focused, hidden) = (signal(None), signal(None), signal(false), signal(Vec::new()));
    let (live, column) = (None, None);
    Sheet { columns, rows, cell, widths, sort, order, chosen, cursor, anchor, focused, hidden, live, column }
}

impl Sheet {
    /// The data row at a place of the table.
    pub fn data(&self, place: usize) -> usize {
        self.order.get().get(place).copied().unwrap_or(place)
    }

    /// Sorts by a column: from the least, or the other way when it is sorted by already.
    pub fn sort_by(&self, column: usize) {
        self.sort.update(|sort| *sort = Some((column, sort.is_some_and(|(by, down)| by == column && !down))));
    }

    /// A click on the row at a place: it alone is selected; with Ctrl it is added or taken
    /// away; with Shift the rows from the last one clicked to it are.
    pub fn pick(&self, place: usize, mods: Mods) {
        let row = self.data(place);
        match (self.anchor.get_untracked(), mods.contains(Mods::SHIFT), mods.contains(Mods::CTRL)) {
            (Some(from), true, _) => {
                let mut rows: Vec<usize> =
                    (from.min(place)..=from.max(place)).map(|p| self.data(p)).collect();
                rows.sort_unstable();
                self.chosen.set(rows);
            }
            (_, _, true) => {
                self.chosen.update(|rows| match rows.binary_search(&row) {
                    Ok(at) => drop(rows.remove(at)),
                    Err(at) => rows.insert(at, row),
                });
                self.anchor.set(Some(place));
            }
            _ => {
                self.chosen.set(vec![row]);
                self.anchor.set(Some(place));
            }
        }
        self.cursor.set(Some(place));
    }

    /// Moves the row the keys are on: `to` is given where it is and the last place.
    pub fn step(&self, to: impl Fn(usize, usize) -> usize, extend: bool) {
        let Some(last) = (self.rows)().checked_sub(1) else { return };
        let place = self.cursor.get_untracked().map_or(0, |at| to(at, last).min(last));
        self.pick(place, if extend { Mods::SHIFT } else { Mods::NONE });
    }

    /// Moves the column of the cell the keys are on by one, past the hidden ones.
    pub fn step_column(&self, by: isize) {
        let Some(column) = self.column else { return };
        let hidden = self.hidden.get_untracked();
        let mut at = column.get_untracked() as isize;
        loop {
            at += by;
            if at < 0 || at as usize >= self.columns.len() {
                return;
            }
            if !hidden.contains(&(at as usize)) {
                return column.set(at as usize);
            }
        }
    }

    /// Whether the keys are on the cell of column `c` at a place: a reactive read.
    pub fn here(&self, place: Signal<usize>, c: usize) -> impl Fn() -> bool + Clone + 'static {
        let (cursor, column) = (self.cursor, self.column);
        move || column.is_some_and(|column| column.get() == c) && cursor.get() == Some(place.get())
    }

    /// The cell of column `c` at a place on its face: a cell to a reader, selected while
    /// the keys are on it; with a cursor on cells a click puts it there.
    pub fn cell(&self, face: Div, place: Signal<usize>, c: usize) -> Div {
        let face = face.role(Role::Cell);
        let Some(column) = self.column else { return face };
        face.selected(self.here(place, c)).on_click(move |_| column.set(c))
    }

    /// The selected rows as text: in the order they show, a tab between cells; with a
    /// cursor on cells, what the cell the keys are on says.
    pub fn copy_text(&self) -> String {
        if let (Some(column), Some(place)) = (self.column, self.cursor.get_untracked()) {
            return (self.cell)(self.data(place), column.get_untracked());
        }
        let (chosen, count) = (self.chosen.get_untracked(), (self.rows)());
        let shown = (0..count).map(|place| self.data(place)).filter(|row| chosen.binary_search(row).is_ok());
        let hidden = self.hidden.get_untracked();
        let line = |row: usize| {
            let shown = (0..self.columns.len()).filter(|c| !hidden.contains(c));
            shown.map(|c| (self.cell)(row, c)).collect::<Vec<_>>().join("\t")
        };
        shown.map(line).collect::<Vec<_>>().join("\n")
    }

    /// Whether the row at a place is selected: a reactive read.
    pub fn selected(&self, place: Signal<usize>) -> impl Fn() -> bool + Clone + 'static {
        let on = self.clone();
        move || on.chosen.with(|rows| rows.binary_search(&on.data(place.get())).is_ok())
    }

    /// The title of column `c` on its face: a cell to a reader; a click sorts by it if it may.
    pub fn title(&self, face: Div, c: usize) -> Div {
        let (face, sorts) = (face.role(Role::Cell), self.clone());
        if self.columns[c].sortable { face.on_click(move |_| sorts.sort_by(c)) } else { face }
    }

    /// The edge of the title of column `c` on its face: dragged with the pointer held, the
    /// column is as wide as it was and the pointer has moved, and never under its least.
    pub fn grip(&self, face: Div, c: usize) -> Div {
        let (widths, least) = (self.widths, self.columns[c].least);
        let held: Rc<Shared<Option<(f32, f32)>>> = Rc::default();
        let (press, drag) = (held.clone(), held.clone());
        let face = face.on_pointer(PointerKind::Down, move |e| {
            press.set(Some((e.at.x, widths.with_untracked(|all| all[c]))));
            e.capture_pointer();
            e.stop();
        });
        let face = face.on_pointer(PointerKind::Move, move |e| {
            if let Some((from, was)) = drag.get() {
                widths.update(|all| all[c] = (was + e.at.x - from).max(least));
            }
        });
        face.on_pointer(PointerKind::Up, move |_| held.set(None))
    }

    /// The row at a place on its face: a row to a reader, selected or not; a click picks it.
    pub fn row(&self, face: Div, place: Signal<usize>) -> Div {
        let pick = self.clone();
        let face = face.role(Role::Row).selected(self.selected(place));
        face.on_click(move |e| pick.pick(place.get_untracked(), e.mods))
    }

    /// The frame of the table: a table to a reader, with the focus and the keys. The key for
    /// copying writes the selected rows to `clipboard`.
    pub fn frame(&self, frame: Div, clipboard: Option<Rc<dyn Clipboard>>) -> Div {
        let focused = self.focused;
        let frame =
            frame.role(Role::Table).focusable().key_context(TABLE).on_focus(move |has| focused.set(has));
        // The arrows move the row the keys are on and select it; with Shift, from where it was.
        let moves = |extend: bool, to: fn(usize, usize) -> usize| {
            let sheet = self.clone();
            move || sheet.step(to, extend)
        };
        let (next, previous) = (moves(false, |at, _| at + 1), moves(false, |at, _| at.saturating_sub(1)));
        let (more, less) = (moves(true, |at, _| at + 1), moves(true, |at, _| at.saturating_sub(1)));
        let (first, last) = (moves(false, |_, _| 0), moves(false, |_, last| last));
        let (down, up) = (moves(false, |at, _| at + PAGE), moves(false, |at, _| at.saturating_sub(PAGE)));
        let frame =
            frame.on_action::<MoveNext>(move |_| next()).on_action::<MovePrevious>(move |_| previous());
        let frame =
            frame.on_action::<ExtendNext>(move |_| more()).on_action::<ExtendPrevious>(move |_| less());
        let frame = frame.on_action::<MoveFirst>(move |_| first()).on_action::<MoveLast>(move |_| last());
        let frame = frame.on_action::<PageNext>(move |_| down()).on_action::<PagePrevious>(move |_| up());
        let (right, left) = (self.clone(), self.clone());
        let frame = frame.on_action::<MoveInto>(move |_| right.step_column(1));
        let frame = frame.on_action::<MoveOut>(move |_| left.step_column(-1));
        let copies = self.clone();
        frame.on_action::<Copy>(move |_| {
            let text = copies.copy_text();
            if let (false, Some(clipboard)) = (text.is_empty(), &clipboard) {
                clipboard.write_text(&text);
            }
        })
    }
}
