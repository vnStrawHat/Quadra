//! What a seeded test of a parser is made of (specs/12, "Fuzzing"): numbers that are the same
//! on every machine, inputs changed the ways that break parsers, and a run that says which
//! input took too long.

use std::time::{Duration, Instant};

/// A small generator (splitmix64): the same numbers for a seed, everywhere.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    pub fn number(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let z = (self.0 ^ (self.0 >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        let z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A number from 0 up to `n`, not `n` itself; 0 when there is none.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.number() % n as u64) as usize }
    }

    pub fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }

    /// One of `items`, which is not empty.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }

    fn pick_mut<'a, T>(&mut self, items: &'a mut [T]) -> &'a mut T {
        &mut items[self.below(items.len())]
    }
}

/// Whole numbers at the edges of the usual widths, as text and as fields of four bytes.
const EDGE_TEXTS: [&str; 14] = [
    "0",
    "-1",
    "1",
    "65535",
    "65536",
    "2147483647",
    "2147483648",
    "4294967295",
    "4294967296",
    "-2147483648",
    "99999999999999999999",
    "1e308",
    "-0",
    "NaN",
];
const EDGE_FIELDS: [u32; 11] =
    [0, 1, 0x7f, 0x80, 0xff, 0xffff, 0x10000, 0x7fff_ffff, 0x8000_0000, 0xffff_fffe, 0xffff_ffff];
/// Bytes that are not UTF-8: stray continuations, overlong forms, surrogates, cut sequences.
const NOT_UTF8: [&[u8]; 8] = [
    &[0xff],
    &[0x80],
    &[0xc0, 0xaf],
    &[0xe2, 0x82],
    &[0xed, 0xa0, 0x80],
    &[0xf4, 0x90, 0x80, 0x80],
    &[0xf0, 0x9f, 0x98],
    &[0xfe, 0xfe, 0xff],
];

/// `input` changed in one to four ways.
pub fn mutate(rng: &mut Rng, input: &[u8], corpus: &[&[u8]]) -> Vec<u8> {
    let mut bytes = input.to_vec();
    for _ in 0..1 + rng.below(4) {
        change(rng, &mut bytes, corpus);
    }
    bytes
}

fn change(rng: &mut Rng, b: &mut Vec<u8>, corpus: &[&[u8]]) {
    let at = rng.below(b.len() + 1);
    let put = |b: &mut Vec<u8>, bytes: &[u8]| drop(b.splice(at..at, bytes.iter().copied()));
    match rng.below(9) {
        // Bits flipped in a few bytes; a few bytes overwritten.
        0 | 1 if b.is_empty() => {}
        0 => (0..1 + rng.below(4)).for_each(|_| *rng.pick_mut(b) ^= 1 << rng.below(8)),
        1 => (0..1 + rng.below(8)).for_each(|_| *rng.pick_mut(b) = rng.number() as u8),
        2 => b.truncate(at),
        // A chunk again, sometimes hundreds of times: what opens something, nested deep.
        3 => {
            let end = at + 1 + rng.below(16);
            if end <= b.len() {
                let times = if rng.chance(20) { 50 + rng.below(500) } else { 1 + rng.below(4) };
                let chunk = b[at..end].repeat(times);
                put(b, &chunk);
            }
        }
        // The start of this one and the end of another.
        4 if !corpus.is_empty() => {
            let other = *rng.pick(corpus);
            b.truncate(at);
            b.extend_from_slice(&other[rng.below(other.len() + 1)..]);
        }
        5 => put(b, rng.pick(&EDGE_TEXTS).as_bytes()),
        // A length at an edge, over four bytes or between them.
        6 => {
            let field = rng.pick(&EDGE_FIELDS).to_be_bytes();
            if rng.chance(50) && at + 4 <= b.len() {
                b[at..at + 4].copy_from_slice(&field);
            } else {
                put(b, &field);
            }
        }
        7 => put(b, rng.pick(&NOT_UTF8)),
        _ => {
            let end = at + 1 + rng.below(8);
            if end <= b.len() {
                b.drain(at..end);
            }
        }
    }
}

/// How many rounds a test runs: `base`, or what `QUADRA_FUZZ_ITERS` says for a long run.
pub fn iters(base: usize) -> usize {
    let asked = std::env::var("QUADRA_FUZZ_ITERS").ok().and_then(|n| n.parse().ok());
    asked.filter(|n| *n > 0).unwrap_or(base)
}

/// Runs `check` on what `make` gives for each round (the generator and the round's number).
/// An input that takes longer than `limit` fails the test, with its number and its length.
pub fn run(
    name: &str,
    rounds: usize,
    limit: Duration,
    mut make: impl FnMut(&mut Rng, usize) -> Vec<u8>,
    mut check: impl FnMut(&[u8]),
) {
    // The seed is the name's: two tests do not walk the same inputs.
    let seed =
        name.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3));
    let mut rng = Rng::new(seed);
    for round in 0..iters(rounds) {
        let input = make(&mut rng, round);
        let started = Instant::now();
        check(&input);
        let took = started.elapsed();
        assert!(took <= limit, "{name}: input {round} ({} bytes) took {took:?}", input.len());
    }
}
