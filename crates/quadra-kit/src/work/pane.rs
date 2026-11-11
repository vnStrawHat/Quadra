//! The pane tree of a workbench (specs/behaviour/16, "Pane tree"): splits in two, nested,
//! whose leaves are groups of tabs. It is plain data: an app keeps it in a signal and saves
//! it with its settings. This file is the tree and where its parts stand in a box; the view
//! is in `panes.rs`.

use quadra_prim::Rect;

/// The width of the strip around the seam of a split, which takes the pointer; the line in
/// its middle is one pixel.
pub(crate) const GRIP: f32 = 6.0;

/// How the two sides of a split lie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// Side by side.
    Row,
    /// One over the other.
    Column,
}

/// A pane tree: a group of tabs, or two trees with a seam between them.
#[derive(Clone, Debug, PartialEq)]
pub enum Pane {
    /// `collapsed`: only its header shows (in a split of one over the other).
    Leaf { group: String, collapsed: bool },
    /// `ratio` is the share of the first side, from 0 to 1.
    Split { axis: Axis, ratio: f32, first: Box<Pane>, second: Box<Pane> },
}

/// A part of a pane tree in the box of the tree: a leaf, or the seam of a split.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Part {
    Leaf {
        group: String,
        rect: Rect,
    },
    /// `path` says which split (the sides taken from the root, 0 or 1), `whole` is the box of
    /// the split, and a seam that is `fixed` is the line beside a collapsed leaf: not dragged.
    Seam {
        path: Vec<usize>,
        axis: Axis,
        rect: Rect,
        whole: Rect,
        fixed: bool,
    },
}

impl Pane {
    pub fn leaf(group: impl Into<String>) -> Pane {
        Pane::Leaf { group: group.into(), collapsed: false }
    }

    /// Two trees side by side; `ratio` is the share of the first.
    pub fn row(first: Pane, second: Pane, ratio: f32) -> Pane {
        Pane::Split { axis: Axis::Row, ratio, first: Box::new(first), second: Box::new(second) }
    }

    /// One tree over another; `ratio` is the share of the first.
    pub fn column(first: Pane, second: Pane, ratio: f32) -> Pane {
        Pane::Split { axis: Axis::Column, ratio, first: Box::new(first), second: Box::new(second) }
    }

    fn is_group(&self, name: &str) -> bool {
        matches!(self, Pane::Leaf { group, .. } if group == name)
    }

    /// The groups of the leaves, in reading order.
    pub fn groups(&self) -> Vec<&str> {
        match self {
            Pane::Leaf { group, .. } => vec![group],
            Pane::Split { first, second, .. } => [first.groups(), second.groups()].concat(),
        }
    }

    /// Puts a new group beside `group` along `axis`, after it or before it, in half of its
    /// room. Nothing happens when no leaf has that group.
    pub fn split(&mut self, group: &str, axis: Axis, new_group: &str, before: bool) {
        match self {
            Pane::Leaf { .. } if self.is_group(group) => {
                let (old, new) = (Box::new(self.clone()), Box::new(Pane::leaf(new_group)));
                let (first, second) = if before { (new, old) } else { (old, new) };
                *self = Pane::Split { axis, ratio: 0.5, first, second };
            }
            Pane::Leaf { .. } => {}
            Pane::Split { first, second, .. } => {
                first.split(group, axis, new_group, before);
                second.split(group, axis, new_group, before);
            }
        }
    }

    /// Takes the leaf of a group out: the other side of its split takes the place of the
    /// split. The last leaf stays.
    pub fn remove(&mut self, group: &str) {
        let Pane::Split { first, second, .. } = self else { return };
        if first.is_group(group) {
            *self = (**second).clone();
        } else if second.is_group(group) {
            *self = (**first).clone();
        } else {
            first.remove(group);
            second.remove(group);
        }
    }

    /// Gives the first side of the split at `path` (the sides taken from the root, 0 or 1)
    /// a share of its room: each side keeps a twentieth at least.
    pub fn set_ratio(&mut self, path: &[usize], to: f32) {
        let Pane::Split { ratio, first, second, .. } = self else { return };
        match path.split_first() {
            None => *ratio = to.clamp(0.05, 0.95),
            Some((0, rest)) => first.set_ratio(rest, to),
            Some((_, rest)) => second.set_ratio(rest, to),
        }
    }

    /// The share of the first side of the split at `path`, if there is a split there.
    pub fn ratio(&self, path: &[usize]) -> Option<f32> {
        let Pane::Split { ratio, first, second, .. } = self else { return None };
        match path.split_first() {
            None => Some(*ratio),
            Some((0, rest)) => first.ratio(rest),
            Some((_, rest)) => second.ratio(rest),
        }
    }

    /// Folds the leaf of a group to its header, or opens it again. Leaves side by side never
    /// fold.
    pub fn toggle(&mut self, group: &str) {
        let mine = self.is_group(group);
        match self {
            Pane::Leaf { collapsed, .. } if mine => *collapsed = !*collapsed,
            Pane::Leaf { .. } => {}
            Pane::Split { axis, first, second, .. } => {
                if *axis == Axis::Row && (first.is_group(group) || second.is_group(group)) {
                    return;
                }
                first.toggle(group);
                second.toggle(group);
            }
        }
    }

    /// Whether the leaf of a group shows only its header.
    pub fn is_collapsed(&self, group: &str) -> bool {
        match self {
            Pane::Leaf { collapsed, .. } => *collapsed && self.is_group(group),
            Pane::Split { first, second, .. } => first.is_collapsed(group) || second.is_collapsed(group),
        }
    }

