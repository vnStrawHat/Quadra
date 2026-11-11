//! Notifications and toasts (specs/behaviour/15, "Notifications"): cards stacked at the
//! bottom right of the window that stay until closed, and short lines at the bottom centre
//! that go by themselves. The app holds a [`Notices`], tells it what to show, and mounts its
//! [`Notices::host`] once in the window.

use std::rc::Rc;
use std::time::Duration;

use quadra_async::timeout;
use quadra_layout::{Align, Justify};
use quadra_reactive::{Scope, Signal, signal};
use quadra_tree::Role;
use quadra_ui::{Div, Place, View, column, div, each, overlay, row, show, text};

use crate::button::{Size, Variant, button, icon_button};
use crate::display::Tone;
use crate::icon::icon;
use crate::popup::surface;
use crate::{icons, size, token};

/// How long a toast shows.
const TOAST: Duration = Duration::from_secs(10);

type Act = Rc<dyn Fn()>;

/// A card of the stack of notifications.
#[derive(Clone)]
pub struct Notice {
    id: u64,
    title: String,
    body: String,
    tone: Tone,
    /// What its buttons say, whether each is the one to take, and what it does.
    actions: Vec<(String, bool, Act)>,
    lasts: Option<Duration>,
    toast: bool,
}

impl Notice {
    pub fn new(title: impl Into<String>) -> Notice {
        let (title, body, tone) = (title.into(), String::new(), Tone::Info);
        Notice { id: 0, title, body, tone, actions: Vec::new(), lasts: None, toast: false }
    }

    /// A line or two under the title.
    pub fn body(mut self, words: impl Into<String>) -> Self {
        self.body = words.into();
        self
    }

    /// What it is about: the icon and its colour.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// A button under the text; a click on it does `f` and closes the card.
    pub fn action(mut self, label: impl Into<String>, f: impl Fn() + 'static) -> Self {
        self.actions.push((label.into(), false, Rc::new(f)));
        self
    }

    /// As [`Notice::action`], for the button to take: it is tinted.
    pub fn primary(mut self, label: impl Into<String>, f: impl Fn() + 'static) -> Self {
        self.actions.push((label.into(), true, Rc::new(f)));
        self
    }

    /// Closes the card by itself after so long; without it the card stays until closed.
    pub fn lasts(mut self, time: Duration) -> Self {
        self.lasts = Some(time);
        self
    }
}

/// The icon of a tone, in its colour.
fn mark(tone: Tone) -> impl View {
    let (body, role) = match tone {
        Tone::Neutral => (icons::BELL, token::TEXT_2),
        Tone::Primary => (icons::INFO, token::ACCENT_TEXT),
        Tone::Info => (icons::INFO, token::INFO),
        Tone::Success => (icons::CIRCLE_CHECK, token::OK),
        Tone::Warning => (icons::TRIANGLE_ALERT, token::WARN),
        Tone::Danger => (icons::CIRCLE_X, token::ERROR),
    };
    icon(body).size(16.0).color(role)
}

/// What a window is told to show, and what it shows. Copies are the same one.
#[derive(Clone, Copy)]
pub struct Notices {
    shown: Signal<Vec<Notice>>,
    last: Signal<u64>,
    owner: Scope,
    /// How far above the bottom of the window the stacks stand: the height of a status bar.
    lift: f32,
}

/// The notifications of a window: make it where the window's view is built.
pub fn notices() -> Notices {
    Notices { shown: signal(Vec::new()), last: signal(0), owner: Scope::current(), lift: 0.0 }
}

impl Notices {
    /// Lifts the stacks over what is at the bottom of the window: 30 for a status bar.
    pub fn above(mut self, px: f32) -> Self {
        self.lift = px;
        self
    }

    /// Shows a card, under those that show already, and returns what [`Notices::close`]
    /// takes.
    pub fn notify(&self, mut notice: Notice) -> u64 {
        let (id, me) = (self.last.get_untracked() + 1, *self);
        self.last.set(id);
        notice.id = id;
        if let Some(time) = notice.lasts.filter(|_| self.owner.is_alive()) {
            self.owner.run(|| timeout(time, move || me.close(id)));
        }
        self.shown.update(|all| all.push(notice));
        id
    }

    /// Shows a line at the bottom centre for ten seconds.
    pub fn toast(&self, tone: Tone, words: impl Into<String>) -> u64 {
        self.notify(Notice { toast: true, ..Notice::new(words).tone(tone).lasts(TOAST) })
    }

    pub fn close(&self, id: u64) {
        if self.shown.with_untracked(|all| all.iter().any(|notice| notice.id == id)) {
            self.shown.update(|all| all.retain(|notice| notice.id != id));
        }
    }

    fn find(&self, id: u64) -> Option<Notice> {
        self.shown.with_untracked(|all| all.iter().find(|notice| notice.id == id).cloned())
    }

    /// The ids of the cards, or of the toasts, the oldest first.
    fn ids(&self, toasts: bool) -> Vec<u64> {
        self.shown.with(|all| all.iter().filter(|notice| notice.toast == toasts).map(|n| n.id).collect())
    }

    /// A card: 448 px on the panel's plane at the level of dialogs.
    fn card(&self, id: u64) -> Div {
        let (me, Some(notice)) = (*self, self.find(id)) else { return div() };
        let words = column().grow(1.0).child(text(notice.title.clone()));
        let words = match notice.body.is_empty() {
            true => words,
            false => words.child(div().text_color(token::TEXT_2).child(text(notice.body))),
        };
        let close = icon_button(icons::X, "Close").on_click(move || me.close(id));
        let top =
            row().gap(8.0).child(div().padding_top(3.0).child(mark(notice.tone))).child(words).child(close);
        let card = surface(2).bg(token::PANEL).width(448.0).padding(12.0).gap(8.0);
        let card = card.role(Role::Group).label(notice.title).child(top);
        if notice.actions.is_empty() {
            return card;
        }
        let mut foot = row().gap(4.0).justify(Justify::End);
        for (label, primary, act) in notice.actions {
            let variant = if primary { Variant::Primary } else { Variant::Secondary };
            let press = move || {
                act();
                me.close(id);
            };
            foot = foot.child(button(label).variant(variant).size(Size::Sm).on_click(press));
        }
        card.child(foot)
    }

    /// A toast: one line with its icon.
    fn line(&self, id: u64) -> Div {
        let Some(notice) = self.find(id) else { return div() };
        let line = row().align_items(Align::Center).gap(8.0).padding_x(10.0).padding_y(6.0);
        let line = line.child(mark(notice.tone)).child(text(notice.title.clone()));
        surface(2).role(Role::Group).label(notice.title).child(line).radius(size::RADIUS_XL)
    }

    /// What shows the notifications: mount it once, anywhere in the window. It takes no room.
    pub fn host(&self) -> impl View + use<> {
        let (me, lift) = (*self, self.lift);
        let cards = move || {
            let stack = each(move || me.ids(false), |id| *id, move |id| me.card(id.get_untracked()));
            overlay(Place::BottomRight(12.0, 12.0 + lift), stack.container(column().gap(8.0)))
        };
        let toasts = move || {
            let stack = each(move || me.ids(true), |id| *id, move |id| me.line(id.get_untracked()));
            let stack = stack.container(column().gap(8.0).align_items(Align::Center));
            overlay(Place::Bottom(12.0 + lift), stack)
        };
        let none = || div().display(quadra_layout::Display::None);
        let cards = show(move || !me.ids(false).is_empty(), cards).container(none());
        let toasts = show(move || !me.ids(true).is_empty(), toasts).container(none());
        none().child(cards).child(toasts)
    }
}
