//! A value that eases to its target (specs/05, "Frame callbacks"): the width of a panel that
//! slides open, the offset of something that glides. It is advanced by the frames of its
//! window, on the window's clock, and costs nothing once it has arrived.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use quadra_prim::Ease;
use quadra_reactive::{Signal, effect, expect, on_cleanup, signal, untrack};
use quadra_tree::Handle;

/// A change under way: from where, since when.
#[derive(Clone, Copy)]
struct Run {
    from: f32,
    since: Instant,
}

/// A signal that follows `target`: it starts at the target, and each time the target
/// changes it goes there over `over`, along `ease`, from wherever it is. A target that
/// changes on the way is gone to from where the value has got to.
///
/// Call it where the window's tree is in context, as [`crate::each_frame`].
#[track_caller]
pub fn tween(target: impl Fn() -> f32 + 'static, over: Duration, ease: Ease) -> Signal<f32> {
    let handle = expect::<Handle>();
    let value = signal(untrack(&target));
    let (run, alive): (Rc<Cell<Option<Run>>>, Rc<Cell<bool>>) = (Rc::default(), Rc::new(Cell::new(true)));
    let ended = alive.clone();
    on_cleanup(move || ended.set(false));
    let target = Rc::new(target);
    effect(move || {
        let (to, from) = (target(), value.get_untracked());
        if to == from {
            return run.set(None); // there already: a change under way is over
        }
        // The clock is read when the target changes, not at the frame after: a window
        // that was asleep has no recent frame to count from.
        let waiting = run.replace(Some(Run { from, since: handle.now() })).is_some();
        if waiting {
            return; // the callback that is running goes on, to the new target
        }
        let (run, alive, target, clock) = (run.clone(), alive.clone(), target.clone(), handle.clone());
        handle.each_frame(move || {
            let Some(Run { from, since }) = run.get().filter(|_| alive.get()) else { return false };
            let to = untrack(|| target());
            let part = (clock.now() - since).as_secs_f32() / over.as_secs_f32().max(f32::MIN_POSITIVE);
            value.set(from + (to - from) * ease.at(part));
            if part >= 1.0 {
                run.set(None);
            }
            part < 1.0
        });
    });
    value
}