    /// Which sides of a split show only their header: leaves of a split of one over the
    /// other, and never both (the second stays open).
    fn folded(axis: Axis, first: &Pane, second: &Pane) -> (bool, bool) {
        let is = |side: &Pane| matches!(side, Pane::Leaf { collapsed: true, .. });
        let stacked = axis == Axis::Column;
        (stacked && is(first), stacked && is(second) && !is(first))
    }

    /// Where the leaves and the seams stand in `rect`, on whole pixels: the leaves in
    /// reading order, then the seams. A collapsed leaf keeps `header` of height and has a
    /// line of one pixel beside it; any other seam is a strip of 6 px.
    pub(crate) fn place(&self, rect: Rect, header: f32) -> Vec<Part> {
        let (mut leaves, mut seams) = (Vec::new(), Vec::new());
        self.place_in(rect, header, &mut Vec::new(), &mut leaves, &mut seams);
        leaves.append(&mut seams);
        leaves
    }

    fn place_in(
        &self,
        rect: Rect,
        header: f32,
        path: &mut Vec<usize>,
        leaves: &mut Vec<Part>,
        seams: &mut Vec<Part>,
    ) {
        let (axis, ratio, first, second) = match self {
            Pane::Leaf { group, .. } => return leaves.push(Part::Leaf { group: group.clone(), rect }),
            Pane::Split { axis, ratio, first, second } => (*axis, *ratio, first, second),
        };
        let row = axis == Axis::Row;
        let length = if row { rect.width } else { rect.height };
        // How much of the length the first side has, and how wide the seam after it is.
        let (before, seam, fixed) = match Pane::folded(axis, first, second) {
            (true, _) => (header.min(length), 1.0, true),
            (_, true) => ((length - header - 1.0).max(0.0), 1.0, true),
            _ => ((ratio * length).floor().min((length - GRIP).max(0.0)), GRIP, false),
        };
        let cut = |from: f32, len: f32| match row {
            true => Rect::new(rect.x + from, rect.y, len.max(0.0), rect.height),
            false => Rect::new(rect.x, rect.y + from, rect.width, len.max(0.0)),
        };
        seams.push(Part::Seam { path: path.clone(), axis, rect: cut(before, seam), whole: rect, fixed });
        for (side, pane, part) in
            [(0, first, cut(0.0, before)), (1, second, cut(before + seam, length - before - seam))]
        {
            path.push(side);
            pane.place_in(part, header, path, leaves, seams);
            path.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workbench() -> Pane {
        Pane::row(
            Pane::leaf("explorer"),
            Pane::column(Pane::leaf("editor"), Pane::leaf("terminal"), 0.5),
            0.25,
        )
    }

    #[test]
    fn a_tree_is_split_and_a_leaf_taken_out() {
        let mut tree = workbench();
        assert_eq!(tree.groups(), ["explorer", "editor", "terminal"]);
        tree.split("editor", Axis::Row, "preview", false);
        tree.split("explorer", Axis::Column, "outline", true);
        tree.split("nowhere", Axis::Row, "lost", false);
        assert_eq!(tree.groups(), ["outline", "explorer", "editor", "preview", "terminal"]);
        tree.remove("explorer");
        tree.remove("preview");
        assert_eq!(
            tree,
            Pane::row(
                Pane::leaf("outline"),
                Pane::column(Pane::leaf("editor"), Pane::leaf("terminal"), 0.5),
                0.25
            )
        );
        // The last leaf stays.
        let mut one = Pane::leaf("only");
        one.remove("only");
        assert_eq!(one.groups(), ["only"]);
    }

    #[test]
    fn a_share_is_kept_off_the_ends_and_only_stacked_leaves_fold() {
        let mut tree = workbench();
        tree.set_ratio(&[], 0.99);
        tree.set_ratio(&[1], 0.01);
        assert_eq!((tree.ratio(&[]), tree.ratio(&[1]), tree.ratio(&[0])), (Some(0.95), Some(0.05), None));
        tree.toggle("explorer"); // beside the rest: it never folds
        tree.toggle("terminal");
        assert_eq!((tree.is_collapsed("explorer"), tree.is_collapsed("terminal")), (false, true));
        tree.toggle("terminal");
        assert!(!tree.is_collapsed("terminal"));
    }

    #[test]
    fn the_parts_stand_on_whole_pixels_and_a_folded_leaf_keeps_its_header() {
        let mut tree = workbench();
        let leaf = |group: &str, x, y, w, h| Part::Leaf { group: group.into(), rect: Rect::new(x, y, w, h) };
        let parts = tree.place(Rect::new(0.0, 0.0, 806.0, 406.0), 28.0);
        assert_eq!(
            parts[..3],
            [
                leaf("explorer", 0.0, 0.0, 201.0, 406.0),
                leaf("editor", 207.0, 0.0, 599.0, 203.0),
                leaf("terminal", 207.0, 209.0, 599.0, 197.0),
            ]
        );
        let whole = Rect::new(207.0, 0.0, 599.0, 406.0);
        let seam = Part::Seam {
            path: vec![1],
            axis: Axis::Column,
            rect: Rect::new(207.0, 203.0, 599.0, 6.0),
            whole,
            fixed: false,
        };
        assert_eq!((parts.len(), &parts[4]), (5, &seam));

        // Folded, the terminal is its header under a line, and the editor has the rest.
        tree.toggle("terminal");
        let parts = tree.place(Rect::new(0.0, 0.0, 806.0, 406.0), 28.0);
        assert_eq!(
            parts[1..3],
            [leaf("editor", 207.0, 0.0, 599.0, 377.0), leaf("terminal", 207.0, 378.0, 599.0, 28.0)]
        );
        let line = Part::Seam {
            path: vec![1],
            axis: Axis::Column,
            rect: Rect::new(207.0, 377.0, 599.0, 1.0),
            whole,
            fixed: true,
        };
        assert_eq!(parts[4], line);
    }
}
