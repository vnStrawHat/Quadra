//! What programs signal (specs/19, "Signals"): OSC 7501 status, OSC 9;4 progress, the bell in
//! a space that is not looked at, OSC 9 notifications. A tab sums its spaces; seeing a tab
//! clears its bells and the "done" it showed.

use std::time::Duration;

use quadra::{Signal, effect, untrack};
use quadra_design::{Notice, Tone};
use quadra_term::vt::Progress;
use quadra_term::{ProgramState, Session};

use crate::model::{Model, Tab};

/// How long a notification's card stays.
const NOTICE: Duration = Duration::from_secs(6);

impl Tab {
    /// The strongest state of its spaces' programs.
    pub fn status(&self) -> ProgramState {
        self.spaces
            .with(|spaces| spaces.iter().map(|space| space.session.status().get()).max().unwrap_or_default())
    }

    /// The progress of the first space that reports one.
    pub fn progress(&self) -> Option<Progress> {
        self.spaces.with(|spaces| spaces.iter().find_map(|space| space.session.progress().get()))
    }

    /// The bells of its spaces not looked at yet.
    pub fn bells(&self) -> u32 {
        self.spaces.with(|spaces| spaces.iter().map(|space| space.session.bells().get()).sum())
    }
}

/// The word a state is said with, none for idle.
pub fn state_word(state: ProgramState) -> Option<&'static str> {
    match state {
        ProgramState::Idle => None,
        ProgramState::Done => Some("done"),
        ProgramState::Working => Some("working"),
        ProgramState::Error => Some("error"),
        ProgramState::Blocked => Some("blocked"),
    }
}

/// The part done of a progress, in percent, or none when it has no part (indeterminate).
pub fn percent(progress: Progress) -> Option<u8> {
    match progress {
        Progress::Set(n) | Progress::Error(n) | Progress::Paused(n) => Some(n.min(100)),
        _ => None,
    }
}

impl Model {
    /// Shows a tab: what it signalled is seen.
    pub fn show(&self, id: u64) {
        self.active().set(id);
        let Some(tab) = self.active_tab() else { return };
        for space in tab.spaces.get_untracked() {
            seen(&space.session);
        }
    }

    /// Follows what a new space's program signals: a bell in the space that has the focus is
    /// seen at once, a notification and a program that waits or failed raise a card.
    pub(crate) fn watch(&self, id: u64, session: &Session) {
        let (bells, status) = (session.bells(), session.status());
        let looked_at = self.clone();
        effect(move || {
            if bells.get() > 0 && looked_at.looks_at(id) {
                bells.set(0);
            }
        });
        let raises = self.clone();
        let was: std::cell::Cell<ProgramState> = Default::default();
        effect(move || {
            let state = status.get();
            if was.replace(state) != state && matches!(state, ProgramState::Blocked | ProgramState::Error) {
                let tone = if state == ProgramState::Error { Tone::Danger } else { Tone::Warning };
                let title = format!("{} {}", raises.tab_name(id), state_word(state).unwrap_or(""));
                raises.notify(Notice::new(title).tone(tone));
            }
        });
        let tells = self.clone();
        session.on_notification(move |n| {
            let words = if n.title.is_empty() { n.body.clone() } else { format!("{}: {}", n.title, n.body) };
            let card = Notice::new("OSC 9").body(format!("{} \u{B7} now\n{words}", tells.tab_name(id)));
            tells.notify(card.lasts(NOTICE));
        });
    }

    /// Whether a space is the focused one of the tab that shows.
    fn looks_at(&self, id: u64) -> bool {
        self.active_tab().and_then(|tab| tab.focused()).is_some_and(|space| space.id == id)
    }

    /// The title of the tab that holds a space.
    fn tab_name(&self, id: u64) -> String {
        let tabs = self.tabs().get_untracked();
        let tab =
            tabs.iter().find(|tab| tab.spaces.with_untracked(|spaces| spaces.iter().any(|it| it.id == id)));
        tab.map(|tab| untrack(|| tab.title())).unwrap_or_default()
    }

    fn notify(&self, notice: Notice) {
        self.notices().notify(notice);
        self.unread().update(|n| *n += 1);
    }
}

/// What a space signalled is seen: its bells, and a "done".
pub(crate) fn seen(session: &Session) {
    session.bells().set(0);
    if session.status().get_untracked() == ProgramState::Done {
        session.status().set(ProgramState::Idle);
    }
}

/// The count of what is not seen yet, for the status bar: bells everywhere.
pub fn all_bells(tabs: Signal<Vec<Tab>>) -> u32 {
    tabs.with(|tabs| tabs.iter().map(Tab::bells).sum())
}
