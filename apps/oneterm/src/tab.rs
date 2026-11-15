//! A tab and its spaces (specs/19, "Spaces"), and what the app's keys ask for.

use quadra::{Action, Scope, Signal};
use quadra_term::Session;

use crate::prefs::Prefs;

/// A space: a session, and the scope its signals and tasks live in.
#[derive(Clone)]
pub struct Space {
    pub id: u64,
    pub session: Session,
    pub(crate) scope: Scope,
    /// The host it connected to, if it is remote.
    pub remote: Option<crate::shells::Remote>,
}

impl PartialEq for Space {
    fn eq(&self, other: &Space) -> bool {
        self.id == other.id
    }
}

/// A tab: its spaces, the one with the focus, and the name the user gave it.
#[derive(Clone, Copy)]
pub struct Tab {
    pub id: u64,
    pub spaces: Signal<Vec<Space>>,
    pub focus: Signal<usize>,
    pub name: Signal<Option<String>>,
    /// The settings, for how its signals show.
    pub(crate) prefs: Prefs,
}

impl PartialEq for Tab {
    fn eq(&self, other: &Tab) -> bool {
        self.id == other.id
    }
}

impl Tab {
    /// The space with the focus.
    pub fn focused(&self) -> Option<Space> {
        let at = self.focus.get();
        self.spaces.with(|spaces| spaces.get(at.min(spaces.len().saturating_sub(1))).cloned())
    }

    /// What the tab is called: the user's name, else its focused program's title.
    pub fn title(&self) -> String {
        if let Some(name) = self.name.get() {
            return name;
        }
        let title = self.focused().map(|space| space.session.title().get()).unwrap_or_default();
        if title.is_empty() { "Terminal".into() } else { title }
    }
}

/// What the keys of the app ask for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    NewTab,
    Split,
    CloseSpace,
    /// The space to the right, or to the left.
    FocusNext(bool),
    /// Lock at once (with a master password).
    Lock,
    /// The quick connect dialog.
    QuickConnect,
}

impl Action for Command {
    const NAME: &'static str = "oneterm::Command";
}
