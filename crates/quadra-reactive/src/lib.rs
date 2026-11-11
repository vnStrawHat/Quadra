//! Quadra's reactive runtime (specs/03-reactive.md): all state, and the graph of who depends
//! on what. It knows nothing about UI.
//!
//! ```
//! use quadra_reactive::{effect, memo, signal};
//!
//! let count = signal(1);
//! let double = memo(move || count.get() * 2);
//! effect(move || assert_eq!(double.get(), count.get() * 2));
//! count.set(2); // the effect runs again, once
//! ```
//!
//! There is one runtime per thread and handles do not cross threads. Handles are small `Copy`
//! values, so closures capture them freely.

mod arena;
mod effect;
mod error;
mod event;
mod graph;
mod memo;
mod runtime;
mod scope;
mod signal;

pub use effect::{Deferred, deferred, effect};
pub use error::{ReactiveError, set_error_hook};
pub use event::{Event, event};
pub use graph::{batch, untrack};
pub use memo::{Memo, memo};
pub use runtime::{Stats, stats};
pub use scope::{Scope, expect, on_cleanup, provide, use_context};
pub use signal::{Signal, Trigger, signal, trigger};
