//! How the platform reaches the app's handler (specs/10, "The handler is never entered
//! twice"): what arrives while a handler method runs waits in a queue.

use quadra_platform::{HitRegion, PlatformEvent, TimerId, WindowId};
use quadra_prim::Point;

use super::Shared;

pub(crate) enum Delivery {
    Event(WindowId, PlatformEvent),
    Frame(WindowId),
    Timer(TimerId),
    Wake,
}

impl Shared {
    /// Hands something to the handler: now, or when the handler that is running returns.
    pub fn deliver(&self, delivery: Delivery) {
        self.queue.borrow_mut().push_back(delivery);
        self.drain();
    }

    /// Hands something to the handler when the loop next turns: for what is asked while
    /// something the handler's answer may reach is in use.
    pub fn later(&self, delivery: Delivery) {
        self.queue.borrow_mut().push_back(delivery);
        super::turn();
    }

    pub(crate) fn drain(&self) {
        let Some(mut handler) = self.handler.get() else { return };
        if self.busy.replace(true) {
            return;
        }
        loop {
            let next = self.queue.borrow_mut().pop_front();
            let Some(delivery) = next else { break };
            // SAFETY: the pointer is the handler `run` borrowed exclusively, and `run` clears
            // it before returning. `busy` makes this the only reference made from it at a
            // time, and `run` does not touch its own reference while the pointer is set.
            let handler = unsafe { handler.as_mut() };
            match delivery {
                Delivery::Event(window, event) => handler.event(window, event),
                Delivery::Frame(window) => handler.frame(window),
                Delivery::Timer(id) => handler.timer(id),
                Delivery::Wake => handler.wake(),
            }
        }
        self.busy.set(false);
    }

    /// Asks the handler what a point of a window is to the system. It must be answered at
    /// once: while a handler method runs, or before there is a handler, the point is the
    /// app's.
    pub(crate) fn hit_test(&self, window: WindowId, at: Point) -> HitRegion {
        let Some(mut handler) = self.handler.get() else { return HitRegion::Client };
        if self.busy.replace(true) {
            return HitRegion::Client;
        }
        // SAFETY: as in `drain`: `busy` makes this the only reference to the handler.
        let region = unsafe { handler.as_mut() }.hit_test(window, at);
        self.busy.set(false);
        self.drain(); // what arrived while it answered
        region
    }

    /// The menu bar's Quit: every window is asked to close, and with none the loop ends.
    pub(crate) fn close_all(&self) {
        let open: Vec<WindowId> =
            self.windows.borrow().iter().filter_map(|it| it.upgrade()).map(|it| it.id).collect();
        if open.is_empty() {
            self.quitting.set(true);
        }
        for window in open {
            self.deliver(Delivery::Event(window, PlatformEvent::CloseRequested));
        }
    }

    /// Gives each window that asked for a frame its frame.
    pub(crate) fn frames(&self) {
        let asking: Vec<WindowId> = {
            let windows = self.windows.borrow();
            let alive = windows.iter().filter_map(|it| it.upgrade());
            alive.filter(|it| it.wants_frame.replace(false)).map(|it| it.id).collect()
        };
        for window in asking {
            self.deliver(Delivery::Frame(window));
            self.place_buttons(window); // the bar they stand in is laid out now
        }
    }
}
