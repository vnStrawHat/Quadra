//! The table (specs/behaviour/09, "Data"): any number of rows of cells under a header.
//! Rows are mounted as they show; a column is sorted by a click on its title and made wider
//! by dragging its edge; rows are selected by the pointer and the keys, and copied.

mod live;
mod view;

use std::rc::Rc;

use quadra_base::live::Key;
use quadra_base::table::{self, Cells, Rows};
use quadra_platform::Clipboard;
use quadra_reactive::{Signal, use_context};
use quadra_tree::NodeId;
use quadra_ui::{MountCx, View};

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

#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    title: String,
    width: f32,
    least: f32,
    sortable: bool,
    numeric: bool,
    kind: CellKind,
}

impl Column {
    /// A column of text, 140 px wide, that a click on its title sorts.
    pub fn new(title: impl Into<String>) -> Column {
        let (title, kind) = (title.into(), CellKind::Text);
        Column { title, width: 140.0, least: 48.0, sortable: true, numeric: false, kind }
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

    /// A click on the title does not sort by this column.
    pub fn unsorted(mut self) -> Self {
        self.sortable = false;
        self
    }

    fn counts(&self) -> bool {
        self.numeric || self.kind == CellKind::Age
    }

    /// What the base knows of it: an age is sorted as a number, and changes all the time.
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
    Table { columns, rows, cell, chosen: None, key: None, row_height: 26.0 }
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

    /// Live data: what each row is known by. The selection and the row the keys are on
    /// stay with their rows when the data changes, and a cell that says something else
    /// than it did is tinted for a moment. Every row is read at each change: for
    /// thousands of rows, not millions.
    pub fn keys(mut self, key: impl Fn(usize) -> String + 'static) -> Self {
        self.key = Some(Rc::new(key));
        self
    }
}

impl View for Table {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let columns = Rc::new(self.columns);
        let known = columns.iter().map(Column::base).collect();
        let mut sheet = table::sheet(known, self.rows, self.cell, self.chosen);
        if let Some(key) = self.key {
            quadra_base::live::watch(&mut sheet, key, cx.handle.clone());
        }
        view::build(sheet, columns, self.row_height, use_context::<Rc<dyn Clipboard>>()).mount(cx)
    }
}
