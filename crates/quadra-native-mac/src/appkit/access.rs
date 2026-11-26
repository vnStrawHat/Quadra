//! Readers (specs/12, "Accessibility"; specs/23): the window's semantic tree, told to the
//! system's accessibility by the adapter of `accesskit`, which answers for the view what a
//! reader asks of it. What the adapter asks of the app is given when the loop next turns:
//! it asks with its own tree borrowed, and the app's answer may be an update of that tree.

use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

use accesskit::{ActionHandler, ActionRequest, ActivationHandler};
use accesskit_macos::SubclassingAdapter;
use objc2::rc::Retained;
use quadra_a11y::TreeUpdate;
use quadra_a11y::bridge::{Runs, request, tree_update};
use quadra_platform::PlatformEvent;

use super::view::View;
use super::window::State;

/// The tree is asked for, once: it is given later, in an update.
struct Asked(Rc<State>);

impl ActivationHandler for Asked {
    fn request_initial_tree(&mut self) -> Option<accesskit::TreeUpdate> {
        self.0.send_later(PlatformEvent::SemanticsWanted);
        None
    }
}

/// What a reader asks of a node, on the main thread.
struct Requests {
    state: Rc<State>,
    runs: Rc<RefCell<Runs>>,
}

impl ActionHandler for Requests {
    fn do_action(&mut self, asked: ActionRequest) {
        if let Some((node, action)) = request(asked, &self.runs.borrow()) {
            self.state.send_later(PlatformEvent::Semantic { node, action });
        }
    }
}

/// A window's side of the bridge.
pub(crate) struct Access {
    adapter: RefCell<SubclassingAdapter>,
    /// The runs of the inputs told so far.
    runs: Rc<RefCell<Runs>>,
}

impl Access {
    /// For a view that is in no window yet: the adapter gives it a class of its own, over
    /// the view's, that answers what accessibility asks of a view.
    pub fn new(view: &Retained<View>, state: &Rc<State>) -> Access {
        let runs = Rc::<RefCell<Runs>>::default();
        let requests = Requests { state: state.clone(), runs: runs.clone() };
        let view = Retained::as_ptr(view) as *mut c_void;
        // SAFETY: the pointer is of a view that is alive, which the adapter keeps.
        let adapter = unsafe { SubclassingAdapter::new(view, Asked(state.clone()), requests) };
        Access { adapter: RefCell::new(adapter), runs }
    }

    /// The window has the keys, or has them no more.
    pub fn focused(&self, focused: bool) {
        let events = self.adapter.borrow_mut().update_view_focus_state(focused);
        if let Some(events) = events {
            events.raise();
        }
    }

    pub fn update(&self, update: &TreeUpdate, scale: f32) {
        let told = || tree_update(update, scale, &mut self.runs.borrow_mut());
        let events = self.adapter.borrow_mut().update_if_active(told);
        if let Some(events) = events {
            events.raise();
        }
    }
}
