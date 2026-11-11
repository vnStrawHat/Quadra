//! Times frames and says on standard output how the run goes: "start" after the warm-up
//! frames, then "done {...}" with the frame intervals, as the runner expects.

use std::io::Write;
use std::time::Instant;

pub struct Recorder {
    warmup: usize,
    frames: usize,
    seen: usize,
    last: Instant,
    started: Instant,
    /// Microseconds between the starts of the measured frames.
    intervals: Vec<u64>,
}

fn env(name: &str, fallback: usize) -> usize {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(fallback)
}

/// Prints a line the runner reads at once.
pub fn say(line: &str) {
    println!("{line}");
    let _ = std::io::stdout().flush();
}

impl Recorder {
    pub fn new() -> Recorder {
        let (now, frames) = (Instant::now(), env("BENCH_FRAMES", 600));
        Recorder {
            warmup: env("BENCH_WARMUP", 120),
            frames,
            seen: 0,
            last: now,
            started: now,
            intervals: Vec::with_capacity(frames),
        }
    }

    /// Counts a frame, and returns whether the run wants another.
    pub fn tick(&mut self) -> bool {
        let now = Instant::now();
        self.seen += 1;
        if self.seen == self.warmup {
            say("start");
            self.started = now;
        } else if self.seen > self.warmup && self.intervals.len() < self.frames {
            self.intervals.push((now - self.last).as_micros() as u64);
            if self.intervals.len() == self.frames {
                self.intervals.sort_unstable();
                let (n, wall) = (self.frames, (now - self.started).as_micros());
                let at = |i: usize| self.intervals[i.min(n - 1)];
                say(&format!(
                    "done {{\"frames\":{n},\"wallUs\":{wall},\"p50Us\":{},\"p99Us\":{},\"maxUs\":{}}}",
                    at(n / 2),
                    at(n * 99 / 100),
                    at(n - 1)
                ));
            }
        }
        self.last = now;
        self.seen <= self.warmup + self.frames
    }
}
