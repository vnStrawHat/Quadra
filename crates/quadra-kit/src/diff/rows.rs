//! The rows a diff shows (specs/13, "Components"): the script of its lines as one column or
//! as two halves, with the lines far from any change folded. Plain functions.

use super::lines::Step;

/// How a diff is laid out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DiffMode {
    /// One column: a line removed on a row, a line added on the next.
    #[default]
    Inline,
    /// The old text at the left, the new at the right.
    SideBySide,
}

/// A row of a diff; lines by their numbers from 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    /// A line of both texts.
    Same { old: usize, new: usize },
    /// A line removed, on a row of its own; `with` is the added line it is paired with.
    Removed { old: usize, with: Option<usize> },
    /// A line added, on a row of its own; `with` is the removed line it is paired with.
    Added { new: usize, with: Option<usize> },
    /// Side by side: a line removed beside the line added in its place; one may be missing.
    Pair { old: Option<usize>, new: Option<usize> },
    /// Lines of both texts that do not show: how many, from this line of the old text.
    Fold { old: usize, count: usize },
}

/// The rows of a script. With `context`, the lines alike that are further than that from a
/// change are one row, unless the fold is in `open` (by its first line in the old text).
pub fn rows(script: &[Step], mode: DiffMode, context: Option<usize>, open: &[usize]) -> Vec<Row> {
    let same = |step: &Step| matches!(step, Step::Same(..));
    let runs: Vec<&[Step]> = script.chunk_by(|a, b| same(a) == same(b)).collect();
    let mut rows = Vec::with_capacity(script.len());
    for (at, run) in runs.iter().enumerate() {
        if let Step::Same(old, new) = run[0] {
            let line = |i: usize| Row::Same { old: old + i, new: new + i };
            // What is kept is beside a change: at an end of the texts there is none.
            let hidden = context.map_or(0..0, |keep| {
                let from = if at > 0 { keep.min(run.len()) } else { 0 };
                let to = if at + 1 < runs.len() { run.len().saturating_sub(keep) } else { run.len() };
                from..to.max(from)
            });
            if hidden.len() > 1 && !open.contains(&(old + hidden.start)) {
                rows.extend((0..hidden.start).map(line));
                rows.push(Row::Fold { old: old + hidden.start, count: hidden.len() });
                rows.extend((hidden.end..run.len()).map(line));
            } else {
                rows.extend((0..run.len()).map(line));
            }
            continue;
        }
        // ponytail: lines are paired by their order in the run; by how alike they are when
        // runs of several edited lines with lines put in between them read badly.
        let of = |keep: fn(&Step) -> Option<usize>| run.iter().filter_map(keep).collect::<Vec<_>>();
        let removed = of(|step| if let Step::Removed(i) = step { Some(*i) } else { None });
        let added = of(|step| if let Step::Added(j) = step { Some(*j) } else { None });
        let (old, new) = (|i: usize| removed.get(i).copied(), |i: usize| added.get(i).copied());
        match mode {
            DiffMode::Inline => {
                rows.extend(removed.iter().enumerate().map(|(i, &old)| Row::Removed { old, with: new(i) }));
                rows.extend(added.iter().enumerate().map(|(i, &new)| Row::Added { new, with: old(i) }));
            }
            DiffMode::SideBySide => {
                let pairs = 0..removed.len().max(added.len());
                rows.extend(pairs.map(|i| Row::Pair { old: old(i), new: new(i) }));
            }
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::Step::{Added, Removed};
    use super::*;

    /// Ten lines alike, two replaced by one, two alike, one added, twelve alike.
    fn script() -> Vec<Step> {
        let alike = |old: usize, new: usize, n: usize| (0..n).map(move |i| Step::Same(old + i, new + i));
        let script = alike(0, 0, 10).chain([Removed(10), Removed(11), Added(10)]).chain(alike(12, 11, 2));
        script.chain([Added(13)]).chain(alike(14, 14, 12)).collect()
    }

    #[test]
    fn a_run_of_changes_is_rows_one_after_the_other_or_pairs() {
        let inline = rows(&script(), DiffMode::Inline, None, &[]);
        assert_eq!(inline.len(), 28);
        assert_eq!(inline[9], Row::Same { old: 9, new: 9 });
        assert_eq!(inline[10], Row::Removed { old: 10, with: Some(10) });
        assert_eq!(inline[11], Row::Removed { old: 11, with: None });
        assert_eq!(inline[12], Row::Added { new: 10, with: Some(10) });
        assert_eq!(inline[15], Row::Added { new: 13, with: None });

        let sides = rows(&script(), DiffMode::SideBySide, None, &[]);
        assert_eq!(sides.len(), 27);
        assert_eq!(sides[10], Row::Pair { old: Some(10), new: Some(10) });
        assert_eq!(sides[11], Row::Pair { old: Some(11), new: None });
        assert_eq!(sides[14], Row::Pair { old: None, new: Some(13) });
        assert_eq!(sides[15], Row::Same { old: 14, new: 14 });
    }

    #[test]
    fn lines_far_from_a_change_are_one_row_until_it_is_opened() {
        // Three lines kept: seven of the first ten fold, the two between the changes stay,
        // nine of the last twelve fold.
        let folded = rows(&script(), DiffMode::SideBySide, Some(3), &[]);
        assert_eq!(folded.len(), 13);
        assert_eq!(folded[0], Row::Fold { old: 0, count: 7 });
        assert_eq!(folded[1], Row::Same { old: 7, new: 7 });
        assert_eq!(folded[11], Row::Same { old: 16, new: 16 });
        assert_eq!(folded[12], Row::Fold { old: 17, count: 9 });
        let opened = rows(&script(), DiffMode::SideBySide, Some(3), &[17]);
        assert_eq!((opened.len(), opened[0], opened[20]), (21, folded[0], Row::Same { old: 25, new: 25 }));
        // One line more than is kept on both sides is not worth a fold; texts alike are one.
        let near: Vec<Step> = [Removed(0)].into_iter().chain((1..8).map(|i| Step::Same(i, i - 1))).collect();
        let near: Vec<Step> = near.into_iter().chain([Removed(8)]).collect();
        assert_eq!(rows(&near, DiffMode::Inline, Some(3), &[]).len(), 9);
        let alike: Vec<Step> = (0..5).map(|i| Step::Same(i, i)).collect();
        assert_eq!(rows(&alike, DiffMode::Inline, Some(3), &[]), [Row::Fold { old: 0, count: 5 }]);
        assert_eq!(rows(&alike, DiffMode::Inline, None, &[]).len(), 5);
    }
}
