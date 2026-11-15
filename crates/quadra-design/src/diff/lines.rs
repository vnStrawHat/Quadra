//! What differs between two texts (specs/13, "Components"): the shortest script of lines
//! removed and added that turns one into the other.
//! Plain functions.

use std::collections::HashMap;

/// A step of the script, with the numbers of its lines from 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// A line of both: its number in the old text, and in the new.
    Same(usize, usize),
    /// A line of the old text that the new one has not.
    Removed(usize),
    /// A line of the new text that the old one had not.
    Added(usize),
}

/// How far the search goes from each end of a part of the texts before it gives the part up
/// for "all of one, then all of the other": parts more than twice this many lines apart.
const FARTHEST: usize = 4096;

/// The script from `old` to `new`: the fewest lines removed and added (the search of Myers,
/// in linear space: the middle of a shortest script is found from both ends, and the parts
/// before and after it are searched alike). In a run of changes the lines removed come
/// before those added.
pub fn diff_lines<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<Step> {
    // Lines as numbers, equal for equal lines: compared at the cost of a number.
    let mut ids: HashMap<&'a str, u32> = HashMap::new();
    let (mut a, mut b) = (Vec::with_capacity(old.len()), Vec::with_capacity(new.len()));
    for (lines, out) in [(old, &mut a), (new, &mut b)] {
        for &line in lines {
            let next = ids.len() as u32;
            out.push(*ids.entry(line).or_insert(next));
        }
    }
    diff_ids(&a, &b)
}

/// The same script between texts whose lines are given as values equal for equal lines
/// (numbers, or hashes of the lines): what a caller that keeps them as a text changes
/// compares.
pub fn diff_ids<T: PartialEq>(a: &[T], b: &[T]) -> Vec<Step> {
    let mut script = Vec::with_capacity(a.len().max(b.len()));
    part(a, b, (0, 0), &mut script);
    // In each run of changes, the lines removed before those added, each kind in its order.
    let same = |step: &Step| matches!(step, Step::Same(..));
    for run in script.chunk_by_mut(|a, b| same(a) == same(b)) {
        run.sort_by_key(|step| matches!(step, Step::Added(_))); // stable
    }
    script
}

/// The script of a part of the texts, `(i, j)` where it starts in each: what it starts and
/// ends with alike is not searched, and what is between is split where a shortest script
/// is half done.
fn part<T: PartialEq>(a: &[T], b: &[T], (i, j): (usize, usize), out: &mut Vec<Step>) {
    let start = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    out.extend((0..start).map(|k| Step::Same(i + k, j + k)));
    let (a, b, i, j) = (&a[start..], &b[start..], i + start, j + start);
    let end = a.iter().rev().zip(b.iter().rev()).take_while(|(x, y)| x == y).count();
    let (a, b) = (&a[..a.len() - end], &b[..b.len() - end]);
    let half = if a.is_empty() || b.is_empty() { None } else { middle(a, b) };
    match half {
        Some((x, y)) => {
            part(&a[..x], &b[..y], (i, j), out);
            part(&a[x..], &b[y..], (i + x, j + y), out);
        }
        None => {
            out.extend((0..a.len()).map(|k| Step::Removed(i + k)));
            out.extend((0..b.len()).map(|k| Step::Added(j + k)));
        }
    }
    out.extend((0..end).map(|k| Step::Same(i + a.len() + k, j + b.len() + k)));
}

