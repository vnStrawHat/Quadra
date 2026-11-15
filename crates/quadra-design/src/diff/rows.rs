//! The rows a diff shows (specs/13, "Components"): the script of its lines as one column or
//! as two halves, with the lines far from any change folded. Plain functions.

use super::lines::Step;

/// The most lines a side of a run of changes may have to be paired by how alike they are:
/// past them, by their order.
const MOST: usize = 64;

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

/// The lines of a run of changes in the order they show side by side: a removed line beside
/// the added one most like it (`alike`), as many such pairs as there can be in order; what is
/// left between them beside each other in order. The flag says the two are alike.
fn pair(
    removed: &[usize],
    added: &[usize],
    alike: &dyn Fn(usize, usize) -> bool,
) -> Vec<(Option<usize>, Option<usize>, bool)> {
    let (n, m) = (removed.len(), added.len());
    let mut matched = Vec::new();
    if n <= MOST && m <= MOST {
        // best[i][j]: the most pairs of what is alike in removed[i..] and added[j..].
        let like: Vec<Vec<bool>> =
            removed.iter().map(|&i| added.iter().map(|&j| alike(i, j)).collect()).collect();
        let mut best = vec![vec![0u16; m + 1]; n + 1];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                let taken = if like[i][j] { best[i + 1][j + 1] + 1 } else { 0 };
                best[i][j] = taken.max(best[i + 1][j]).max(best[i][j + 1]);
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < n && j < m {
            if like[i][j] && best[i][j] == best[i + 1][j + 1] + 1 {
                matched.push((i, j, true));
                (i, j) = (i + 1, j + 1);
            } else if best[i + 1][j] >= best[i][j + 1] {
                i += 1;
            } else {
                j += 1;
            }
        }
    } else {
        matched.extend((0..n.min(m)).map(|k| (k, k, false)));
    }
    let (mut out, mut i, mut j) = (Vec::with_capacity(n.max(m)), 0, 0);
    for (to_i, to_j, like) in matched.into_iter().chain([(n, m, false)]) {
        let left = |k: usize| (i + k < to_i).then(|| removed[i + k]);
        let right = |k: usize| (j + k < to_j).then(|| added[j + k]);
        out.extend((0..(to_i - i).max(to_j - j)).map(|k| (left(k), right(k), false)));
        if to_i < n {
            out.push((Some(removed[to_i]), Some(added[to_j]), like));
        }
        (i, j) = (to_i + 1, to_j + 1);
    }
    out
}

/// The rows of a script. With `context`, the lines alike that are further than that from a
/// change are one row, unless the fold is in `open` (by its first line in the old text).
/// `alike` says whether a removed line and an added one are the same line edited.
pub fn rows(
    script: &[Step],
    mode: DiffMode,
    context: Option<usize>,
    open: &[usize],
    alike: &dyn Fn(usize, usize) -> bool,
) -> Vec<Row> {
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
        let of = |keep: fn(&Step) -> Option<usize>| run.iter().filter_map(keep).collect::<Vec<_>>();
        let removed = of(|step| if let Step::Removed(i) = step { Some(*i) } else { None });
        let added = of(|step| if let Step::Added(j) = step { Some(*j) } else { None });
        let pairs = pair(&removed, &added, alike);
        match mode {
            DiffMode::Inline => {
                let with = |other: Option<usize>, like: bool| other.filter(|_| like);
                let gone = pairs
                    .iter()
                    .filter_map(|&(old, new, like)| Some(Row::Removed { old: old?, with: with(new, like) }));
                rows.extend(gone);
                let came = pairs
                    .iter()
                    .filter_map(|&(old, new, like)| Some(Row::Added { new: new?, with: with(old, like) }));
                rows.extend(came);
            }
            DiffMode::SideBySide => rows.extend(pairs.iter().map(|&(old, new, _)| Row::Pair { old, new })),
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

    /// Every line alike every other: what pairing by order would give.
    fn any(_: usize, _: usize) -> bool {
        true
    }

    #[test]
    fn lines_are_paired_by_how_alike_they_are() {
        // "a = 1" and "run(x);" went, "run(y);" came: it is the second edited.
        let (old, new) = (["a = 1", "run(x);"], ["run(y);"]);
        let alike = |i: usize, j: usize| super::super::words::changed_parts(old[i], new[j]).is_some();
        let script = [Removed(0), Removed(1), Added(0)];
        assert_eq!(
            rows(&script, DiffMode::SideBySide, None, &[], &alike),
            [Row::Pair { old: Some(0), new: None }, Row::Pair { old: Some(1), new: Some(0) }]
        );
        assert_eq!(
            rows(&script, DiffMode::Inline, None, &[], &alike),
            [
                Row::Removed { old: 0, with: None },
                Row::Removed { old: 1, with: Some(0) },
                Row::Added { new: 0, with: Some(1) }
            ]
        );
        // Nothing alike: side by side in order, and no part of a line marked.
        let never = |_: usize, _: usize| false;
        assert_eq!(
            rows(&script, DiffMode::SideBySide, None, &[], &never)[0],
            Row::Pair { old: Some(0), new: Some(0) }
        );
        assert_eq!(rows(&script, DiffMode::Inline, None, &[], &never)[2], Row::Added { new: 0, with: None });
    }

    #[test]
    fn a_run_of_changes_is_rows_one_after_the_other_or_pairs() {
        let inline = rows(&script(), DiffMode::Inline, None, &[], &any);
        assert_eq!(inline.len(), 28);
        assert_eq!(inline[9], Row::Same { old: 9, new: 9 });
        assert_eq!(inline[10], Row::Removed { old: 10, with: Some(10) });
        assert_eq!(inline[11], Row::Removed { old: 11, with: None });
        assert_eq!(inline[12], Row::Added { new: 10, with: Some(10) });
        assert_eq!(inline[15], Row::Added { new: 13, with: None });

        let sides = rows(&script(), DiffMode::SideBySide, None, &[], &any);
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
        let folded = rows(&script(), DiffMode::SideBySide, Some(3), &[], &any);
        assert_eq!(folded.len(), 13);
        assert_eq!(folded[0], Row::Fold { old: 0, count: 7 });
        assert_eq!(folded[1], Row::Same { old: 7, new: 7 });
        assert_eq!(folded[11], Row::Same { old: 16, new: 16 });
        assert_eq!(folded[12], Row::Fold { old: 17, count: 9 });
        let opened = rows(&script(), DiffMode::SideBySide, Some(3), &[17], &any);
        assert_eq!((opened.len(), opened[0], opened[20]), (21, folded[0], Row::Same { old: 25, new: 25 }));
        // One line more than is kept on both sides is not worth a fold; texts alike are one.
        let near: Vec<Step> = [Removed(0)].into_iter().chain((1..8).map(|i| Step::Same(i, i - 1))).collect();
        let near: Vec<Step> = near.into_iter().chain([Removed(8)]).collect();
        assert_eq!(rows(&near, DiffMode::Inline, Some(3), &[], &any).len(), 9);
        let alike: Vec<Step> = (0..5).map(|i| Step::Same(i, i)).collect();
        assert_eq!(rows(&alike, DiffMode::Inline, Some(3), &[], &any), [Row::Fold { old: 0, count: 5 }]);
        assert_eq!(rows(&alike, DiffMode::Inline, None, &[], &any).len(), 5);
    }
}
