//! The table (specs/behaviour/09, "Data"): any number of rows of cells under a header.
//! Rows are mounted as they show; a column is sorted by a click on its title and made wider
//! by dragging its edge; rows are selected by the pointer and the keys, and copied.

mod view;

use std::rc::Rc;

use quadra_platform::{Clipboard, Mods};
use quadra_reactive::{Memo, Signal, memo, signal, use_context};
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

type Rows = Rc<dyn Fn() -> usize>;
type Cell = Rc<dyn Fn(usize, usize) -> String>;

pub struct Table {
    columns: Vec<Column>,
    rows: Rows,
    cell: Cell,
    chosen: Option<Signal<Vec<usize>>>,
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
    Table { columns, rows: Rc::new(rows), cell: Rc::new(cell), chosen: None, row_height: 26.0 }
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
}

/// What a mounted table keeps: its data, and what the user did to it.
#[derive(Clone)]
pub(crate) struct State {
    columns: Rc<Vec<Column>>,
    rows: Rows,
    cell: Cell,
    widths: Signal<Vec<f32>>,
    /// The column the rows are sorted by, and whether from the most.
    sort: Signal<Option<(usize, bool)>>,
    /// The data row at each place of the table; empty: as the data has them.
    order: Memo<Rc<Vec<usize>>>,
    chosen: Signal<Vec<usize>>,
    /// The place of the row the keys are on, and of the one a range is selected from.
    cursor: Signal<Option<usize>>,
    anchor: Signal<Option<usize>>,
    focused: Signal<bool>,
    row_height: f32,
}

impl State {
    /// The data row at a place of the table.
    fn data(&self, place: usize) -> usize {
        self.order.get().get(place).copied().unwrap_or(place)
    }

    /// Sorts by a column: from the least, or the other way when it is sorted by already.
    fn sort_by(&self, column: usize) {
        self.sort.update(|sort| *sort = Some((column, sort.is_some_and(|(by, down)| by == column && !down))));
    }

    /// A click on the row at a place: it alone is selected; with Ctrl it is added or taken
    /// away; with Shift the rows from the last one clicked to it are.
    fn pick(&self, place: usize, mods: Mods) {
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
    fn step(&self, to: impl Fn(usize, usize) -> usize, extend: bool) {
        let Some(last) = (self.rows)().checked_sub(1) else { return };
        let place = self.cursor.get_untracked().map_or(0, |at| to(at, last).min(last));
        self.pick(place, if extend { Mods::SHIFT } else { Mods::NONE });
    }

    /// The selected rows as text: in the order they show, a tab between cells.
    fn copy_text(&self) -> String {
        let (chosen, count) = (self.chosen.get_untracked(), (self.rows)());
        let shown = (0..count).map(|place| self.data(place)).filter(|row| chosen.binary_search(row).is_ok());
        let line = |row: usize| {
            let cells: Vec<String> = (0..self.columns.len()).map(|c| (self.cell)(row, c)).collect();
            cells.join("\t")
        };
        shown.map(line).collect::<Vec<_>>().join("\n")
    }
}

impl View for Table {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (columns, rows, cell) = (Rc::new(self.columns), self.rows, self.cell);
        let sort: Signal<Option<(usize, bool)>> = signal(None);
        let (by, count, says) = (columns.clone(), rows.clone(), cell.clone());
        // Sorted again when the rows or what they say change: the memo reads them.
        let order = memo(move || {
            let Some((column, down)) = sort.get() else { return Rc::new(Vec::new()) };
            let mut order: Vec<usize> = (0..count()).collect();
            if by[column].counts() {
                let keys: Vec<f64> =
                    order.iter().map(|&row| says(row, column).trim().parse().unwrap_or(0.0)).collect();
                order.sort_by(|&a, &b| keys[a].total_cmp(&keys[b]));
            } else {
                let keys: Vec<String> = order.iter().map(|&row| says(row, column).to_lowercase()).collect();
                order.sort_by(|&a, &b| keys[a].cmp(&keys[b]));
            }
            if down {
                order.reverse();
            }
            Rc::new(order)
        });
        let state = State {
            widths: signal(columns.iter().map(|column| column.width).collect()),
            chosen: self.chosen.unwrap_or_else(|| signal(Vec::new())),
            cursor: signal(None),
            anchor: signal(None),
            focused: signal(false),
            row_height: self.row_height,
            columns,
            rows,
            cell,
            sort,
            order,
        };
        view::build(state, use_context::<Rc<dyn Clipboard>>()).mount(cx)
    }
}
