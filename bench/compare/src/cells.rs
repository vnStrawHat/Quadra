//! What a change costs against the size of the UI (specs/14, "What a change costs"): the
//! window filled with `BENCH_CELLS` numbers (600 unless set), every one on screen, each in a
//! box of its colour. `BENCH_CHANGE` says what a frame changes: `one` number, the one in the
//! middle, as a clock in a dashboard (the default),
//! `all` of them, or `rebuild`: the whole grid mounted again, as a UI without a retained tree
//! and incremental layout redoes it every frame.

use quadra::{
    Align, AnyView, Color, Direction, Div, IntoProp, Justify, Paint, TextView, column, div, dynamic, row,
    signal, text,
};

use crate::{animate, cell_color, cell_value, full};

pub fn cells() -> AnyView {
    let n: usize = std::env::var("BENCH_CELLS").ok().and_then(|v| v.parse().ok()).unwrap_or(600);
    let change = std::env::var("BENCH_CHANGE").unwrap_or_else(|_| "one".into());
    // Boxes twice as wide as tall, as the grid scene's: 600 is 20 columns by 30 rows.
    let rows = ((n as f64 * 1.5).sqrt().round() as usize).max(1);
    let shape = Shape { n, rows, cols: n.div_ceil(rows) };
    if change == "rebuild" {
        let frame = signal(0u32);
        animate(move |k| frame.set(k));
        let grid = move || {
            let k = frame.get();
            shape.grid(|i| shape.cell(cell_color(cell_value(i, k)), text(format!("{:.2}", cell_value(i, k)))))
        };
        return AnyView::new(dynamic(grid).container(full()));
    }
    let values: Vec<_> = (0..n).map(|i| signal(cell_value(i, 0))).collect();
    let all = change == "all";
    let set = values.clone();
    animate(move |k| {
        if all {
            set.iter().enumerate().for_each(|(i, v)| v.set(cell_value(i, k)));
        } else {
            set[n / 2].set(cell_value(n / 2, k));
        }
    });
    let cell = |i: usize| {
        let v = values[i];
        shape.cell(move || cell_color(v.get()), text(move || format!("{:.2}", v.get())))
    };
    AnyView::new(shape.grid(cell))
}

#[derive(Clone, Copy)]
struct Shape {
    n: usize,
    rows: usize,
    cols: usize,
}

impl Shape {
    fn grid(self, cell: impl Fn(usize) -> Div) -> Div {
        let line = |r: usize| {
            row().children((0..self.cols).map(|c| r * self.cols + c).filter(|&i| i < self.n).map(&cell))
        };
        full().direction(Direction::Column).child(column().children((0..self.rows).map(line)))
    }

    fn cell<M>(self, bg: impl IntoProp<Paint, M>, number: TextView) -> Div {
        let (w, h) = (800.0 / self.cols as f32, 600.0 / self.rows as f32);
        let number = number.size((h * 0.55).min(11.0)).color(Color::rgb(0x101010));
        div().size(w, h).align_items(Align::Center).justify(Justify::Center).bg(bg).child(number)
    }
}
