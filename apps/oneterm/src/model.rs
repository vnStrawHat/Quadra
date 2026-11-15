//! Tabs and spaces (specs/19, "Rail", "Spaces"): a tab holds spaces side by side, each a
//! session. They live in the model's scope, not in the views that show them: a tab that is
//! not shown keeps its programs running, and one made by a key has a scope to live in.

use std::cell::Cell;
use std::rc::Rc;

use quadra::{Scope, Signal, signal};
use quadra_design::{Notices, notices};

use crate::NewSession;
use crate::lock::Lock;
use crate::prefs::Prefs;
use crate::shells::Launch;
pub use crate::tab::{Command, Space, Tab};

/// What is open over the window: the settings page (it takes the body), the quick connect
/// dialog, the dialog that adds a host.
#[derive(Clone, Copy)]
pub struct Ui {
    pub settings: Signal<bool>,
    pub connecting: Signal<bool>,
    pub adding: Signal<bool>,
    /// The settings' page shown.
    pub page: Signal<usize>,
    /// The saved host being edited, by name.
    pub editing: Signal<Option<String>>,
    /// The SFTP dock.
    pub dock: Signal<bool>,
    /// SFTP in the workspace's place.
    pub expanded: Signal<bool>,
}

/// The window's tabs. A copy is the same one.
#[derive(Clone)]
pub struct Model(Rc<Inner>);

struct Inner {
    owner: Scope,
    new_session: NewSession,
    next: Cell<u64>,
    tabs: Signal<Vec<Tab>>,
    active: Signal<u64>,
    /// The tab whose name is being edited in place.
    renaming: Signal<Option<u64>>,
    /// Tabs in the rail, or in a strip under the title bar.
    prefs: Prefs,
    /// The cards of notifications, and how many came since they were last looked at.
    notices: Notices,
    unread: Signal<u32>,
    ui: Ui,
    lock: Lock,
}

impl Model {
    /// A model with one tab of one space, owned by the scope it is made in.
    pub fn new(new_session: NewSession, prefs: Prefs) -> Model {
        let inner = Inner {
            owner: Scope::current(),
            new_session,
            next: Cell::new(1),
            tabs: signal(Vec::new()),
            active: signal(0),
            renaming: signal(None),
            prefs,
            notices: notices().below(38.0 + 12.0),
            unread: signal(0),
            ui: Ui {
                settings: signal(false),
                connecting: signal(false),
                adding: signal(false),
                page: signal(0),
                editing: signal(None),
                dock: signal(false),
                expanded: signal(false),
            },
            lock: Lock::new(prefs),
        };
        let model = Model(Rc::new(inner));
        model.new_tab();
        model
    }

    pub fn tabs(&self) -> Signal<Vec<Tab>> {
        self.0.tabs
    }

    /// The id of the tab that shows.
    pub fn active(&self) -> Signal<u64> {
        self.0.active
    }

    pub fn renaming(&self) -> Signal<Option<u64>> {
        self.0.renaming
    }

    pub fn vertical(&self) -> Signal<bool> {
        self.0.prefs.vertical
    }

    pub fn prefs(&self) -> Prefs {
        self.0.prefs
    }

    pub fn notices(&self) -> Notices {
        self.0.notices
    }

    pub fn unread(&self) -> Signal<u32> {
        self.0.unread
    }

    /// What is open over the window.
    pub fn ui(&self) -> Ui {
        self.0.ui
    }

    pub fn lock(&self) -> Lock {
        self.0.lock
    }

    pub fn active_tab(&self) -> Option<Tab> {
        let active = self.0.active.get();
        self.0.tabs.with(|tabs| tabs.iter().find(|tab| tab.id == active).copied())
    }

    fn id(&self) -> u64 {
        let id = self.0.next.get();
        self.0.next.set(id + 1);
        id
    }

    /// A space of its own scope under the model's, its session made in it.
    fn space(&self, launch: Launch) -> Space {
        let scope = self.0.owner.run(Scope::child);
        let (id, remote) = (self.id(), launch.ssh.clone());
        let session = scope.run(|| {
            let session = (self.0.new_session)(launch);
            self.watch(id, &session);
            let (rows, kept) = (self.0.prefs.scrollback, session.clone());
            quadra::effect(move || kept.set_scrollback(rows.get().clamp(0.0, 1.0e6) as u32));
            session
        });
        Space { id, session, scope, remote }
    }

    /// A tab with a new local shell, after the others, shown.
    pub fn new_tab(&self) {
        self.new_tab_with(self.launch(None));
    }

    /// A tab running a launch, after the others, shown.
    pub fn new_tab_with(&self, launch: Launch) {
        self.insert_tab(usize::MAX, launch);
    }

