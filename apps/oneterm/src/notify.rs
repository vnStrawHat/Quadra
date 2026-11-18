//! A program's notifications as cards (specs/19, "Signals"): OSC 9 and 777 words, and OSC 99
//! in full: buttons, opening, closing and the reports a program asked for, a card that takes
//! the place of one with its id, the program's close and its question of what still shows.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use quadra_design::{Notice, Tone};
use quadra_term::vt::input::NotificationReport;
use quadra_term::vt::{NotificationExpiry, NotificationSupport, NotificationUrgency};
use quadra_term::{Notice as Told, Session};

use crate::model::Model;

/// How long a card lasts unless the setting keeps it or the program says.
const NOTICE: Duration = Duration::from_secs(6);

/// What 1Term does with an OSC 99 notification, from which the engine answers `p=?`.
pub(crate) fn abilities() -> NotificationSupport {
    NotificationSupport {
        focus: true,
        report: true,
        close_events: true,
        buttons: true,
        close: true,
        alive: true,
        urgency: true,
        expiry: true,
        ..NotificationSupport::default()
    }
}

impl Model {
    /// Shows a space's notifications as cards and answers what its program asks of them.
    pub(crate) fn follow_notices(&self, id: u64, session: &Session) {
        // The cards that show, by the id the program gave them.
        let cards: Rc<RefCell<Vec<(String, u64)>>> = Rc::default();
        let (me, shown) = (self.clone(), cards.clone());
        session.on_notification(move |n| {
            let lasts = match me.prefs().notices.get_untracked() {
                0 => Some(NOTICE),
                1 => None,
                _ => return, // silent
            };
            let card = me.card_of(id, n, lasts, &shown);
            let held = shown.borrow().iter().find(|it| !n.id.is_empty() && it.0 == n.id).map(|it| it.1);
            let card = match held.filter(|&at| me.notices().shows(at)) {
                Some(at) => Some(me.notices().replace(at, card)),
                None => me.notify_card(card),
            };
            let mut shown = shown.borrow_mut();
            shown.retain(|it| it.0 != n.id || n.id.is_empty());
            shown.extend(card.filter(|_| !n.id.is_empty()).map(|card| (n.id.clone(), card)));
        });
        let (me, shown) = (self.clone(), cards.clone());
        session.on_notice_close(move |asked| {
            let card = shown.borrow().iter().find(|it| it.0 == asked).map(|it| it.1);
            card.into_iter().for_each(|card| me.notices().close(card));
        });
        let me = self.clone();
        session.on_notice_alive(move |asked| {
            let notices = me.notices();
            let alive: Vec<String> =
                cards.borrow().iter().filter(|it| notices.shows(it.1)).map(|it| it.0.clone()).collect();
            let alive: Vec<&str> = alive.iter().map(String::as_str).collect();
            me.tell(id, NotificationReport::Alive { id: asked, alive: &alive });
        });
    }

    /// The card of a notification: its words, its buttons, what opening and closing it tell.
    fn card_of(
        &self,
        id: u64,
        n: &Told,
        lasts: Option<Duration>,
        shown: &Rc<RefCell<Vec<(String, u64)>>>,
    ) -> Notice {
        // The title, else the text in its place; then where it came from.
        let (title, words) = match n.title.is_empty() {
            true => (n.body.clone(), String::new()),
            false => (n.title.clone(), n.body.clone()),
        };
        let critical = n.urgency == Some(NotificationUrgency::Critical);
        let lasts = match n.expiry {
            _ if critical => None,
            NotificationExpiry::Never => None,
            NotificationExpiry::After(ms) => Some(Duration::from_millis(ms.into())),
            NotificationExpiry::Platform => lasts,
        };
        let mut card = Notice::new(title).body(words).note(format!("{} \u{B7} OSC 9", self.tab_name(id)));
        card = card.tone(if critical { Tone::Danger } else { Tone::Info });
        if let Some(time) = lasts {
            card = card.lasts(time);
        }
        for (at, label) in n.buttons.iter().enumerate() {
            let (me, told) = (self.clone(), n.clone());
            card = card.action(label, move || {
                if told.report_activation {
                    me.tell(id, NotificationReport::Button { id: &told.id, button: at as u32 + 1 });
                }
            });
        }
        let (me, told) = (self.clone(), n.clone());
        card = card.on_open(move || {
            if told.focus {
                me.reveal(id);
            }
            if told.report_activation {
                me.tell(id, NotificationReport::Activated { id: &told.id });
            }
        });
        let (me, told, shown) = (self.clone(), n.clone(), shown.clone());
        card.on_close(move || {
            shown.borrow_mut().retain(|it| it.0 != told.id);
            if told.report_close {
                me.tell(id, NotificationReport::Closed { id: &told.id });
            }
        })
    }

    /// Writes a report to the program of a space, while it is there.
    fn tell(&self, id: u64, report: NotificationReport) {
        let tabs = self.tabs().get_untracked();
        let spaces = tabs.iter().flat_map(|tab| tab.spaces.get_untracked());
        if let Some(space) = spaces.into_iter().find(|space| space.id == id) {
            space.session.write(&report.encode());
        }
    }

    /// Shows the tab of a space and gives the space the focus.
    fn reveal(&self, id: u64) {
        let tabs = self.tabs().get_untracked();
        for tab in tabs {
            let at = tab.spaces.with_untracked(|spaces| spaces.iter().position(|space| space.id == id));
            if let Some(at) = at {
                self.show(tab.id);
                tab.focus.set(at);
                return;
            }
        }
    }
}
