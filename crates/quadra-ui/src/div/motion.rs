//! Changes of a box's style that take time (specs/05, "The pose and transitions"): a
//! transition, and a timeline the box plays. The tree runs both.

use std::time::Duration;

use quadra_prim::Ease;
use quadra_tree::{Step, Transition};

use super::{Div, Refine};

/// Steps a box plays one after another: see [`Div::animate`].
pub struct Timeline(quadra_tree::Timeline);

/// A timeline with no steps yet, played once.
pub fn timeline() -> Timeline {
    Timeline(quadra_tree::Timeline { steps: Vec::new(), plays: 1, alternate: false })
}

fn over(ms: u64, ease: Ease) -> Transition {
    Transition { over: Duration::from_millis(ms), ease }
}

impl Timeline {
    /// A step: over `ms`, along `ease`, to what `refine` sets of the background, the
    /// border's colour and the pose, from where the step before left them.
    pub fn to(mut self, ms: u64, ease: Ease, refine: impl FnOnce(Refine) -> Refine) -> Self {
        self.0.steps.push(Step { how: over(ms, ease), to: refine(Refine::default()).0 });
        self
    }

    /// A step that changes nothing.
    pub fn pause(self, ms: u64) -> Self {
        self.to(ms, Ease::Linear, |s| s)
    }

    /// The steps are played `times` times, each from what the box's style says.
    pub fn repeat(mut self, times: u32) -> Self {
        self.0.plays = times.max(1);
        self
    }

    /// Played until the box goes.
    pub fn forever(mut self) -> Self {
        self.0.plays = 0;
        self
    }

    /// Every second play goes backwards.
    pub fn alternate(mut self) -> Self {
        self.0.alternate = true;
        self
    }
}

impl Div {
    /// The changes of the background, the border's colour and the pose take `ms` to show,
    /// fast then slow.
    pub fn transition(self, ms: u64) -> Self {
        self.transition_with(ms, Ease::Out)
    }

    /// The same, along a curve of the caller's.
    pub fn transition_with(self, ms: u64, ease: Ease) -> Self {
        self.on_mount(move |tree, id| tree.set_transition(id, Some(over(ms, ease))))
    }

    /// Plays a timeline from when the box is mounted.
    pub fn animate(self, line: Timeline) -> Self {
        self.on_mount(move |tree, id| tree.set_timeline(id, line.0))
    }
}
