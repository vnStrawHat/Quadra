//! Comparing the texts of a diff: on the window's thread when they are small, on the pool
//! when they are long, so that a frame is not held (13 ms for 100 000 lines).

use std::cell::Cell;
use std::rc::Rc;

use quadra_async::{Task, background, spawn};
use quadra_reactive::{Signal, effect, signal};

use super::{Lines, Sides, diff_lines};

/// From how many lines, the two texts together, the comparison is made on the pool.
const FAR: usize = 20_000;

impl Sides {
    fn of(old: String, new: String) -> Sides {
        let (old, new) = (Lines::new(old), Lines::new(new));
        let script = diff_lines(&old.all(), &new.all());
        Sides { old, new, script }
    }
}

/// The texts and their script, compared again when either changes. A long comparison runs
/// on the pool; until it comes back the last one shows, and a newer text cancels it.
pub(super) fn compare(old: Rc<dyn Fn() -> String>, new: Rc<dyn Fn() -> String>) -> Signal<Rc<Sides>> {
    let empty = || Rc::new(Sides::of(String::new(), String::new()));
    let sides = signal(empty());
    let running: Rc<Cell<Option<Task<()>>>> = Rc::default();
    effect(move || {
        let (old, new) = (old(), new());
        let lines = |text: &str| text.bytes().filter(|&b| b == b'\n').count();
        if lines(&old) + lines(&new) < FAR {
            running.set(None);
            sides.set(Rc::new(Sides::of(old, new)));
            return;
        }
        running.set(Some(spawn(async move {
            if let Ok(compared) = background(move || Sides::of(old, new)).await {
                sides.set(Rc::new(compared));
            }
        })));
    });
    sides
}
