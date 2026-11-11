//! Times frames and says on standard output how the run goes: "start" after the warm-up
//! frames, then "done {...}" with the frame intervals, as the runner expects.
//!
//! With `BENCH_PHASES=1` it also says, before "done", what a measured frame did on average
//! and how long each part took.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;
use std::time::{Duration, Instant};

use quadra::FrameReport;

/// The reports of the measured frames, added up.
#[derive(Default)]
pub struct Phases {
    measuring: bool,
    frames: u32,
    times: [Duration; 6],
    counts: [u64; 6],
}

pub type SharedPhases = Rc<RefCell<Phases>>;

impl Phases {
    pub fn add(&mut self, r: &FrameReport) {
        if !self.measuring {
            return;
        }
        self.frames += 1;
        let p = r.passes;
        for (sum, part) in
            self.times.iter_mut().zip([p.update, p.layout, p.paint, p.compose, r.draw, r.present])
        {
            *sum += part;
        }
        let counts = [
            r.stats.bindings_run,
            r.stats.nodes_laid_out,
            r.stats.nodes_painted,
            r.stats.instances_written,
            r.render.instances_uploaded,
            r.render.draw_calls,
        ];
        for (sum, count) in self.counts.iter_mut().zip(counts) {
            *sum += count as u64;
        }
    }

    fn report(&self) -> String {
        let n = self.frames.max(1);
        let us = |i: usize| self.times[i].as_secs_f64() * 1e6 / n as f64;
        let per = |i: usize| self.counts[i] / n as u64;
        format!(
            "phases us: update {:.0} layout {:.0} paint {:.0} compose {:.0} draw {:.0} present {:.0}; \
             a frame: {} bindings, {} laid out, {} painted, {} instances written, {} uploaded, {} draws",
            us(0),
            us(1),
            us(2),
            us(3),
            us(4),
            us(5),
            per(0),
            per(1),
            per(2),
            per(3),
            per(4),
            per(5)
        )
    }
}

pub struct Recorder {
    warmup: usize,
    frames: usize,
    seen: usize,
    last: Instant,
    started: Instant,
    /// Microseconds between the starts of the measured frames.
    intervals: Vec<u64>,
    phases: SharedPhases,
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
    pub fn new(phases: SharedPhases) -> Recorder {
        let (now, frames) = (Instant::now(), env("BENCH_FRAMES", 600));
        Recorder {
            warmup: env("BENCH_WARMUP", 120),
            frames,
            seen: 0,
            last: now,
            started: now,
            intervals: Vec::with_capacity(frames),
            phases,
        }
    }

    /// Counts a frame, and returns whether the run wants another.
    pub fn tick(&mut self) -> bool {
        let now = Instant::now();
        self.seen += 1;
        if self.seen == self.warmup {
            say("start");
            self.started = now;
            self.phases.borrow_mut().measuring = true;
        } else if self.seen > self.warmup && self.intervals.len() < self.frames {
            self.intervals.push((now - self.last).as_micros() as u64);
            if self.intervals.len() == self.frames {
                self.phases.borrow_mut().measuring = false;
                if env("BENCH_PHASES", 0) != 0 {
                    say(&self.phases.borrow().report());
                }
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
