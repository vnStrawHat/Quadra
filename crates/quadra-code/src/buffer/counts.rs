//! The bytes and the line breaks of the pieces of a buffer, summed in a tree of sums
//! (a Fenwick tree): what is before a piece, and which piece holds an offset or a line
//! break, cost the log of the count of pieces, and so does a piece that changes. A piece
//! that comes or goes counts them all again, which is linear and seldom.

pub(super) struct Counts {
    /// At `i` (from 1), the sums of the pieces `i - (i & -i)` to `i - 1`.
    tree: Vec<(usize, usize)>,
    total: (usize, usize),
}

impl Counts {
    pub fn new(pieces: impl Iterator<Item = (usize, usize)>) -> Counts {
        let mut tree = vec![(0, 0)];
        tree.extend(pieces);
        for i in 1..tree.len() {
            let up = i + (i & i.wrapping_neg());
            if up < tree.len() {
                tree[up] = (tree[up].0 + tree[i].0, tree[up].1 + tree[i].1);
            }
        }
        let mut counts = Counts { tree, total: (0, 0) };
        counts.total = counts.before(counts.tree.len() - 1);
        counts
    }

    /// The bytes and the line breaks of all of them.
    pub fn total(&self) -> (usize, usize) {
        self.total
    }

    /// Piece `k` has `bytes` and `breaks` more (or fewer).
    pub fn add(&mut self, k: usize, (bytes, breaks): (isize, isize)) {
        let (mut i, n) = (k + 1, self.tree.len());
        while i < n {
            let at = &mut self.tree[i];
            *at = (at.0.wrapping_add_signed(bytes), at.1.wrapping_add_signed(breaks));
            i += i & i.wrapping_neg();
        }
        self.total = (self.total.0.wrapping_add_signed(bytes), self.total.1.wrapping_add_signed(breaks));
    }

    /// The bytes and the line breaks of the pieces before `k`.
    pub fn before(&self, k: usize) -> (usize, usize) {
        let (mut i, mut sum) = (k, (0, 0));
        while i > 0 {
            sum = (sum.0 + self.tree[i].0, sum.1 + self.tree[i].1);
            i -= i & i.wrapping_neg();
        }
        sum
    }

    /// How many pieces from the first have, together, no more than `most` of what `of`
    /// takes from a sum (its bytes, or its line breaks).
    pub fn within(&self, most: usize, of: fn((usize, usize)) -> usize) -> usize {
        let (mut at, mut left) = (0, most);
        let mut step = (self.tree.len() - 1).checked_ilog2().map_or(0, |log| 1 << log);
        while step > 0 {
            if at + step < self.tree.len() && of(self.tree[at + step]) <= left {
                at += step;
                left -= of(self.tree[at]);
            }
            step /= 2;
        }
        at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_and_searches_agree_with_counting() {
        let mut pieces: Vec<(usize, usize)> = (0..37).map(|i| (1 + i * 7 % 13, i % 3)).collect();
        let mut counts = Counts::new(pieces.iter().copied());
        counts.add(5, (4, 1));
        pieces[5] = (pieces[5].0 + 4, pieces[5].1 + 1);
        counts.add(36, (-1, 0));
        pieces[36].0 -= 1;
        let before = |k: usize| pieces[..k].iter().fold((0, 0), |s, p| (s.0 + p.0, s.1 + p.1));
        for k in 0..=pieces.len() {
            assert_eq!(counts.before(k), before(k));
        }
        assert_eq!(counts.total(), before(pieces.len()));
        for most in 0..counts.total().0 + 2 {
            let wanted = (0..=pieces.len()).rev().find(|&k| before(k).0 <= most).unwrap();
            assert_eq!(counts.within(most, |s| s.0), wanted, "{most}");
        }
    }
}
