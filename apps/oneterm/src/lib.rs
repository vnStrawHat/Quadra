//! 1Term (specs/19): the window, its title bar, the rail of tabs, the workspace of spaces,
//! the status bar. Where a session's program comes from is the caller's: a local shell in the
//! app ([`local_shell`]), `quadra_term::echo` in tests.

mod chrome;
mod connect;
mod git;
mod hostlist;
mod hosts;
mod local;
mod lock;
mod locked;
mod marks;
mod model;
mod page;
mod prefs;
mod rail;
mod remote;
mod shells;
mod signals;
mod spaces;
mod strip;
mod tab;

use std::rc::Rc;

use quadra::{AnyView, Keymap, Signal, View, Window, column, div, dynamic, row, signal, use_context};
use quadra_term::Session;

pub use hosts::Host;
pub use local::local_shell;
pub use model::{Command, Model, Space, Tab};
pub use prefs::{Prefs, theme};
pub use remote::remote_shell;
pub use shells::{Launch, Remote, Shell};
pub use signals::{percent, state_word};

/// Makes the session of a new space: it is called in the scope the space lives in.
pub type NewSession = Rc<dyn Fn(Launch) -> Session>;

/// The app's sessions: a host's shell when the launch names one, else a local one.
pub fn start(launch: Launch) -> Session {
    if launch.ssh.is_some() { remote_shell(launch) } else { local_shell(launch) }
}

/// The app's keys: the kit's, the terminal's, and the tabs' and spaces'.
pub fn keymap() -> Keymap {
    let mut keys = quadra_design::keymap();
    quadra_term::bind_keys(&mut keys);
    let ours = [
        ("ctrl-t", Command::NewTab),
        ("ctrl-d", Command::Split),
        ("ctrl-w", Command::CloseSpace),
        ("ctrl-alt-right", Command::FocusNext(true)),
        ("ctrl-alt-left", Command::FocusNext(false)),
        ("ctrl-shift-l", Command::Lock),
        ("ctrl-shift-k", Command::QuickConnect),
    ];
    for (stroke, command) in ours {
        keys.bind(stroke, command).expect("the app's bindings are well formed");
    }
    keys
}

/// What the window shows.
pub struct OneTerm {
    pub model: Model,
    /// The time of the status bar, kept by a timer.
    pub clock: Signal<String>,
}

impl OneTerm {
    /// The window's model, with one tab of one space: made where the window's view is built.
    pub fn new(new_session: NewSession) -> OneTerm {
        OneTerm { model: Model::new(new_session, Prefs::load()), clock: signal(String::new()) }
    }

    pub fn view(&self) -> impl View + use<> {
        let (model, window) = (self.model.clone(), use_context::<Window>());
        // The tabs in the rail at the left, or in a strip above the spaces.
        let (vertical, shown) = (model.vertical(), model.clone());
        let body = dynamic(move || match vertical.get() {
            true => AnyView::new(row().grow(1.0).child(rail::rail(&shown)).child(spaces::workspace(&shown))),
            false => {
                AnyView::new(column().grow(1.0).child(strip::strip(&shown)).child(spaces::workspace(&shown)))
            }
        });
        // Locked, only the lock's card is displayed.
        let (open, lock) = (model.ui().settings, model.lock());
        let shows = move |on: bool| move || !lock.locked.get() && open.get() == on;
        let body = column()
            .grow(1.0)
            .min_height(0.0)
            .child(body.container(column().grow(1.0).display(locked::shown(shows(false)))))
            .child(page::page(&model).display(locked::shown(shows(true))))
            .child(locked::Card(model.clone()));
        let cards =
            div().display(locked::shown(move || !lock.locked.get())).child(self.model.notices().host());
        locked::watches(quadra_design::root(), lock)
            .on_action::<Command>(move |&command| {
                if !run(&model, command)
                    && let Some(window) = &window
                {
                    window.close_now(); // the last tab went
                }
            })
            .child(chrome::title_bar(&self.model))
            .child(body)
            .child(chrome::status_bar(&self.model, self.clock))
            .child(cards)
            .child(connect::dialog(&self.model))
            .child(hostlist::add_dialog(&self.model))
    }
}

/// Does a command; false when no tab is left. Locked, only locking does anything.
fn run(model: &Model, command: Command) -> bool {
    model.lock().active();
    if model.lock().locked.get_untracked() {
        return true;
    }
    match command {
        Command::NewTab => model.new_tab(),
        Command::Split => model.split(),
        Command::CloseSpace => return model.close_focused(),
        Command::FocusNext(forward) => model.focus_next(forward),
        Command::Lock => model.lock().lock_now(),
        Command::QuickConnect => model.ui().connecting.set(true),
    }
    true
}
