//! What differs between two texts (specs/13, "Components"): the shortest script of lines
//! removed and added that turns one into the other, and the part of a line that changed.
//! Plain functions.

use std::ops::Range;

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

/// How many lines may differ between the parts of two texts that are not their common
/// start and end before the script is given up for "all of one, then all of the other".
// ponytail: the search keeps what it found at each distance, which is the square of this in
// memory (9 MB); the linear-space search when texts further apart than this are compared.
const FARTHEST: usize = 1500;

/// The script from `old` to `new`: the fewest lines removed and added (the search of Myers).
/// In a run of changes the lines removed come before those added.
pub fn diff_lines(old: &[&str], new: &[&str]) -> Vec<Step> {
    // What they start and end with alike is not searched.
    let start = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let (a, b) = (&old[start..], &new[start..]);
    let end = a.iter().rev().zip(b.iter().rev()).take_while(|(a, b)| a == b).count();
    let (a, b) = (&a[..a.len() - end], &b[..b.len() - end]);

    let mut script: Vec<Step> = (0..start).map(|i| Step::Same(i, i)).collect();
    let middle = search(a, b)
        .unwrap_or_else(|| (0..a.len()).map(Step::Removed).chain((0..b.len()).map(Step::Added)).collect());
    script.extend(middle.into_iter().map(|step| match step {
        Step::Same(i, j) => Step::Same(i + start, j + start),
        Step::Removed(i) => Step::Removed(i + start),
        Step::Added(j) => Step::Added(j + start),
    }));
    script.extend((0..end).map(|i| Step::Same(start + a.len() + i, start + b.len() + i)));
    // In each run of changes, the lines removed before those added, each kind in its order.
    let same = |step: &Step| matches!(step, Step::Same(..));
    for run in script.chunk_by_mut(|a, b| same(a) == same(b)) {
        run.sort_by_key(|step| matches!(step, Step::Added(_))); // stable
    }
    script
}

/// The shortest script between two lists of lines, or none when they are further apart than
/// [`FARTHEST`]. `reach[k]` is how far along `a` the best path on the diagonal `k` has got.
fn search(a: &[&str], b: &[&str]) -> Option<Vec<Step>> {
    let (n, m) = (a.len() as isize, b.len() as isize);
    let farthest = ((n + m) as usize).min(FARTHEST) as isize;
    let at = |k: isize| (k + farthest + 1) as usize;
    let mut reach = vec![0isize; 2 * farthest as usize + 3];
    // What `reach` was before each distance was tried: the way back.
    let mut before: Vec<Vec<u32>> = Vec::new();
    let mut found = None;
    'search: for d in 0..=farthest {
        before.push(reach[at(-d - 1)..=at(d + 1)].iter().map(|x| *x as u32).collect());
        for k in (-d..=d).step_by(2) {
            let down = k == -d || (k != d && reach[at(k - 1)] < reach[at(k + 1)]);
            let mut x = if down { reach[at(k + 1)] } else { reach[at(k - 1)] + 1 };
            while x < n && x - k < m && a[x as usize] == b[(x - k) as usize] {
                x += 1;
            }
            reach[at(k)] = x;
            if x >= n && x - k >= m {
                found = Some(d);
                break 'search;
            }
        }
    }
    // Back from the end: at each distance, the diagonal it came from, the lines alike along
    // this one, and the one line that was removed or added to get on it.
    let (mut x, mut y, mut script) = (n, m, Vec::new());
    for d in (0..=found?).rev() {
        let was = |k: isize| before[d as usize][(k + d + 1) as usize] as isize;
        let k = x - y;
        let from = if k == -d || (k != d && was(k - 1) < was(k + 1)) { k + 1 } else { k - 1 };
        let (from_x, from_y) = (was(from), was(from) - from);
        while x > from_x && y > from_y {
            (x, y) = (x - 1, y - 1);
            script.push(Step::Same(x as usize, y as usize));
        }
        if d > 0 {
            let step =
                if x == from_x { Step::Added(from_y as usize) } else { Step::Removed(from_x as usize) };
            (x, y) = (from_x, from_y);
            script.push(step);
        }
    }
    script.reverse();
    Some(script)
}

/// The part of each of two lines that is not their common start and end: what changed in a
/// line that was edited. None when less than half of the shorter one is common, spaces not
/// counted: they are two lines, not one that changed.
pub fn changed_part(old: &str, new: &str) -> Option<(Range<usize>, Range<usize>)> {
    let bytes = |alike: &mut dyn Iterator<Item = (char, char)>| -> usize {
        alike.take_while(|(a, b)| a == b).map(|(a, _)| a.len_utf8()).sum()
    };
    let start = bytes(&mut old.chars().zip(new.chars()));
    let end = bytes(&mut old[start..].chars().rev().zip(new[start..].chars().rev()));
    let inked = |text: &str| text.chars().filter(|c| !c.is_whitespace()).count();
    let (common, shorter) =
        (inked(&old[..start]) + inked(&old[old.len() - end..]), inked(old).min(inked(new)));
    (common * 2 >= shorter && shorter > 0 && old != new)
        .then(|| (start..old.len() - end, start..new.len() - end))
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
        let far: Vec<String> = (0..4000).map(|i| i.to_string()).collect();
        let (first, last) = (vec!["first"], vec!["last"]);
        let a = [first.clone(), lines(&far[..2000]), last.clone()].concat();
        let b = [first, lines(&far[2000..]), last].concat();
        let script = diff_lines(&a, &b);
        assert_eq!((script.len(), script[0], script[4001]), (4002, Step::Same(0, 0), Step::Same(2001, 2001)));
        assert!((1..2001).all(|i| script[i] == Step::Removed(i) && script[i + 2000] == Step::Added(i)));
    }

    #[test]
    fn the_part_of_a_line_that_changed_is_between_what_stays() {
        assert_eq!(changed_part("let x = 1;", "let x = 22;"), Some((8..9, 8..10)));
        assert_eq!(changed_part("name: wéb", "name: wöb"), Some((7..9, 7..9)));
        assert_eq!(changed_part("abc", "abcdef"), Some((3..3, 3..6)));
        assert_eq!(changed_part("aaa", "aaaa"), Some((3..3, 3..4)));
        // Nothing much alike but how far in they start, the same line, an empty one: no part.
        assert_eq!(changed_part("fn main() {", "use std::rc::Rc;"), None);
        assert_eq!(changed_part("        env:", "          - port: 9090"), None);
        assert_eq!((changed_part("same", "same"), changed_part("", "x")), (None, None));
    }
}
