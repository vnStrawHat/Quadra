//! The parts of a line that changed, word by word (specs/13, "Components"). Plain functions.

use std::ops::Range;

use super::lines::{Step, diff_lines};

/// Parts of a line, as byte ranges.
pub type Parts = Vec<Range<usize>>;

/// The words of a line, as byte ranges: a run of letters, digits and `_`, a run of spaces,
/// or one other character.
fn words(line: &str) -> Vec<Range<usize>> {
    let class = |c: char| {
        if c.is_alphanumeric() || c == '_' {
            1
        } else if c.is_whitespace() {
            2
        } else {
            0
        }
    };
    let mut out: Vec<Range<usize>> = Vec::new();
    let mut last = None;
    for (at, c) in line.char_indices() {
        let kind = class(c);
        match out.last_mut() {
            Some(word) if kind != 0 && last == Some(kind) => word.end = at + c.len_utf8(),
            _ => out.push(at..at + c.len_utf8()),
        }
        last = Some(kind);
    }
    out
}

/// The parts of each of two lines that changed, word by word, in as many places as there
/// are: runs of words removed and added, those between which only spaces stay made one, and
/// each cut to what is not common to its start and end. None when less than half of the
/// shorter line is common, spaces not counted: they are two lines, not one that changed.
pub fn changed_parts(old: &str, new: &str) -> Option<(Parts, Parts)> {
    let (a, b) = (words(old), words(new));
    let (wa, wb): (Vec<&str>, Vec<&str>) =
        (a.iter().map(|w| &old[w.clone()]).collect(), b.iter().map(|w| &new[w.clone()]).collect());
    let inked = |text: &str| text.chars().filter(|c| !c.is_whitespace()).count();
    let (mut common, mut parts) = (0, Vec::<(Range<usize>, Range<usize>)>::new());
    // Where the next run starts in each line, and whether only spaces stayed since the last.
    let (mut from, mut spaces) = ((0, 0), false);
    let script = diff_lines(&wa, &wb);
    for run in script.chunk_by(|x, y| matches!(x, Step::Same(..)) == matches!(y, Step::Same(..))) {
        if let Step::Same(i, j) = run[0] {
            let said: Vec<&str> =
                run.iter().map(|step| if let Step::Same(i, _) = step { wa[*i] } else { "" }).collect();
            common += said.iter().map(|w| inked(w)).sum::<usize>();
            spaces = said.iter().all(|w| inked(w) == 0);
            from = (
                a.get(i + run.len()).map_or(old.len(), |w| w.start),
                b.get(j + run.len()).map_or(new.len(), |w| w.start),
            );
            continue;
        }
        let (mut x, mut y) = (from.0..from.0, from.1..from.1);
        for step in run {
            match *step {
                Step::Removed(i) => x = x.start.min(a[i].start)..a[i].end.max(x.end),
                Step::Added(j) => y = y.start.min(b[j].start)..b[j].end.max(y.end),
                Step::Same(..) => {}
            }
        }
        if let (true, Some(last)) = (spaces, parts.last_mut()) {
            (last.0.end, last.1.end) = (x.end, y.end);
        } else {
            parts.push((x, y));
        }
    }
    // Each part cut to what is not common to its start and end.
    let bytes = |alike: &mut dyn Iterator<Item = (char, char)>| -> (usize, usize) {
        alike
            .take_while(|(a, b)| a == b)
            .fold((0, 0), |(n, k), (a, _)| (n + a.len_utf8(), k + !a.is_whitespace() as usize))
    };
    for (x, y) in &mut parts {
        let (s, t) = (&old[x.clone()], &new[y.clone()]);
        let (start, ink) = bytes(&mut s.chars().zip(t.chars()));
        let (end, more) = bytes(&mut s[start..].chars().rev().zip(t[start..].chars().rev()));
        common += ink + more;
        (*x, *y) = (x.start + start..x.end - end, y.start + start..y.end - end);
    }
    let shorter = inked(old).min(inked(new));
    (common * 2 >= shorter && shorter > 0 && old != new).then(|| parts.into_iter().unzip())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_parts_of_a_line_that_changed_are_between_what_stays() {
        let one = |old, new| changed_parts(old, new).map(|(x, y)| (x[0].clone(), y[0].clone()));
        assert_eq!(one("let x = 1;", "let x = 22;"), Some((8..9, 8..10)));
        assert_eq!(one("name: wéb", "name: wöb"), Some((7..9, 7..9)));
        assert_eq!(one("abc", "abcdef"), Some((3..3, 3..6)));
        assert_eq!(one("aaa", "aaaa"), Some((3..3, 3..4)));
        // Words changed in two places are two parts; words apart by spaces alone are one.
        let two = changed_parts("let width = old(a, 1);", "let height = old(a, 2);");
        assert_eq!(two, Some((vec![4..9, 19..20], vec![4..10, 20..21])));
        assert_eq!(
            changed_parts("a = red big box;", "a = blue small box;"),
            Some((vec![4..11; 1], vec![4..14; 1]))
        );
        // Nothing much alike but how far in they start, the same line, an empty one: no part.
        assert_eq!(changed_parts("fn main() {", "use std::rc::Rc;"), None);
        assert_eq!(changed_parts("        env:", "          - port: 9090"), None);
        assert_eq!((changed_parts("same", "same"), changed_parts("", "x")), (None, None));
    }
}
