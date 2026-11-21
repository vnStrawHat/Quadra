//! The table (specs/behaviour/09, "Data"): any number of rows of cells under a header.
//! Rows are mounted as they show; a column is sorted by a click on its title and made wider
//! by dragging its edge; rows are selected by the pointer and the keys, and copied.

mod live;
mod view;

use std::rc::Rc;

use quadra_headless::live::Key;
use quadra_headless::table::{self, Activate, Cells, Rows};
use quadra_platform::Clipboard;
use quadra_reactive::{Memo, Signal, use_context};
use quadra_tree::NodeId;
use quadra_view::{AnyView, MountCx, View};

/// How a column shows what its cells say.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CellKind {
    #[default]
    Text,
    /// The word of a status, shown with its mark in its colour.
    Status,
    /// Seconds, shown as an age (45s, 12m, 3h, 6d) and sorted as a number.
    Age,
}

/// What builds a cell's view from its row, as the data numbers it, and what it says.
pub(crate) type Shows = Rc<dyn Fn(Memo<usize>, Memo<String>) -> AnyView>;

#[derive(Clone)]
pub struct Column {
    title: String,
    width: f32,
    least: f32,
    sortable: bool,
    numeric: bool,
    kind: CellKind,
    mono: bool,
    muted: bool,
    shows: Option<Shows>,
}

impl Column {
    /// A column of text, 140 px wide, that a click on its title sorts.
    pub fn new(title: impl Into<String>) -> Column {
        let (title, kind) = (title.into(), CellKind::Text);
        let (mono, muted, shows) = (false, false, None);
        Column { title, width: 140.0, least: 48.0, sortable: true, numeric: false, kind, mono, muted, shows }
    }

    pub fn width(mut self, px: f32) -> Self {
        self.width = px;
        self
    }

    /// Numbers: at the right of their cells in the mono face, and sorted as numbers.
    pub fn numeric(mut self) -> Self {
        self.numeric = true;
        self
    }

    pub fn kind(mut self, kind: CellKind) -> Self {
        self.kind = kind;
        self
    }

    /// Names and images: in the mono face, at the left.
    pub fn mono(mut self) -> Self {
        self.mono = true;
        self
    }

    /// Notes: in the quiet text.
    pub fn muted(mut self) -> Self {
        self.muted = true;
        self
    }

    /// What a cell shows, built once for each row that is mounted from the row as the data
    /// numbers it and what the cell says: memos, which follow a reused row and a change.
    /// What the cell says is still what sorts, is copied and is compared.
    pub fn view<V: View + 'static>(
        mut self,
        view: impl Fn(Memo<usize>, Memo<String>) -> V + 'static,
    ) -> Self {
        self.shows = Some(Rc::new(move |row, says| AnyView::new(view(row, says))));
        self
    }

    /// A click on the title does not sort by this column.
    pub fn unsorted(mut self) -> Self {
        self.sortable = false;
        self
    }

    fn counts(&self) -> bool {
        self.numeric || self.kind == CellKind::Age
    }

    /// What the headless crate knows of it: an age is sorted as a number, and changes all the time.
    fn base(&self) -> table::Column {
        let (width, least, sortable, counts) = (self.width, self.least, self.sortable, self.counts());
        table::Column { width, least, sortable, counts, restless: self.kind == CellKind::Age }
    }
}

/// Seconds as tools write an age: 45s, 12m, 3h, 6d.
pub fn age_text(seconds: i64) -> String {
    match seconds {
        s if s < 60 => format!("{s}s"),
        s if s < 3600 => format!("{}m", s / 60),
        s if s < 86400 => format!("{}h", s / 3600),
        s => format!("{}d", s / 86400),
    }
}

pub struct Table {
    columns: Vec<Column>,
    rows: Rows,
    cell: Cells,
    chosen: Option<Signal<Vec<usize>>>,
    key: Option<Key>,
    row_height: f32,
    hidden: Option<Signal<Vec<usize>>>,
    pinned: usize,
    column: Option<Signal<usize>>,
    changes: Option<Signal<Vec<usize>>>,
    activate: Option<Activate>,
}

/// A table of `rows` rows: `cell(row, column)` is what a cell says. Both are asked again
/// when what they read changes. Give the table a height, or let it grow.
pub fn table(
    columns: impl IntoIterator<Item = Column>,
    rows: impl Fn() -> usize + 'static,
    cell: impl Fn(usize, usize) -> String + 'static,
) -> Table {
    let columns = columns.into_iter().collect();
    let (rows, cell) = (Rc::new(rows), Rc::new(cell));
    let (chosen, key, hidden, column, changes) = (None, None, None, None, None);
    let (row_height, pinned, activate) = (26.0, 0, None);
    Table { columns, rows, cell, chosen, key, row_height, hidden, pinned, column, changes, activate }
}

impl Table {
    /// The signal that holds the selected rows, as they are numbered in the data, ascending.
    pub fn selected(mut self, rows: Signal<Vec<usize>>) -> Self {
        self.chosen = Some(rows);
        self
    }

    pub fn row_height(mut self, px: f32) -> Self {
        self.row_height = px;
        self
    }

    /// The columns, by their places, that do not show and are not copied: a signal the
    /// caller writes (from a menu of the columns, say).
    pub fn hidden(mut self, columns: Signal<Vec<usize>>) -> Self {
        self.hidden = Some(columns);
        self
    }

    /// A cursor on cells: the column of the cell the keys are on, a signal the caller may
    /// read. The left and right arrows move it, a click on a cell puts it there, it is
    /// outlined while the table has the focus, and Ctrl-C copies what that cell says.
    pub fn cells(mut self, column: Signal<usize>) -> Self {
        self.column = Some(column);
        self
    }

    /// The first `columns` columns stay where they are while the others scroll sideways
    /// under the wheel turned so, or with Shift.
    pub fn pinned(mut self, columns: usize) -> Self {
        self.pinned = columns;
        self
    }

    /// Live data: what each row is known by. The selection and the row the keys are on
    /// stay with their rows when the data changes, and a cell that says something else
    /// than it did is tinted for a moment. Every row is read at each change: for
    /// thousands of rows, not millions.
    pub fn keys(mut self, key: impl Fn(usize) -> String + 'static) -> Self {
        self.key = Some(Rc::new(key));
        self
    }

    /// What opens a row, given as the data numbers it: a double click on the row, or Enter on
    /// the row the keys are on. The arrows only move.
    pub fn on_activate(mut self, f: impl Fn(usize) + 'static) -> Self {
        self.activate = Some(Rc::new(f));
        self
    }

    /// With live data, the source says which rows (as the data numbers them) it changed,
    /// setting `rows` after each change: the cells of those are read, not of every row.
    pub fn changes(mut self, rows: Signal<Vec<usize>>) -> Self {
        self.changes = Some(rows);
        self
    }
}

impl View for Table {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let columns = Rc::new(self.columns);
        let known = columns.iter().map(Column::base).collect();
        let mut sheet = table::sheet(known, self.rows, self.cell, self.chosen);
        if let Some(hidden) = self.hidden {
            sheet.hidden = hidden;
        }
        (sheet.column, sheet.activate) = (self.column, self.activate);
        if let Some(key) = self.key {
            quadra_headless::live::watch(&mut sheet, key, self.changes, cx.handle.clone());
        }
        let clipboard = use_context::<Rc<dyn Clipboard>>();
        let pinned = self.pinned.min(columns.len());
        view::build(sheet, columns, (self.row_height, pinned), clipboard).mount(cx)
    }
}
