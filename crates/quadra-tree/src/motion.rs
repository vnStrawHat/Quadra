//! The pose of a node and the changes that ease (specs/05, "The pose and transitions").
//!
//! The pose is how what a node paints is placed over its box: constants of the node's layer,
//! so a pose that changes paints nothing. A node with a transition goes to a new background,
//! border colour or pose over a time, and a node with a timeline plays its steps: the tree
//! moves both on at the start of each frame. No app code runs.

use std::time::{Duration, Instant};

use quadra_prim::{Color, Ease, Point};

use crate::node::{NodeId, Pose, Style};
use crate::patch::{Part, Patch};
use crate::theme::Theme;
use crate::tree::Tree;

/// How a change of what can ease is gone through: over how long, along which curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transition {
    pub over: Duration,
    pub ease: Ease,
}

/// A step of a timeline: over a time, to what it sets of what can ease. One that sets
/// nothing is a pause.
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub how: Transition,
    pub to: Vec<Patch>,
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

/// What of a style can ease.
#[derive(Clone, Copy, PartialEq)]
struct Eased {
    background: Color,
    border: Color,
    pose: Pose,
}

impl Eased {
    fn of(style: &Style) -> Eased {
        Eased { background: style.paint.background, border: style.paint.border_color, pose: style.pose }
    }

    fn into(self, style: &mut Style) {
        (style.paint.background, style.paint.border_color, style.pose) =
            (self.background, self.border, self.pose);
    }

    fn lerp(self, to: Eased, t: f32) -> Eased {
        let mix = |a: f32, b: f32| a + (b - a) * t;
        let (a, b) = (self.pose, to.pose);
        let pose = Pose {
            translate: Point::new(mix(a.translate.x, b.translate.x), mix(a.translate.y, b.translate.y)),
            scale: mix(a.scale, b.scale),
            rotate: mix(a.rotate, b.rotate),
            opacity: mix(a.opacity, b.opacity),
        };
        Eased {
            background: self.background.lerp(to.background, t),
            border: self.border.lerp(to.border, t),
            pose,
        }
    }

    /// With what a step sets.
    fn with(mut self, patches: &[Patch], theme: &Theme) -> Eased {
        for patch in patches {
            match *patch {
                Patch::Background(v) => self.background = theme.resolve(v),
                Patch::BorderColor(v) => self.border = theme.resolve(v),
                Patch::Translate(v) => self.pose.translate = v,
                Patch::Scale(v) => self.pose.scale = v,
                Patch::Rotate(v) => self.pose.rotate = v,
                Patch::Opacity(v) => self.pose.opacity = v,
                _ => {} // nothing else eases
            }
        }
        self
    }
}

impl Timeline {
    /// What shows `after` the start, for a style that says `rest`, and whether the last
    /// play is over. Every play starts from `rest`.
    fn at(&self, after: Duration, rest: Eased, theme: &Theme) -> (Eased, bool) {
        let length = |step: &Step| step.how.over.as_secs_f32();
        // In 64 bits: a timeline that plays for days keeps its milliseconds.
        let (once, after) = (self.steps.iter().map(length).sum::<f32>() as f64, after.as_secs_f64());
        let over = once <= 0.0 || (self.plays != 0 && after >= once * self.plays as f64);
        let play = if over { self.plays.max(1) - 1 } else { (after / once) as u32 };
        let into = if over { once } else { after - once * play as f64 };
        let mut left = (if self.alternate && play % 2 == 1 { once - into } else { into }) as f32;
        let mut now = rest;
        for step in &self.steps {
            let to = now.with(&step.to, theme);
            if left < length(step) {
                return (now.lerp(to, step.how.ease.at(left / length(step))), over);
            }
            (now, left) = (to, left - length(step));
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
}

impl Tree {
    fn motion(&mut self, id: NodeId) -> Option<&mut Motion> {
        self.get(id)?;
        self.keep_own_style(id); // every change of the style goes through `resolve` from now
        let born = self.frames;
        let fresh = || Box::new(Motion { how: None, born, run: None, line: None });
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
        if line.steps.iter().flat_map(|step| &step.to).any(|patch| patch.part() == Part::Pose) {
            self.make_layer(id, false);
        }
        let (rest, now) = (self.eased_now(id), self.handle().now());
        if let Some(motion) = self.motion(id) {
            (motion.line, motion.run) = (Some((line, now, rest)), None);
        }
        self.moving.push(id);
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
        let (frames, snap, theme) = (self.frames, self.snap, self.theme.clone());
        if self.node(id).motion.is_none() {
            return;
        }
        let (target, shown, now) = (Eased::of(style), self.eased_now(id), self.handle().now());
        let Some(motion) = self.nodes[id.index as usize].motion.as_mut() else { return };
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
        let (now, theme, mut moving) =
            (self.handle().now(), self.theme.clone(), std::mem::take(&mut self.moving));
        moving.retain(|&id| {
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
                (motion.line, motion.run) = (None, None);
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
