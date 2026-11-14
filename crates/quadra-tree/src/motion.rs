//! The pose of a node and the changes that ease (specs/05, "The pose and transitions").
//!
//! The pose is how what a node paints is placed over its box: constants of the node's layer,
//! so a pose that changes paints nothing. A node with a transition goes to a new background,
//! border colour or pose over a time, and a node with a timeline plays its steps: the tree
//! moves both on at the start of each frame. No app code runs.

use std::time::{Duration, Instant};

use quadra_prim::Ease;

use crate::node::{NodeId, Pose, Style};

mod eased;
use crate::patch::{Part, Patch};
use crate::theme::Theme;
use crate::tree::Tree;
use eased::Eased;

/// How a change of what can ease is gone through: over how long, along which curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transition {
    pub over: Duration,
    pub ease: Ease,
}

/// A step of a timeline.
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    /// Over a time, to what it sets of what can ease. One that sets nothing is a pause.
    To(Transition, Vec<Patch>),
    /// Steps that start together, each over its own time; the step lasts as long as the
    /// longest. Of two that set the same thing, the later shows.
    Together(Vec<Step>),
    /// A timeline played as a step, its plays from where the steps before it left (one that
    /// plays for ever plays once).
    Line(Box<Timeline>),
}

impl Step {
    fn length(&self) -> f32 {
        match self {
            Step::To(how, _) => how.over.as_secs_f32(),
            Step::Together(steps) => steps.iter().map(Step::length).fold(0.0, f32::max),
            Step::Line(line) => line.once() * line.plays.max(1) as f32,
        }
    }

    /// What shows `left` into the step, from `now`; at its length or past it, where it ends.
    fn at(&self, now: Eased, left: f32, theme: &Theme) -> Eased {
        match self {
            Step::To(how, to) => {
                let (to, length) = (now.with(to, theme), how.over.as_secs_f32());
                if left >= length { to } else { now.lerp(to, how.ease.at(left / length)) }
            }
            Step::Together(steps) => steps.iter().fold(now, |shown, step| step.at(shown, left, theme)),
            Step::Line(line) => line.at(Duration::from_secs_f32(left.max(0.0)), now, theme).0,
        }
    }
}

/// Steps played one after another, from when the timeline is given to a node.
#[derive(Clone, Debug, PartialEq)]
pub struct Timeline {
    pub steps: Vec<Step>,
    /// How many times the steps are played; 0 is for ever.
    pub plays: u32,
    /// Every second play goes backwards.
    pub alternate: bool,
}

impl Timeline {
    /// Whether a step of it, or of what it holds, sets the pose.
    fn poses(&self) -> bool {
        fn sets(step: &Step) -> bool {
            match step {
                Step::To(_, to) => to.iter().any(|patch| patch.part() == Part::Pose),
                Step::Together(steps) => steps.iter().any(sets),
                Step::Line(line) => line.poses(),
            }
        }
        self.steps.iter().any(sets)
    }

    /// How long one play is, in seconds.
    fn once(&self) -> f32 {
        self.steps.iter().map(Step::length).sum()
    }

    /// What shows `after` the start, for a style that says `rest`, and whether the last
    /// play is over. Every play starts from `rest`.
    fn at(&self, after: Duration, rest: Eased, theme: &Theme) -> (Eased, bool) {
        // In 64 bits: a timeline that plays for days keeps its milliseconds.
        let (once, after) = (self.once() as f64, after.as_secs_f64());
        let over = once <= 0.0 || (self.plays != 0 && after >= once * self.plays as f64);
        let play = if over { self.plays.max(1) - 1 } else { (after / once) as u32 };
        let into = if over { once } else { after - once * play as f64 };
        let mut left = (if self.alternate && play % 2 == 1 { once - into } else { into }) as f32;
        let mut now = rest;
        for step in &self.steps {
            let length = step.length();
            if left < length {
                return (step.at(now, left, theme), over);
            }
            (now, left) = (step.at(now, length, theme), left - length);
        }
        (now, over)
    }
}

/// A change under way.
#[derive(Clone, Copy)]
struct Run {
    from: Eased,
    to: Eased,
    since: Instant,
    how: Transition,
}

/// What a node whose style eases keeps.
pub(crate) struct Motion {
    how: Option<Transition>,
    /// The frame the transition was set in: what the style says in that frame shows at once.
    born: u64,
    run: Option<Run>,
    /// The timeline that plays, since when, and what the node's style says meanwhile.
    line: Option<(Timeline, Instant, Eased)>,
    /// What the node's style said when the last timeline ended: one played again starts
    /// from it, not from where that one left the node.
    rest: Option<Eased>,
}

impl Tree {
    fn motion(&mut self, id: NodeId) -> Option<&mut Motion> {
        self.get(id)?;
        self.keep_own_style(id); // every change of the style goes through `resolve` from now
        let born = self.frames;
        let fresh = || Box::new(Motion { how: None, born, run: None, line: None, rest: None });
        Some(&mut **self.nodes[id.index as usize].motion.get_or_insert_with(fresh))
    }

