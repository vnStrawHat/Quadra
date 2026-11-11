//! Pointer dispatch (specs/09, "Dispatch", "Hover and press"): an event goes down the path to
//! its target and back up; hover follows the pointer; a click is a press and a release that
//! share a node. Handlers write signals and never the tree, so the path stays valid while it
//! is walked.

use std::time::{Duration, Instant};

use quadra_platform::{Mods, MouseButton};
use quadra_prim::Point;
use quadra_reactive::batch;

use crate::input::{Handler, PointerEvent, PointerKind};
use crate::node::NodeId;
use crate::style::State;
use crate::tree::Tree;

/// Two clicks count as a double click within this time and this distance of each other.
const MULTI_CLICK: Duration = Duration::from_millis(500);
const MULTI_CLICK_DISTANCE: f32 = 4.0;
/// What one line of the wheel scrolls, in logical pixels.
const WHEEL_LINE: f32 = 40.0;

/// What the OS reports of the pointer (the platform's kinds, named here so the tree's input
/// can be driven without a platform event).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerInput {
    Move,
    Down,
    Up,
    /// The pointer left the window.
    Leave,
}

impl Tree {
    /// Takes a pointer event of the window: hit tests, updates hover, dispatches to the
    /// handlers on the path, and synthesizes clicks. `now` counts double clicks. Everything
    /// the handlers write is one batch.
    pub fn pointer(&mut self, input: PointerInput, button: MouseButton, at: Point, mods: Mods, now: Instant) {
        batch(|| {
            self.input.mods = mods;
            self.input.pointer = (input != PointerInput::Leave).then_some(at);
            match input {
                PointerInput::Leave => self.rehover(),
                PointerInput::Move => {
                    self.rehover();
                    if let Some(target) = self.input.capture.or_else(|| self.hit_test(at)) {
                        self.dispatch_pointer(PointerKind::Move, target, button, 0);
                    }
                }
                PointerInput::Down => {
                    self.rehover();
                    let Some(target) = self.hit_test(at) else { return };
                    self.input.pressed = self.path(target);
                    self.set_states(State::ACTIVE, true);
                    let path = self.input.pressed.clone();
                    self.focus_on_press(&path);
                    self.input.capture = self.dispatch_pointer(PointerKind::Down, target, button, 0).captured;
                }
                PointerInput::Up => self.release(button, at, now),
            }
        });
    }

    /// Takes a wheel event of the window: `delta` in lines, or in pixels when `precise` (a
    /// touchpad). It goes up from what is under the pointer: a scroll container takes what it
    /// can scroll, and the rest goes on to the ones around it.
    pub fn wheel(&mut self, delta: Point, precise: bool, at: Point, mods: Mods) {
        batch(|| {
            (self.input.mods, self.input.pointer) = (mods, Some(at));
            self.rehover();
            let Some(target) = self.hit_test(at) else { return };
            let by = if precise { 1.0 } else { WHEEL_LINE };
            let delta = Point::new(delta.x * by, delta.y * by);
            self.send(PointerEvent { delta, ..self.event(PointerKind::Wheel, target) });
        });
    }

    fn release(&mut self, button: MouseButton, at: Point, now: Instant) {
        let under = self.hit_test(at);
        self.set_states(State::ACTIVE, false);
        let pressed = std::mem::take(&mut self.input.pressed);
        if let Some(target) = self.input.capture.take().or(under) {
            self.dispatch_pointer(PointerKind::Up, target, button, 0);
        }
        // A click belongs to the deepest node that both the press and the release were on.
        let shared =
            under.map(|u| self.path(u)).and_then(|path| path.into_iter().find(|n| pressed.contains(n)));
        if let Some(target) = shared.filter(|&n| self.is_alive(n)) {
            let again = self.input.last_click.filter(|&(when, place, which, _)| {
                let (dx, dy) = (at.x - place.x, at.y - place.y);
                which == button
                    && now.duration_since(when) <= MULTI_CLICK
                    && dx.hypot(dy) <= MULTI_CLICK_DISTANCE
            });
            let clicks = again.map_or(1, |(.., count)| count.saturating_add(1));
            self.input.last_click = Some((now, at, button, clicks));
            self.dispatch_pointer(PointerKind::Click, target, button, clicks);
        }
        self.rehover(); // the capture is over: what is under the pointer may be something else
    }