/// Where a shortest script between two parts that differ at both ends is half done, found
/// by searching from the start and from the end at once until the two meet; none when they
/// are further apart than [`FARTHEST`] from each end. `reach[0][k]` is how far along `a`
/// the best path from the start on the diagonal `k` has got, `reach[1][k]` the same from
/// the end (in the coordinates of the texts turned round), -1 where none has been.
fn middle<T: PartialEq>(a: &[T], b: &[T]) -> Option<(usize, usize)> {
    let (n, m) = (a.len() as isize, b.len() as isize);
    let most = ((n + m + 1) / 2).min(FARTHEST as isize);
    let width = 2 * most + 2;
    let mut reach = [vec![-1isize; width as usize], vec![-1isize; width as usize]];
    (reach[0][most as usize + 1], reach[1][most as usize + 1]) = (0, 0);
    let (delta, at) = (n - m, |k: isize| (most + k) as usize);
    let odd = delta % 2 != 0;
    // How many diagonals ran off the grid at the low and the high side, from each end.
    let mut off = [[0isize; 2]; 2];
    for d in 0..most {
        for end in [0, 1] {
            let mut k = -d + off[end][0];
            while k <= d - off[end][1] {
                let line = &reach[end];
                let down = k == -d || (k != d && line[at(k) - 1] < line[at(k) + 1]);
                let mut x = if down { line[at(k) + 1] } else { line[at(k) - 1] + 1 };
                let mut y = x - k;
                let same = |x: isize, y: isize| match end {
                    0 => a[x as usize] == b[y as usize],
                    _ => a[(n - x - 1) as usize] == b[(m - y - 1) as usize],
                };
                while x < n && y < m && same(x, y) {
                    (x, y) = (x + 1, y + 1);
                }
                reach[end][at(k)] = x;
                if x > n {
                    off[end][1] += 2;
                } else if y > m {
                    off[end][0] += 2;
                } else if odd == (end == 0) {
                    // The other search on the same diagonal: met when they overlap.
                    let mirror = delta - k;
                    let place = most + mirror;
                    let other = if (0..width).contains(&place) { reach[1 - end][place as usize] } else { -1 };
                    if other != -1 {
                        let (ahead, behind) = if end == 0 { (x, other) } else { (other, x) };
                        if ahead >= n - behind {
                            // Where the search from the start is, on its diagonal.
                            let diagonal = if end == 0 { k } else { mirror };
                            return Some((ahead as usize, (ahead - diagonal) as usize));
                        }
                    }
                }
                k += 2;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The script of two texts of lines of one word: `=` for a line of both, `-` and `+`.
    fn script(old: &str, new: &str) -> String {
        let (old, new): (Vec<&str>, Vec<&str>) =
            (old.split_whitespace().collect(), new.split_whitespace().collect());
        let word = |step: &Step| match *step {
            Step::Same(i, j) if old[i] == new[j] => format!("={}", old[i]),
            Step::Same(..) => "?".to_string(),
            Step::Removed(i) => format!("-{}", old[i]),
            Step::Added(j) => format!("+{}", new[j]),
        };
        diff_lines(&old, &new).iter().map(word).collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn the_script_is_the_shortest_and_removes_before_it_adds() {
        assert_eq!(script("a b c", "a b c"), "=a =b =c");
        assert_eq!(script("a b c", "a x c"), "=a -b +x =c");
        assert_eq!(script("a b c d", "a c d e"), "=a -b =c =d +e");
        assert_eq!(script("", "a b"), "+a +b");
        assert_eq!(script("a b", ""), "-a -b");
        assert_eq!(script("", ""), "");
        // The example of the paper: five edits, and no more.
        let found = script("a b c a b b a", "c b a b a c");
        assert_eq!(found.matches(['-', '+']).count(), 5, "{found}");
        // Two lines replaced by three: those that go, then those that come.
        assert_eq!(script("k a b z", "k x y w z"), "=k -a -b +x +y +w =z");
    }

    #[test]
    fn a_script_has_every_line_of_both_texts_once_and_in_order() {
        let old: Vec<String> = (0..200).map(|i| format!("line {}", i % 17)).collect();
        let new: Vec<String> = (0..230).map(|i| format!("line {}", (i * 7) % 19)).collect();
        fn lines(text: &[String]) -> Vec<&str> {
            text.iter().map(String::as_str).collect()
        }
        let (old, new) = (lines(&old), lines(&new));
        let script = diff_lines(&old, &new);
        let of = |keep: fn(&Step) -> Option<usize>| script.iter().filter_map(keep).collect::<Vec<_>>();
        let olds = of(|step| match *step {
            Step::Same(i, _) | Step::Removed(i) => Some(i),
            Step::Added(_) => None,
        });
        let news = of(|step| match *step {
            Step::Same(_, j) | Step::Added(j) => Some(j),
            Step::Removed(_) => None,
        });
        assert_eq!((olds, news), ((0..200).collect(), (0..230).collect()));
        assert!(script.iter().all(|step| !matches!(*step, Step::Same(i, j) if old[i] != new[j])));

        // Texts with nothing alike, further apart than the search goes: all of one, then
        // all of the other, between what they start and end with.
        let far: Vec<String> = (0..20_000).map(|i| i.to_string()).collect();
        let (first, last) = (vec!["first"], vec!["last"]);
        let a = [first.clone(), lines(&far[..10_000]), last.clone()].concat();
        let b = [first, lines(&far[10_000..]), last].concat();
        let script = diff_lines(&a, &b);
        let ends = (script.len(), script[0], script[20_001]);
        assert_eq!(ends, (20_002, Step::Same(0, 0), Step::Same(10_001, 10_001)));
        assert!((1..10_001).all(|i| script[i] == Step::Removed(i) && script[i + 10_000] == Step::Added(i)));
    }

    #[test]
    fn the_script_keeps_as_many_lines_as_the_longest_common_part() {
        let mut seed = 7u64;
        let mut next = move |below: u64| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) % below
        };
        const WORDS: [&str; 4] = ["a", "b", "c", "d"];
        for _ in 0..300 {
            let mut text = || (0..next(30)).map(|_| WORDS[next(4) as usize]).collect::<Vec<_>>();
            let (old, new) = (text(), text());
            // The longest common part, by the table of every pair of starts.
            let mut best = vec![vec![0usize; new.len() + 1]; old.len() + 1];
            for i in (0..old.len()).rev() {
                for j in (0..new.len()).rev() {
                    best[i][j] = if old[i] == new[j] {
                        best[i + 1][j + 1] + 1
                    } else {
                        best[i + 1][j].max(best[i][j + 1])
                    };
                }
            }
            let script = diff_lines(&old, &new);
            let same = script.iter().filter(|step| matches!(step, Step::Same(..))).count();
            assert_eq!(same, best[0][0], "{old:?} {new:?}");
            assert_eq!(script.len(), old.len() + new.len() - same);
        }
    }

    #[test]
    fn texts_far_apart_are_searched_in_linear_space() {
        // Every other line of 6000 changed: 6000 lines apart, and each line that stays found.
        let old: Vec<String> = (0..6000).map(|i| format!("line {i}")).collect();
        let new: Vec<String> =
            (0..6000).map(|i| if i % 2 == 0 { format!("line {i}") } else { format!("new {i}") }).collect();
        fn lines(text: &[String]) -> Vec<&str> {
            text.iter().map(String::as_str).collect()
        }
        let script = diff_lines(&lines(&old), &lines(&new));
        let same = script.iter().filter(|step| matches!(step, Step::Same(..))).count();
        assert_eq!((same, script.len()), (3000, 9000));
        assert_eq!(script[..4], [Step::Same(0, 0), Step::Removed(1), Step::Added(1), Step::Same(2, 2)]);
    }
}
