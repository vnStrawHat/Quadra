//! How the platform reaches the app's handler (specs/10, "The handler is never entered
//! twice"): what arrives while a handler method runs waits in a queue.

use quadra_platform::{HitRegion, PlatformEvent, TimerId, WindowId};
use quadra_prim::Point;

use crate::platform::Shared;

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

    /// Asks the handler what a point of a window is to the OS. It must be answered at once:
    /// while a handler method runs, or before there is a handler, the point is the app's.
    pub fn hit_test(&self, window: WindowId, at: Point) -> HitRegion {
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
}