    /// A tab with a new local shell right after a tab, shown.
    /// A tab after it starting what its focused space started: the same host, else a shell.
    pub fn duplicate(&self, id: u64) {
        let tab = self.0.tabs.with_untracked(|tabs| tabs.iter().find(|tab| tab.id == id).copied());
        let ssh = tab.and_then(|tab| quadra::untrack(|| tab.focused())).and_then(|space| space.remote);
        self.insert_tab(
            self.index_of(id).map_or(usize::MAX, |at| at + 1),
            Launch { ssh, ..self.launch(None) },
        );
    }

    fn index_of(&self, id: u64) -> Option<usize> {
        self.0.tabs.with_untracked(|tabs| tabs.iter().position(|tab| tab.id == id))
    }

    /// Moves a tab to `to` among the tabs (counted before it moves).
    pub fn move_tab(&self, id: u64, to: usize) {
        let Some(from) = self.index_of(id) else { return };
        self.0.tabs.update(|tabs| {
            let tab = tabs.remove(from);
            tabs.insert(to.min(tabs.len()), tab);
        });
    }

    /// Closes every tab but one, which shows.
    pub fn close_others(&self, id: u64) {
        let others: Vec<u64> =
            self.0.tabs.get_untracked().iter().map(|tab| tab.id).filter(|&it| it != id).collect();
        self.0.active.set(id);
        for other in others {
            self.close_tab(other);
        }
    }

    fn insert_tab(&self, at: usize, launch: Launch) {
        let space = self.space(launch);
        let tab = self.0.owner.run(|| Tab {
            id: self.id(),
            spaces: signal(vec![space]),
            focus: signal(0),
            name: signal(None),
            prefs: self.0.prefs,
        });
        self.0.tabs.update(|tabs| tabs.insert(at.min(tabs.len()), tab));
        self.0.active.set(tab.id);
    }

    /// A new space beside the focused one of the active tab, with the focus.
    pub fn split(&self) {
        self.split_with(Launch { cwd: self.focused_cwd(), ..self.launch(None) });
    }

    /// A new space running a launch beside the focused one of the active tab, with the focus.
    pub fn split_with(&self, launch: Launch) {
        let Some(tab) = self.active_tab() else { return };
        let space = self.space(launch);
        let at = tab.focus.get_untracked() + 1;
        tab.spaces.update(|spaces| spaces.insert(at.min(spaces.len()), space));
        tab.focus.set(at);
    }

    /// Closes the focused space of the active tab, and the tab with its last space. False
    /// when no tab is left.
    pub fn close_focused(&self) -> bool {
        match self.active_tab().and_then(|tab| tab.focused()) {
            Some(space) => self.close_space(space.id),
            None => false,
        }
    }

    /// Closes a space, and its tab when it was the last. False when no tab is left.
    pub fn close_space(&self, id: u64) -> bool {
        let tabs = self.0.tabs.get_untracked();
        let found = tabs.iter().find_map(|tab| {
            tab.spaces.with_untracked(|spaces| spaces.iter().position(|it| it.id == id)).map(|at| (*tab, at))
        });
        let Some((tab, at)) = found else { return !tabs.is_empty() };
        let mut gone = None;
        tab.spaces.update(|spaces| gone = Some(spaces.remove(at)));
        if let Some(space) = gone {
            end(space);
        }
        let left = tab.spaces.with_untracked(Vec::len);
        if left == 0 {
            return self.close_tab(tab.id);
        }
        let focus = tab.focus.get_untracked();
        tab.focus.set(if focus > at { focus - 1 } else { focus.min(left - 1) });
        true
    }

    /// Closes a tab and its spaces; the one after it shows, or the one before. False when
    /// no tab is left.
    pub fn close_tab(&self, id: u64) -> bool {
        let mut gone = None;
        let mut left = 0;
        self.0.tabs.update(|tabs| {
            if let Some(at) = tabs.iter().position(|tab| tab.id == id) {
                gone = Some((at, tabs.remove(at)));
            }
            left = tabs.len();
        });
        let Some((at, tab)) = gone else { return left > 0 };
        for space in tab.spaces.get_untracked() {
            end(space);
        }
        if self.0.active.get_untracked() == id
            && let Some(next) =
                self.0.tabs.with_untracked(|tabs| tabs.get(at.min(tabs.len().saturating_sub(1))).copied())
        {
            self.0.active.set(next.id);
        }
        left > 0
    }

    /// Moves the focus to the next space of the active tab, or the one before.
    pub fn focus_next(&self, forward: bool) {
        let Some(tab) = self.active_tab() else { return };
        let count = tab.spaces.with_untracked(Vec::len);
        let at = tab.focus.get_untracked();
        tab.focus.set(if forward { (at + 1).min(count - 1) } else { at.saturating_sub(1) });
    }
}

/// Ends a space: its program, then its scope.
fn end(space: Space) {
    space.session.close();
    space.scope.dispose();
}
