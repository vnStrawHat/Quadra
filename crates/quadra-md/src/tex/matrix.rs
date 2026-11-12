//! The rows of an environment: cells in columns, the rows about the axis, between the
//! delimiters the environment names.

use super::layout::{Laid, Lay};
use super::{Class, List};

impl Lay<'_> {
    pub(super) fn matrix(&mut self, env: &str, rows: &[Vec<List>], style: usize) -> Laid {
        // Aligned: columns in pairs, the first of a pair to the right and the second to the
        // left, with no space between them (`a &= b`).
        let aligned = matches!(env, "aligned" | "align" | "align*" | "split");
        let cases = env == "cases";
        let cell_style = match env {
            _ if aligned => style,
            "gathered" => style,
            "smallmatrix" => 2,
            _ => 1,
        };
        let em = self.em(style);
        let mut cells: Vec<Vec<Laid>> = Vec::new();
        for row in rows {
            let lay = |(column, cell): (usize, &List)| {
                // The second of a pair starts as if after an atom: `&= b` has its space.
                let after = (aligned && column % 2 == 1).then_some(Class::Ord);
                self.list_after(cell, cell_style, after)
            };
            cells.push(row.iter().enumerate().map(lay).collect());
        }
        let columns = cells.iter().map(Vec::len).max().unwrap_or(0);
        let mut widths = vec![0.0f32; columns];
        for row in &cells {
            for (width, cell) in widths.iter_mut().zip(row) {
                *width = width.max(cell.width);
            }
        }
        // What is after a column, when another follows it.
        let gap = |column: usize| match (aligned, cases) {
            (true, _) if column % 2 == 1 => em * 2.0,
            (true, _) => 0.0,
            (false, true) => em,
            (false, false) => em * 0.9,
        };
        let row_gap = em * if aligned { 0.3 } else { 0.12 };
        let (mut grid, mut top) = (Laid::default(), 0.0f32);
        for row in cells {
            let height = row.iter().fold(em * 0.75, |height, cell| height.max(cell.height));
            let depth = row.iter().fold(em * 0.3, |depth, cell| depth.max(cell.depth));
            let mut x = 0.0;
            for (column, cell) in row.into_iter().enumerate() {
                let spare = widths[column] - cell.width;
                let cell_x = match (aligned, cases) {
                    (true, _) if column % 2 == 0 => x + spare,
                    (true, _) | (_, true) => x,
                    _ => x + spare / 2.0,
                };
                grid.put(cell, cell_x, top + height);
                x += widths[column] + gap(column);
            }
            top += height + depth + row_gap;
        }
        let total = (top - row_gap).max(0.0);
        let width: f32 = widths.iter().enumerate().map(|(column, width)| width + gap(column)).sum();
        let width = width - if columns > 0 { gap(columns - 1) } else { 0.0 };
        // The rows, their middle on the axis.
        let shift = -(total / 2.0 + self.axis(style));
        let mut laid = Laid::sized(width, -shift, total + shift);
        laid.put(grid, 0.0, shift);
        let (left, right) = match env {
            "pmatrix" => ("(", ")"),
            "bmatrix" => ("[", "]"),
            "Bmatrix" => ("{", "}"),
            "vmatrix" => ("|", "|"),
            "Vmatrix" => ("\u{2016}", "\u{2016}"),
            "cases" => ("{", ""),
            _ => return laid,
        };
        let pad = em * 0.15;
        let mut padded = Laid::sized(laid.width + pad * 2.0, laid.height, laid.depth);
        padded.put(laid, pad, 0.0);
        self.wrap(padded, left, right, style)
    }
}
