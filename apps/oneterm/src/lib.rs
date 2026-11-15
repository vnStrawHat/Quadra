//! 1Term (specs/19): the window, its title bar, the rail of tabs, the workspace of spaces,
//! the status bar. Where a session's program comes from is the caller's: a local shell in the
//! app ([`local_shell`]), `quadra_term::echo` in tests.

mod chrome;
mod git;
mod local;
mod marks;
mod model;
mod page;
mod prefs;
mod rail;
mod shells;
mod signals;
mod spaces;
mod strip;
mod tab;

use std::rc::Rc;

use quadra::{AnyView, Display, Keymap, Signal, View, Window, column, dynamic, row, signal, use_context};
use quadra_term::Session;

pub use local::local_shell;
pub use model::{Command, Model, Space, Tab};
pub use prefs::{Prefs, theme};
pub use shells::{Launch, Shell};
pub use signals::{percent, state_word};

/// Makes the session of a new space: it is called in the scope the space lives in.
pub type NewSession = Rc<dyn Fn(Launch) -> Session>;

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
        let open = model.settings_open();
        let shows = move |on: bool| move || if open.get() == on { Display::Flex } else { Display::None };
        let body = column()
            .grow(1.0)
            .min_height(0.0)
            .child(body.container(column().grow(1.0).display(shows(false))))
            .child(page::page(&model).display(shows(true)));
        quadra_design::root()
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
            .child(self.model.notices().host())
    }
}

/// Does a command; false when no tab is left.
fn run(model: &Model, command: Command) -> bool {
    match command {
        Command::NewTab => model.new_tab(),
        Command::Split => model.split(),
        Command::CloseSpace => return model.close_focused(),
        Command::FocusNext(forward) => model.focus_next(forward),
    }
    true
}
