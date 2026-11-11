//! Grid placement: which cells each item takes.

use std::collections::HashSet;

use crate::grid::{GridStyle, parse_areas};
use crate::tree::{LayoutId, LayoutTree};

/// An item and its cells, counted from zero.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GridItem {
    pub id: LayoutId,
    pub row: usize,
    pub col: usize,
    pub rows: usize,
    pub cols: usize,
}

type Taken = HashSet<(usize, usize)>;

fn is_free(taken: &Taken, row: usize, col: usize, rows: usize, cols: usize) -> bool {
    (row..row + rows).all(|r| (col..col + cols).all(|c| !taken.contains(&(r, c))))
}

fn take(taken: &mut Taken, it: &GridItem) {
    for r in it.row..it.row + it.rows {
        taken.extend((it.col..it.col + it.cols).map(|c| (r, c)));
    }
}

impl LayoutTree {
    /// Places the in-flow children of a grid: items with an area or with both lines first,
    /// then the rest in order, into the first free cells.
    pub(crate) fn place_grid_items(
        &self,
        ids: &[LayoutId],
        g: &GridStyle,
        ncols: usize,
        nrows: usize,
    ) -> Vec<GridItem> {
        let areas = parse_areas(&g.areas);
        let (ncols, nrows) = (ncols.max(1), nrows.max(1));
        let mut taken = Taken::new();
        let mut items = Vec::with_capacity(ids.len());
        // (index into `items`, fixed row, fixed column) of the items still to place.
        let mut pending = Vec::new();
        let none = GridStyle::default();
        for &id in ids {
            let cg = self.nodes[id.index()].style.grid.as_deref().unwrap_or(&none);
            let mut it = GridItem { id, row: 0, col: 0, rows: cg.row_span.max(1), cols: cg.col_span.max(1) };
            let (mut row, mut col) = (cg.row_start.checked_sub(1), cg.col_start.checked_sub(1));
            if let Some(area) = areas.get(cg.area.as_str()) {
                it = GridItem { id, row: area.row, col: area.col, rows: area.rows, cols: area.cols };
                (row, col) = (Some(area.row), Some(area.col));
            }
            if let (Some(row), Some(col)) = (row, col) {
                (it.row, it.col) = (row, col);
                take(&mut taken, &it);
            } else {
                pending.push((items.len(), row, col));
            }
            items.push(it);
        }
        let (mut cr, mut cc) = (0, 0);
        for (index, row, col) in pending {
            let it = &mut items[index];
            if g.dense {
                (cr, cc) = (0, 0);
            }
            // An item wider than the grid, or in a fixed row that is full, takes columns
            // beyond the defined ones instead of never fitting.
            let wrap_at = if row.is_some() { usize::MAX } else { ncols.max(it.cols) };
            loop {
                let (r, c) = (row.unwrap_or(cr), col.unwrap_or(cc));
                let inside = g.column_flow || col.is_some() || c.saturating_add(it.cols) <= wrap_at;
                if inside && is_free(&taken, r, c, it.rows, it.cols) {
                    (it.row, it.col) = (r, c);
                    break;
                }
                // Move along the flow; a fixed line leaves only the other axis to move on.
                if g.column_flow {
                    if row.is_some() {
                        cc += 1;
                    } else {
                        cr += 1;
                        if cr >= nrows {
                            (cr, cc) = (0, cc + 1);
                        }
                    }
                } else if col.is_some() {
                    cr += 1;
                } else {
                    cc += 1;
                    if cc.saturating_add(it.cols) > wrap_at {
                        (cc, cr) = (0, cr + 1);
                    }
                }
            }
            take(&mut taken, it);
            if g.column_flow {
                cr = it.row + it.rows;
            } else {
                cc = it.col + it.cols;
            }
        }
        items
    }
}
