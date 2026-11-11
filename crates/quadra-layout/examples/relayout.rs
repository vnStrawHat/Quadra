//! Micro-bench of incremental layout (specs/14, Tooling): what one change costs in a list of
//! 10,000 rows, against laying the list out from scratch.
//!
//!     tools/cargo.sh run --release -p quadra-layout --example relayout

use std::time::Instant;

use quadra_layout::{Direction, Known, LayoutId, LayoutTree, Length, Measure, Overflow, RootSize, Style};
use quadra_prim::Size;

const ROWS: usize = 10_000;

/// Text of 8 px characters in 16 px lines; the character count is stored by node index.
struct Text(Vec<usize>);

impl Measure for Text {
    fn measure(&mut self, node: LayoutId, known: Known, avail: Size) -> Size {
        let chars = self.0[index(node)];
        let limit = known.width.unwrap_or(avail.width);
        let per_line = if limit.is_finite() { ((limit / 8.0) as usize).max(1) } else { chars.max(1) };
        Size::new(chars.min(per_line) as f32 * 8.0, chars.div_ceil(per_line).max(1) as f32 * 16.0)
    }
}

/// Node ids are handed out in order from zero in a fresh tree: recover the index for the bench.
fn index(id: LayoutId) -> usize {
    format!("{id:?}").trim_matches(|c: char| !c.is_ascii_digit()).parse().unwrap()
}

fn list(fixed_rows: bool) -> (LayoutTree, Text, LayoutId, Vec<LayoutId>) {
    let (mut t, mut text) = (LayoutTree::new(), Text(Vec::new()));
    let mut leaves = Vec::new();
    let mut rows = Vec::new();
    for _ in 0..ROWS {
        let leaf = t.insert_leaf(Style::default());
        text.0.resize(index(leaf) + 3, 20);
        let height = if fixed_rows { Length::Px(28.0) } else { Length::Auto };
        let row = t.insert(Style { height, direction: Direction::Column, ..Style::default() });
        t.set_children(row, &[leaf]);
        leaves.push(leaf);
        rows.push(row);
    }
    let root =
        t.insert(Style { direction: Direction::Column, overflow_y: Overflow::Scroll, ..Style::default() });
    t.set_children(root, &rows);
    (t, text, root, leaves)
}

fn main() {
    let size = RootSize::Fill(Size::new(400.0, 600.0));
    for fixed_rows in [true, false] {
        let (mut t, mut text, root, leaves) = list(fixed_rows);
        let start = Instant::now();
        t.layout(root, size, &mut text);
        let full = start.elapsed().as_secs_f64() * 1e6;
        let kind = if fixed_rows { "rows of fixed height" } else { "rows sized by their text" };
        println!("{ROWS} {kind}: {full:.0} us from scratch");

        // One row's text changes and takes more lines.
        const CHANGES: usize = 2_000;
        t.take_stats();
        let start = Instant::now();
        for n in 0..CHANGES {
            let leaf = leaves[(n * 37) % ROWS];
            text.0[index(leaf)] = 60 + n % 100;
            t.mark_dirty(leaf);
            t.layout(root, size, &mut text);
        }
        let us = start.elapsed().as_secs_f64() * 1e6 / CHANGES as f64;
        let stats = t.take_stats();
        println!(
            "  one row's text changes: {us:.2} us, {:.1} nodes asked, {:.1} ancestors laid out again",
            stats.asked as f64 / CHANGES as f64,
            stats.climbed as f64 / CHANGES as f64
        );
    }
}
