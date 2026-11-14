//! Changes of a box's style that take time (specs/05, "The pose and transitions"): a
//! transition, and a timeline the box plays. The tree runs both.

use std::time::Duration;

use quadra_prim::Ease;
use quadra_tree::{Amount, Step, Transition};

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
        self.0.steps.push(Step::To(over(ms, ease), refine(Refine::default()).0));
        self
    }

    /// A step that starts together with the step before, over its own time: the two are
    /// one step, as long as the longer. Of two that set the same thing, this one shows.
    pub fn with(mut self, ms: u64, ease: Ease, refine: impl FnOnce(Refine) -> Refine) -> Self {
        let step = Step::To(over(ms, ease), refine(Refine::default()).0);
        match self.0.steps.pop() {
            Some(Step::Together(mut steps)) => {
                steps.push(step);
                self.0.steps.push(Step::Together(steps));
            }
            Some(before) => self.0.steps.push(Step::Together(vec![before, step])),
            None => self.0.steps.push(step),
        }
        self
    }

    /// Another timeline played as a step, as many times as it says, each from where the
    /// steps before it left (one that plays for ever plays once).
    pub fn then(mut self, inner: Timeline) -> Self {
        self.0.steps.push(Step::Line(Box::new(inner.0)));
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
    pub fn transition(self, ms: impl Into<Amount>) -> Self {
        self.transition_with(ms, Ease::Out)
    }

    /// The same, along a curve of the caller's. A time that is a measure of the theme
    /// (`token::DURATION_FAST`) is read from the box's theme when it is mounted.
    pub fn transition_with(self, ms: impl Into<Amount>, ease: Ease) -> Self {
        let ms = ms.into();
        self.on_mount(move |tree, id| {
            let ms = tree.theme_at(id).amount(ms).max(0.0) as u64;
            tree.set_transition(id, Some(over(ms, ease)))
        })
    }

    /// Plays a timeline from when the box is mounted.
    pub fn animate(self, line: Timeline) -> Self {
        self.on_mount(move |tree, id| tree.set_timeline(id, line.0))
    }

    /// Plays a timeline from when the box is mounted, and again from the start each time
    /// what `start` reads changes: a count the app moves on, say.
    pub fn animate_on<T: 'static>(mut self, start: impl Fn() -> T + 'static, line: Timeline) -> Self {
        self.binders.push(Box::new(move |id, handle| {
            let writer = handle.clone();
            handle.bind(move || {
                start();
                let line = line.0.clone();
                writer.with(move |tree| tree.set_timeline(id, line));
            });
        }));
        self
    }
}