    /// Makes the changes of a node's background, border colour and pose ease, or with `None`
    /// happen at once again. What the node's style says in the frame this is set in shows at
    /// once: a node is not mounted in the middle of a change.
    pub fn set_transition(&mut self, id: NodeId, how: Option<Transition>) {
        let born = self.frames;
        if let Some(motion) = self.motion(id) {
            (motion.how, motion.born) = (how, born);
        }
    }

    /// Plays a timeline on a node, from now and from what its style says.
    pub fn set_timeline(&mut self, id: NodeId, line: Timeline) {
        if self.get(id).is_none() {
            return;
        }
        if line.poses() {
            self.make_layer(id, false);
        }
        let (shown, now) = (self.eased_now(id), self.handle().now());
        let Some(motion) = self.motion(id) else { return };
        // Played again, it starts from what the style says, not from where the last play is.
        let moving = motion.line.is_some() || motion.run.is_some();
        let rest = motion.line.as_ref().map(|(_, _, rest)| *rest).or(motion.rest).unwrap_or(shown);
        (motion.line, motion.run) = (Some((line, now, rest)), None);
        if !moving {
            self.moving.push(id);
        }
    }

    /// The pose a node shows.
    pub fn pose(&self, id: NodeId) -> Pose {
        self.get(id).and_then(|node| node.layer.as_ref()).map_or(Pose::default(), |layer| layer.pose)
    }

    /// Gives a node the pose it is composed with. A node with a pose is a layer.
    pub(crate) fn put_pose(&mut self, id: NodeId, pose: Pose) {
        if self.pose(id) == pose || self.get(id).is_none() {
            return;
        }
        self.make_layer(id, false);
        if let Some(layer) = self.nodes[id.index as usize].layer.as_mut() {
            layer.pose = pose;
            self.recompose.push(id);
        }
    }

    fn eased_now(&self, id: NodeId) -> Eased {
        let paint = self.node(id).paint;
        Eased { background: paint.background, border: paint.border_color, pose: self.pose(id) }
    }

    /// Called with the style a node resolved to, before it is given to the node. Under a
    /// timeline the style is what the timeline plays from, and what it shows now goes in its
    /// place. With a transition, where the style says something else of what can ease than
    /// the node is going to, a change starts from what shows; and while one is under way,
    /// what shows stays in the style in place of where it is going.
    pub(crate) fn ease(&mut self, id: NodeId, style: &mut Style) {
        let (frames, snap, theme) = (self.frames, self.snap, self.theme_at(id).clone());
        if self.node(id).motion.is_none() {
            return;
        }
        let (target, shown, now) = (Eased::of(style), self.eased_now(id), self.handle().now());
        let Some(motion) = self.nodes[id.index as usize].motion.as_mut() else { return };
        motion.rest = motion.rest.map(|_| target);
        if let Some((line, since, rest)) = &mut motion.line {
            *rest = target;
            return line.at(now - *since, target, &theme).0.into(style);
        }
        let Some(how) = motion.how.filter(|_| !snap && motion.born != frames) else {
            return motion.run = None;
        };
        if target != motion.run.map_or(shown, |run| run.to) {
            if motion.run.is_none() {
                self.moving.push(id);
            }
            motion.run = Some(Run { from: shown, to: target, since: now, how });
        }
        shown.into(style);
    }

    /// At the start of a frame: every change under way is moved on to where the window's
    /// clock says it is. Colours flag the node for paint, the pose its layer for compose.
    pub(crate) fn motion_pass(&mut self) {
        if self.moving.is_empty() {
            return;
        }
        let (now, mut moving) = (self.handle().now(), std::mem::take(&mut self.moving));
        moving.retain(|&id| {
            let theme = self.theme_at(id).clone();
            let Some(motion) = self.get_mut(id).and_then(|node| node.motion.as_mut()) else { return false };
            let (value, done) = match (&motion.line, motion.run) {
                (Some((line, since, rest)), _) => line.at(now - *since, *rest, &theme),
                (None, Some(run)) => {
                    let part = (now - run.since).as_secs_f32() / run.how.over.as_secs_f32();
                    // A time of nothing is over at once: the division says so, or is not a number.
                    let done = part.is_nan() || part >= 1.0;
                    (if done { run.to } else { run.from.lerp(run.to, run.how.ease.at(part)) }, done)
                }
                (None, None) => return false,
            };
            if done {
                motion.rest = motion.line.take().map(|(_, _, rest)| rest).or(motion.rest);
                motion.run = None;
            }
            let mut paint = self.node(id).paint;
            (paint.background, paint.border_color) = (value.background, value.border);
            if paint != self.node(id).paint {
                self.stats.nodes_styled += 1;
            }
            self.put_paint(id, paint);
            self.put_pose(id, value.pose);
            !done
        });
        self.moving = moving;
    }
}
