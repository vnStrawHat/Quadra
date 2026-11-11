//! What the runtime could not handle, and where it goes (specs/01, R18).

use std::fmt;
use std::rc::Rc;

use crate::runtime::{Loc, rt};

pub(crate) type Hook = Rc<dyn Fn(&ReactiveError)>;

/// What the runtime could not handle. It goes to the error hook.
#[derive(Clone, Debug)]
pub enum ReactiveError {
    /// An effect kept waking itself and was stopped for the rest of the batch.
    EffectCycle {
        /// Where the effect was created.
        effect: Loc,
        /// Where the write that last woke it was made.
        woken_by: Option<Loc>,
    },
}

impl fmt::Display for ReactiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ReactiveError::EffectCycle { effect, woken_by } = self;
        match woken_by {
            Some(write) => {
                write!(f, "the effect created at {effect} keeps being woken by the write at {write}; stopped")
            }
            None => write!(f, "the effect created at {effect} keeps waking itself; stopped"),
        }
    }
}

impl std::error::Error for ReactiveError {}

/// Sets where errors go. Without a hook they are printed to standard error.
pub fn set_error_hook(hook: impl Fn(&ReactiveError) + 'static) {
    let hook: Hook = Rc::new(hook);
    drop(rt(|r| r.hook.replace(hook)));
}

pub(crate) fn report(error: ReactiveError) {
    match rt(|r| r.hook.clone()) {
        Some(hook) => hook(&error),
        None => eprintln!("quadra: {error}"),
    }
}
