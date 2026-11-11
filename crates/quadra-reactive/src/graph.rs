//! The dependency graph: recording reads, marking on writes, bringing nodes up to date, and the
//! flush that runs effects (specs/03-reactive.md, "The graph", "Batches", "Implementation").

use crate::arena::Id;
use crate::error::{ReactiveError, report};
use crate::runtime::{Inner, Kind, Loc, Notify, State, Tracking, rt};

/// An effect woken more often than this in one flush is a cycle.
const RUN_LIMIT: u32 = 64;

impl Inner {
    /// Records that the running node read `source`.
    pub(crate) fn track(&mut self, source: Id) {
        let Some(observer) = self.observer else {
            return;
        };
        let Inner {
            nodes, tracking, ..
        } = self;
        let Some(node) = nodes.get(observer) else {
            return;
        };
        let old = &node.sources;
        // The same source at the same position as last time: nothing to do.
        if tracking.new.is_empty() && old.get(tracking.index) == Some(&source) {
            tracking.index += 1;
            return;
        }
        if old[..tracking.index].contains(&source) || tracking.new.contains(&source) {
            return;
        }
        let had_edge = old[tracking.index..].contains(&source);
        tracking.new.push(source);
        // The edge exists from now, not from the end of the run: a write later in this run
        // must reach the running node.
        if !had_edge && let Some(node) = nodes.get_mut(source) {
            node.subscribers.push(observer);
        }
    }

    /// Ends a run of `observer`: its sources are what it read, in order.
    fn finish(&mut self, observer: Id, tracking: Tracking) {
        let Some(node) = self.nodes.get_mut(observer) else {
            return;
        };
        if tracking.new.is_empty() && tracking.index == node.sources.len() {
            return;
        }
        let dropped = node.sources.split_off(tracking.index);
        node.sources.extend_from_slice(&tracking.new);
        for source in dropped {
            if !tracking.new.contains(&source) {
                self.unlink(source, observer);
            }
        }
    }

    /// A write to `source`: its readers are dirty, and what reads a memo among them is "to
    /// check". A node already marked is not walked again.
    fn mark(&mut self, source: Id, at: Loc, notifies: &mut Vec<Notify>) {
        let Inner {
            nodes,
            stack,
            effects,
            ..
        } = self;
        let Some(node) = nodes.get(source) else {
            return;
        };
        stack.extend(node.subscribers.iter().map(|&s| (s, State::Dirty)));
        while let Some((id, to)) = stack.pop() {
            let Some(node) = nodes.get_mut(id) else {
                continue;
            };
            if node.state >= to {
                continue;
            }
            let was_clean = node.state == State::Clean;
            node.state = to;
            if !was_clean {
                continue;
            }
            match &node.kind {
                Kind::Memo => stack.extend(node.subscribers.iter().map(|&s| (s, State::Check))),
                Kind::Effect => {
                    node.woken_by = Some(at);
                    effects.push(id);
                }
                Kind::Deferred(notify) => notifies.push(notify.clone()),
                Kind::Signal => {}
            }
        }
    }

    /// A memo's value changed: its readers that were "to check" must run.
    fn raise_subscribers(&mut self, id: Id) {
        let Some(node) = self.nodes.get_mut(id) else {
            return;
        };
        let subscribers = std::mem::take(&mut node.subscribers);
        for &s in &subscribers {
            if let Some(reader) = self.nodes.get_mut(s)
                && reader.state == State::Check
            {
                reader.state = State::Dirty;
            }
        }
        if let Some(node) = self.nodes.get_mut(id) {
            node.subscribers = subscribers;
        }
    }
}

fn state(id: Id) -> Option<State> {
    rt(|r| r.nodes.get(id).map(|n| n.state))
}

/// Brings `id` up to date: asks its sources if it is "to check", runs it if it is dirty.
pub(crate) fn update(id: Id) {
    if state(id) == Some(State::Check) {
        let mut i = 0;
        while let Some(source) = rt(|r| r.nodes.get(id).and_then(|n| n.sources.get(i).copied())) {
            update(source);
            if state(id) != Some(State::Check) {
                break;
            }
            i += 1;
        }
    }
    // Clean before it runs: a write during the run to something it reads marks it again.
    let dirty = rt(|r| {
        r.nodes
            .get_mut(id)
            .map(|n| std::mem::replace(&mut n.state, State::Clean))
    });
    if dirty == Some(State::Dirty) {
        run(id);
    }
}

fn run(id: Id) {
    let Some(compute) = rt(|r| r.nodes.get(id).and_then(|n| n.compute.clone())) else {
        return;
    };
    // Writes made by the run are flushed after it, never inside it.
    let saved = rt(|r| {
        r.batch += 1;
        (r.observer.replace(id), std::mem::take(&mut r.tracking))
    });
    let changed = (compute.borrow_mut())();
    rt(|r| {
        let tracking = std::mem::replace(&mut r.tracking, saved.1);
        r.observer = saved.0;
        r.batch -= 1;
        r.finish(id, tracking);
        if changed {
            r.raise_subscribers(id);
        }
    });
}

/// After a write to `id`: mark, tell deferred effects, flush.
pub(crate) fn changed(id: Id, at: Loc) {
    let mut notifies = Vec::new();
    rt(|r| r.mark(id, at, &mut notifies));
    for notify in notifies {
        notify();
    }
    flush();
}

/// Delivers events and runs effects until nothing is left. Does nothing inside a batch or a
/// flush: the outermost one does the work.
pub(crate) fn flush() {
    let start = rt(|r| {
        let start = r.batch == 0 && !r.flushing;
        if start {
            r.flushing = true;
            r.flush += 1;
        }
        start
    });
    if !start {
        return;
    }
    loop {
        if let Some(deliver) = rt(|r| r.deliveries.pop_front()) {
            deliver();
            continue;
        }
        let mut due = rt(|r| {
            let ids = std::mem::take(&mut r.effects);
            ids.into_iter()
                .filter_map(|id| r.nodes.get(id).map(|n| (n.seq, id)))
                .collect::<Vec<_>>()
        });
        if due.is_empty() {
            break;
        }
        due.sort_unstable_by_key(|&(seq, _)| seq);
        for (_, id) in due {
            if within_limit(id) {
                update(id);
            }
        }
    }
    rt(|r| r.flushing = false);
}

/// Counts a wake of an effect in this flush; past the limit it is stopped and reported.
fn within_limit(id: Id) -> bool {
    let cycle = rt(|r| {
        let flush = r.flush;
        let node = r.nodes.get_mut(id)?;
        if node.flush != flush {
            node.flush = flush;
            node.runs = 0;
        }
        node.runs += 1;
        if node.runs <= RUN_LIMIT {
            return None;
        }
        node.state = State::Clean; // a later write may wake it again
        Some(ReactiveError::EffectCycle {
            effect: node.created,
            woken_by: node.woken_by,
        })
    });
    match cycle {
        Some(error) => {
            report(error);
            false
        }
        None => true,
    }
}

/// Runs `f` as one batch: effects and events wait until it returns. Every entry point of the
/// framework is a batch; this is for code outside them, such as tests.
pub fn batch<R>(f: impl FnOnce() -> R) -> R {
    rt(|r| r.batch += 1);
    let out = f();
    rt(|r| r.batch -= 1);
    flush();
    out
}

/// Runs `f` without recording what it reads.
pub fn untrack<R>(f: impl FnOnce() -> R) -> R {
    let observer = rt(|r| r.observer.take());
    let out = f();
    rt(|r| r.observer = observer);
    out
}
