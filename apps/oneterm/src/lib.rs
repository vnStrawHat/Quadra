//! 1Term (specs/19): the window, its title bar, the workspace of terminals, the status bar.
//! Where a session's program comes from is the caller's: a local shell in the app
//! ([`local_shell`]), `quadra_term::echo` in tests.

mod chrome;
mod local;

use std::rc::Rc;

use quadra::{Keymap, Signal, View, column, signal};
use quadra_term::{Session, terminal};

pub use local::local_shell;

/// Makes the session of a new space: it is called in the scope the space lives in.
pub type NewSession = Rc<dyn Fn() -> Session>;

/// The app's keys: the kit's and the terminal's.
pub fn keymap() -> Keymap {
    let mut keys = quadra_design::keymap();
    quadra_term::bind_keys(&mut keys);
    keys
}

/// What the window shows. M1 has one space: a shell filling the workspace.
pub struct OneTerm {
    pub session: Session,
    /// The time of the status bar, kept by a timer.
    pub clock: Signal<String>,
}

impl OneTerm {
    pub fn new(new_session: &NewSession) -> OneTerm {
        OneTerm { session: new_session(), clock: signal(String::new()) }
    }

    pub fn view(&self) -> impl View + use<> {
        let s = &self.session;
        let space = column().grow(1.0).bg(quadra_design::token::CANVAS).child(terminal(s).autofocus());
        quadra_design::root()
            .child(chrome::title_bar(s))
            .child(space)
            .child(chrome::status_bar(s, self.clock))
    }
}
