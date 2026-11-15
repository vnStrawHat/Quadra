//! How tall the rows of a list are, and where each one is: one height for all, or a height a
//! row, measured or estimated, with running sums that take a logarithmic time to change and
//! to ask (a binary indexed tree): a million rows cost twenty steps.

/// A row that was not measured yet.
const UNKNOWN: f32 = -1.0;

pub(crate) enum Heights {
    /// Every row has this height: arithmetic, and no memory.
    Uniform { height: f32, count: usize },
    /// `own[i]` is row `i`'s measured height, or `UNKNOWN`: it then counts with `estimate`.
    /// `sums[k]` holds the heights of the `k & -k` rows that end at row `k - 1`.
    Measured { estimate: f32, own: Vec<f32>, sums: Vec<f64> },
}

impl Heights {
    pub fn measured(estimate: f32) -> Heights {
        Heights::Measured { estimate, own: Vec::new(), sums: vec![0.0] }
    }

    /// The height every row has, in a list that does not measure them.
    pub fn fixed(&self) -> Option<f32> {
        match self {
            Heights::Uniform { height, .. } => Some(*height),
            Heights::Measured { .. } => None,
        }
    }

    /// Gives every row of a list that does not measure them another height.
    pub fn set_fixed(&mut self, to: f32) {
        if let Heights::Uniform { height, .. } = self {
            *height = to;
        }
    }

    /// Where a row starts and ends, and whether that is measured and not estimated.
    pub fn span(&self, row: usize) -> (f64, f64, bool) {
        let measured = match self {
            Heights::Uniform { .. } => true,
            Heights::Measured { own, .. } => own.get(row).is_some_and(|&height| height >= 0.0),
        };
        (self.before(row), self.before(row + 1), measured)
    }

    /// The row that holds `y`: as many rows end at or before it.
    pub fn row_at(&self, y: f64) -> usize {
        self.ending(y, true)
    }

    /// How many rows start before `y`.
    pub fn rows_before(&self, y: f64) -> usize {
        if y > 0.0 { self.ending(y, false) + 1 } else { 0 }
    }

    /// Makes the list `count` rows long. Rows that stay keep what was measured of them, and
    /// their sums: a sum covers rows that end at its own, so fewer rows cut the sums, and a
    /// row more adds one, its height and those of the rows its sum covers besides (the log
    /// of the count of rows).
    pub fn resize(&mut self, count: usize) {
        match self {
            Heights::Uniform { count: n, .. } => *n = count,
            Heights::Measured { estimate, own, sums } => {
                own.resize(count, UNKNOWN);
                sums.truncate(count + 1);
                for k in sums.len()..=count {
                    let height = if own[k - 1] < 0.0 { *estimate } else { own[k - 1] } as f64;
                    let covered = prefix(sums, k - 1) - prefix(sums, k - (k & k.wrapping_neg()));
                    sums.push(height + covered);
                }
            }
        }
    }

    /// Records the height a row was measured with, and returns how much taller that made it.
    pub fn set(&mut self, row: usize, height: f32) -> f32 {
        let Heights::Measured { estimate, own, sums } = self else { return 0.0 };
        let Some(known) = own.get_mut(row) else { return 0.0 };
        let delta = height - if *known < 0.0 { *estimate } else { *known };
        *known = height;
        let mut k = row + 1;
        while delta != 0.0 && k < sums.len() {
            sums[k] += delta as f64;
            k += k & k.wrapping_neg();
        }
        delta
    }

    /// Where a row starts: the heights of the rows before it. `count()` gives the list's end.
    pub fn before(&self, row: usize) -> f64 {
        match self {
            Heights::Uniform { height, count } => row.min(*count) as f64 * *height as f64,
            Heights::Measured { sums, .. } => prefix(sums, row.min(sums.len() - 1)),
        }
    }

    /// How many rows end before `y`, or at it too when `at` is set.
    fn ending(&self, y: f64, at: bool) -> usize {
        let ends = |sum: f64| if at { sum <= y } else { sum < y };
        match self {
            Heights::Uniform { height, count } => {
                let rows = (y / *height as f64).floor().max(0.0) as usize;
                let rows = if rows > 0 && !ends(rows as f64 * *height as f64) { rows - 1 } else { rows };
                rows.min(*count)
            }
            Heights::Measured { sums, .. } => {
                let (count, mut rows, mut left) = (sums.len() - 1, 0, y);
                let mut step = count.checked_ilog2().map_or(0, |bits| 1usize << bits);
                while step > 0 {
                    // `left` is what remains of `y` after the rows counted so far.
                    let next = rows + step;
                    if next <= count && (if at { sums[next] <= left } else { sums[next] < left }) {
                        rows = next;
                        left -= sums[next];
                    }
                    step >>= 1;
                }
                rows
            }
        }
    }
}

/// The heights of the first `k` rows, from the sums.
fn prefix(sums: &[f64], mut k: usize) -> f64 {
    let mut sum = 0.0;
    while k > 0 {
        sum += sums[k];
        k &= k - 1;
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measured_heights_answer_as_a_plain_sum_does() {
        let mut heights = Heights::measured(20.0);
        let mut plain: Vec<f32> = Vec::new();
        let mut seed = 7u64;
        let mut below = |n: usize| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) as usize % n
        };
        // Longer, shorter, longer again: what stays keeps its sums.
        for round in 0..280 {
            if round % 40 == 0 {
                let count = [0, 1, 64, 37, 100, 3, 80][round / 40];
                heights.resize(count);
                plain.resize(count, 20.0);
            }
            if !plain.is_empty() {
                let (row, height) = (below(plain.len()), 5.0 + below(40) as f32);
                assert_eq!(heights.set(row, height), height - plain[row]);
                plain[row] = height;
            }
            let row = below(plain.len() + 2);
            let before: f32 = plain.iter().take(row).sum();
            assert_eq!(heights.before(row), before as f64, "round {round}");
            let y = below(before as usize + 50) as f64;
            for at in [true, false] {
                let ends = |sum: f32| if at { sum as f64 <= y } else { (sum as f64) < y };
                let want = (1..=plain.len()).take_while(|&n| ends(plain[..n].iter().sum())).count();
                assert_eq!(heights.ending(y, at), want, "round {round}, y {y}, at {at}");
            }
        }
    }

    #[test]
    fn uniform_heights_are_arithmetic() {
        let mut heights = Heights::Uniform { height: 20.0, count: 0 };
        heights.resize(100);
        assert_eq!((heights.before(3), heights.before(500), heights.set(3, 50.0)), (60.0, 2000.0, 0.0));
        // The row that holds 600 is row 30; rows start before 600 up to row 29.
        assert_eq!((heights.ending(600.0, true), heights.ending(600.0, false)), (30, 29));
        assert_eq!((heights.ending(610.0, true), heights.ending(610.0, false)), (30, 30));
        assert_eq!((heights.ending(0.0, false), heights.ending(1e9, true)), (0, 100));
        assert_eq!(
            (heights.row_at(605.0), heights.rows_before(600.0), heights.rows_before(0.0)),
            (30, 30, 0)
        );
        assert_eq!(heights.fixed(), Some(20.0));
    }
}
