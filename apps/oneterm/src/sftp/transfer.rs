//! A transfer of the dock (specs/19, "M9 in detail"): how far it is and how it ended.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use quadra::Signal;

/// A file going one way or the other.
pub(crate) struct Transfer {
    pub name: String,
    /// To the server.
    pub up: bool,
    /// The folder it goes to.
    pub to: String,
    pub done: Arc<AtomicU64>,
    pub whole: Arc<AtomicU64>,
    pub cancel: Arc<AtomicBool>,
    /// None while it runs; then how it ended.
    pub end: Signal<Option<Result<(), String>>>,
}

impl Transfer {
    /// `name → remote 48%`, `name ← local`, or why it failed.
    pub fn said(&self) -> String {
        let arrow = if self.up { "\u{2192} remote" } else { "\u{2190} local" };
        match self.end.get() {
            Some(Ok(())) => format!("{} {arrow} done", self.name),
            Some(Err(why)) => format!("{}: {why}", self.name),
            None => {
                let (done, whole) = (self.done.load(Ordering::Relaxed), self.whole.load(Ordering::Relaxed));
                format!("{} {arrow} {}%", self.name, (done * 100).checked_div(whole).unwrap_or(0))
            }
        }
    }

    /// How much is done, from 0 to 1.
    pub fn part(&self) -> f32 {
        let (done, whole) = (self.done.load(Ordering::Relaxed), self.whole.load(Ordering::Relaxed));
        match self.end.get() {
            Some(Ok(())) => 1.0,
            _ if whole == 0 => 0.0,
            _ => done as f32 / whole as f32,
        }
    }

    /// `48%`, `done` or `failed`.
    pub fn state(&self) -> String {
        match self.end.get() {
            Some(Ok(())) => "done".into(),
            Some(Err(_)) => "failed".into(),
            None => format!("{:.0}%", self.part() * 100.0),
        }
    }
}