    /// Runs the handlers of a kind along the path of `target`: from the root down those that
    /// capture, then from the target up the others, until one stops it.
    fn dispatch_pointer(
        &mut self,
        kind: PointerKind,
        target: NodeId,
        button: MouseButton,
        clicks: u8,
    ) -> PointerEvent {
        let event = PointerEvent { button, clicks, ..self.event(kind, target) };
        self.send(event)
    }

    /// An event for a node at the pointer's place, with nothing else said.
    fn event(&self, kind: PointerKind, target: NodeId) -> PointerEvent {
        let at = self.input.pointer.unwrap_or(Point::ZERO);
        PointerEvent {
            kind,
            button: MouseButton::None,
            at,
            local: at,
            mods: self.input.mods,
            clicks: 0,
            delta: Point::ZERO,
            target,
            node: target,
            stopped: false,
            captured: None,
        }
    }

    fn send(&mut self, mut event: PointerEvent) -> PointerEvent {
        let path = self.path(event.target);
        for capture in [true, false] {
            let along = (0..path.len()).map(|i| if capture { path[path.len() - 1 - i] } else { path[i] });
            for node in along {
                self.run_handlers(node, capture, &mut event);
                if event.stopped {
                    return event;
                }
            }
        }
        event
    }

    /// Runs the node's handlers for the event's kind, if it has any.
    fn run_handlers(&mut self, id: NodeId, capture: bool, event: &mut PointerEvent) {
        let kind = event.kind;
        let wanted = |h: &Handler| matches!(h, Handler::Pointer { kind: k, capture: c, .. } if *k == kind && *c == capture);
        if !self.get(id).is_some_and(|node| node.handlers.iter().any(wanted)) {
            return;
        }
        let origin = self.window_rect(id).origin();
        (event.node, event.local) = (id, Point::new(event.at.x - origin.x, event.at.y - origin.y));
        for handler in &mut self.nodes[id.index as usize].handlers {
            if let Handler::Pointer { kind: k, capture: c, run } = handler
                && *k == kind
                && *c == capture
                && !event.stopped
            {
                run(event);
            }
        }
    }

    /// Brings the hover path in step with what is under the pointer: the nodes that are no
    /// longer under it are left, the deepest first, then the new ones are entered from the
    /// outside in. While a node holds the pointer, hover stays as it was.
    pub(crate) fn rehover(&mut self) {
        if self.input.capture.is_some() {
            return;
        }
        let under = self.input.pointer.and_then(|at| self.hit_test(at));
        let now = under.map(|target| self.path(target)).unwrap_or_default();
        if now == self.input.hover {
            return;
        }
        let was = std::mem::replace(&mut self.input.hover, now.clone());
        for &node in was.iter().filter(|n| !now.contains(n)) {
            self.set_state(node, State::HOVER, false);
            self.visit(PointerKind::Leave, node);
        }
        for &node in now.iter().rev().filter(|n| !was.contains(n)) {
            self.set_state(node, State::HOVER, true);
            self.visit(PointerKind::Enter, node);
        }
    }

    /// Sets or clears a state on the path of the press.
    fn set_states(&mut self, state: State, on: bool) {
        for i in 0..self.input.pressed.len() {
            self.set_state(self.input.pressed[i], state, on);
        }
    }

    /// An enter or a leave: for one node, not along a path.
    fn visit(&mut self, kind: PointerKind, node: NodeId) {
        let mut event = self.event(kind, node);
        self.run_handlers(node, false, &mut event);
    }
}
