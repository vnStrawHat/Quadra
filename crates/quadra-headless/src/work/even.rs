//! Evening out a run of splits of a pane tree (specs/behaviour/16, "Pane tree"): what makes
//! three panes split off one another side by side take a third each.

use super::pane::{Axis, Pane};

impl Pane {
    /// How many panes the tree lays along `axis`: a split along it adds its sides, anything
    /// else is one.
    fn count_along(&self, axis: Axis) -> usize {
        match self {
            Pane::Split { axis: a, first, second, .. } if *a == axis => {
                first.count_along(axis) + second.count_along(axis)
            }
            _ => 1,
        }
    }

    /// The sides taken from the root to a group's leaf.
    fn path_of(&self, group: &str) -> Option<Vec<usize>> {
        match self {
            Pane::Leaf { group: name, .. } => (name == group).then(Vec::new),
            Pane::Split { first, second, .. } => {
                [first, second].iter().enumerate().find_map(|(side, pane)| {
                    let mut path = pane.path_of(group)?;
                    path.insert(0, side);
                    Some(path)
                })
            }
        }
    }

    fn at_mut(&mut self, path: &[usize]) -> &mut Pane {
        match (self, path.split_first()) {
            (Pane::Split { first, .. }, Some((0, rest))) => first.at_mut(rest),
            (Pane::Split { second, .. }, Some((_, rest))) => second.at_mut(rest),
            (pane, _) => pane,
        }
    }

    /// Evens out the splits around a group's leaf: the split that holds it, and the splits
    /// along the same axis that hold that one, give each side room by how many panes it
    /// lays along the axis. Other splits keep their shares; nothing happens when no leaf has
    /// the group.
    pub fn even(&mut self, group: &str) {
        let Some(path) = self.path_of(group) else { return };
        let mut run = None;
        for depth in (0..path.len()).rev() {
            let Pane::Split { axis, ratio, first, second } = self.at_mut(&path[..depth]) else { return };
            if *run.get_or_insert(*axis) != *axis {
                return;
            }
            let (a, b) = (first.count_along(*axis), second.count_along(*axis));
            *ratio = a as f32 / (a + b) as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_along_one_axis_is_evened_and_the_rest_kept() {
        let mut tree = Pane::leaf("a");
        tree.split("a", Axis::Row, "b", false);
        tree.split("b", Axis::Row, "c", false);
        tree.even("c");
        assert_eq!((tree.ratio(&[]), tree.ratio(&[1])), (Some(1.0 / 3.0), Some(0.5)));
        // One over the other inside the middle pane: only that split is its run.
        tree.split("b", Axis::Column, "d", true);
        tree.set_ratio(&[1, 0], 0.3);
        tree.even("b");
        assert_eq!(
            (tree.ratio(&[]), tree.ratio(&[1]), tree.ratio(&[1, 0])),
            (Some(1.0 / 3.0), Some(0.5), Some(0.5))
        );
        assert_eq!(tree.groups(), ["a", "d", "b", "c"]);
        tree.even("nowhere");
        let mut one = Pane::leaf("only");
        one.even("only");
        assert_eq!(one, Pane::leaf("only"));
    }
}
