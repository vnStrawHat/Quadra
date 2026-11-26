//! How the platform reaches the app's handler (specs/10, "The handler is never entered
//! twice"): what arrives while a handler method runs waits in a queue.

use quadra_platform::{PlatformEvent, TimerId, WindowId};

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

    /// Gives each window that asked for a frame its frame.
    pub(crate) fn frames(&self) {
        let asking: Vec<WindowId> = {
            let windows = self.windows.borrow();
            let alive = windows.iter().filter_map(|it| it.upgrade());
            alive.filter(|it| it.wants_frame.replace(false)).map(|it| it.id).collect()
        };
        for window in asking {
            self.deliver(Delivery::Frame(window));
        }
    }
}
