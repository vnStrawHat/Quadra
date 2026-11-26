//! The bridge to assistive technology (specs/12, "Accessibility"): a window's semantic tree,
//! told to UI Automation by the adapter of `accesskit`, and what a screen reader asks of a
//! node, brought back as semantic actions. What the tree is to `accesskit` is
//! `quadra_a11y::bridge`, which every system's adapter takes.

use std::cell::RefCell;
use std::sync::mpsc::{Receiver, Sender, channel};

use accesskit::{ActionHandler, ActionRequest, ActivationHandler};
use accesskit_windows::Adapter;
use quadra_a11y::bridge::{Runs, request, tree_update};
use quadra_a11y::{SemanticAction, TreeUpdate};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

/// Posted to a window: assistive technology asked something of a node.
pub(crate) const WM_SEMANTIC: u32 = WM_APP + 2;

/// Requests arrive on a thread of the adapter's choosing: they are queued, and the window
/// is posted a message to come and take them on its own thread.
struct Requests {
    hwnd: isize,
    to: Sender<ActionRequest>,
}

impl ActionHandler for Requests {
    fn do_action(&mut self, request: ActionRequest) {
        if self.to.send(request).is_ok() {
            // SAFETY: posting to a window that is gone fails, and that is all it does.
            let _ = unsafe { PostMessageW(Some(HWND(self.hwnd as _)), WM_SEMANTIC, WPARAM(0), LPARAM(0)) };
        }
    }
}

/// Notes that the tree was asked for. It is given later, in an update: the app may be in
/// the middle of something when the question comes.
struct Asked(bool);

impl ActivationHandler for Asked {
    fn request_initial_tree(&mut self) -> Option<accesskit::TreeUpdate> {
        self.0 = true;
        None
    }
}

/// A window's side of the bridge. Nothing is borrowed across a call that can send the window
/// a message: answering one, and raising events, can bring `WM_GETOBJECT` back in.
pub(crate) struct Access {
    adapter: RefCell<Adapter>,
    requests: Receiver<ActionRequest>,
    /// The runs of the inputs told so far.
    runs: RefCell<Runs>,
}

impl Access {
    /// Must not be called while the window handles `WM_GETOBJECT`.
    pub fn new(hwnd: HWND) -> Access {
        let (to, requests) = channel();
        let adapter = Adapter::new(hwnd, false, Requests { hwnd: hwnd.0 as isize, to });
        Access { adapter: RefCell::new(adapter), requests, runs: RefCell::default() }
    }

    /// Answers `WM_GETOBJECT`: the result, if the message was one for the adapter, and
    /// whether this was the first time the tree was asked for.
    pub fn object(&self, wparam: WPARAM, lparam: LPARAM) -> (Option<LRESULT>, bool) {
        let mut asked = Asked(false);
        let result = self.adapter.borrow_mut().handle_wm_getobject(wparam, lparam, &mut asked);
        (result.map(Into::into), asked.0)
    }

    pub fn window_focused(&self, focused: bool) {
        let events = self.adapter.borrow_mut().update_window_focus_state(focused);
        if let Some(events) = events {
            events.raise();
        }
    }

    pub fn update(&self, update: &TreeUpdate, scale: f32) {
        let mut runs = self.runs.borrow_mut();
        let events = self.adapter.borrow_mut().update_if_active(|| tree_update(update, scale, &mut runs));
        drop(runs);
        if let Some(events) = events {
            events.raise();
        }
    }

    /// What was asked since the last time.
    pub fn requests(&self) -> impl Iterator<Item = (u64, SemanticAction)> + '_ {
        self.requests.try_iter().filter_map(|asked| request(asked, &self.runs.borrow()))
    }
}
