//! The order inside the ranks: sweeps down and up that put each node at the mean of what
//! it is linked to in the rank before, then swaps of neighbours that take crossings away;
//! the order with the fewest crossings is kept. The nodes of a subgraph stay side by side:
//! they are sorted as one, by the mean of theirs.

use std::collections::HashMap;

use super::rank::Layers;

/// What is sorted: a node, or the nodes of a subgraph.
struct Unit {
    key: f32,
    items: Vec<usize>,
}

impl Layers {
    pub fn order(&mut self) {
        let (mut best, mut fewest) = (self.rows.clone(), self.crossings());
        let mut means = vec![0.0f32; self.n.len()];
        for sweep in 0..8 {
            if fewest == 0 {
                break;
            }
            let down = sweep % 2 == 0;
            for k in 1..self.rows.len() {
                self.sort_row(if down { k } else { self.rows.len() - 1 - k }, down, &mut means);
            }
            self.transpose();
            let crossings = self.crossings();
            if crossings < fewest {
                (fewest, best) = (crossings, self.rows.clone());
            }
        }
        self.rows = best;
        for row in &self.rows {
            for (at, v) in row.iter().enumerate() {
                self.n[*v].order = at;
            }
        }
    }

    /// Sorts a rank by the mean place of what each node is linked to in the rank above
    /// (going down) or below; a node linked to nothing there keeps its place.
    fn sort_row(&mut self, r: usize, down: bool, means: &mut [f32]) {
        for v in &self.rows[r] {
            let node = &self.n[*v];
            let near = if down { &node.up } else { &node.down };
            let sum: usize = near.iter().map(|w| self.n[*w].order).sum();
            means[*v] = if near.is_empty() { node.order as f32 } else { sum as f32 / near.len() as f32 };
        }
        self.rows[r] = self.group_sort(&self.rows[r], means, 0);
        for (at, v) in self.rows[r].iter().enumerate() {
            self.n[*v].order = at;
        }
    }

    /// Sorts by the means, the nodes of one subgraph at this depth kept together. It goes as
    /// deep as subgraphs are inside each other, of which a chart has a hundred at most.
    fn group_sort(&self, items: &[usize], means: &[f32], depth: usize) -> Vec<usize> {
        let mut units: Vec<Unit> = Vec::new();
        let mut of: HashMap<usize, usize> = HashMap::new();
        for v in items {
            match self.n[*v].path.get(depth) {
                None => units.push(Unit { key: means[*v], items: vec![*v] }),
                Some(sub) => {
                    let unit = *of.entry(*sub).or_insert(units.len());
                    if unit == units.len() {
                        units.push(Unit { key: 0.0, items: Vec::new() });
                    }
                    units[unit].items.push(*v);
                }
            }
        }
        for unit in units.iter_mut().filter(|unit| self.n[unit.items[0]].path.len() > depth) {
            let sum: f32 = unit.items.iter().map(|v| means[*v]).sum();
            unit.key = sum / unit.items.len() as f32;
            unit.items = self.group_sort(&unit.items, means, depth + 1);
        }
        units.sort_by(|a, b| a.key.total_cmp(&b.key).then(a.items[0].cmp(&b.items[0])));
        units.into_iter().flat_map(|unit| unit.items).collect()
    }

    /// How many times links cross, between every two ranks next to each other.
    // ponytail: every two links between two ranks are compared; a chart has 500 links at
    // most. A merge count when a chart may have more.
    fn crossings(&self) -> usize {
        let mut count = 0;
        for row in &self.rows {
            let ends = |v: &usize| {
                let from = self.n[*v].order;
                self.n[*v].down.iter().map(move |w| (from, self.n[*w].order))
            };
            let links: Vec<(usize, usize)> = row.iter().flat_map(ends).collect();
            for (i, a) in links.iter().enumerate() {
                count += links[i + 1..]
                    .iter()
                    .filter(|b| (a.0 < b.0 && a.1 > b.1) || (a.0 > b.0 && a.1 < b.1))
                    .count();
            }
        }
        count
    }

    /// How many times the links of `a` cross those of `b` when `a` is before `b`.
    fn pair_cross(&self, a: usize, b: usize) -> usize {
        let count = |of_a: &[usize], of_b: &[usize]| -> usize {
            of_a.iter().map(|x| of_b.iter().filter(|y| self.n[*x].order > self.n[**y].order).count()).sum()
        };
        count(&self.n[a].up, &self.n[b].up) + count(&self.n[a].down, &self.n[b].down)
    }

    /// Swaps neighbours in the same subgraphs while that takes crossings away.
    fn transpose(&mut self) {
        for _ in 0..4 {
            let mut better = false;
            for r in 0..self.rows.len() {
                for i in 1..self.rows[r].len() {
                    let (a, b) = (self.rows[r][i - 1], self.rows[r][i]);
                    if self.n[a].path == self.n[b].path && self.pair_cross(b, a) < self.pair_cross(a, b) {
                        self.rows[r].swap(i - 1, i);
                        (self.n[a].order, self.n[b].order) = (i, i - 1);
                        better = true;
                    }
                }
            }
            if !better {
                return;
            }
        }
    }
}
