//! Notifications and toasts (specs/behaviour/15, "Notifications"): cards stacked at the
//! bottom right of the window that stay until closed, and short lines at the bottom centre
//! that go by themselves. The app holds a [`Notices`], tells it what to show, and mounts its
//! [`Notices::host`] once in the window.

use std::rc::Rc;
use std::time::Duration;

use quadra_headless::notices::{Queue, TOAST};
use quadra_layout::{Align, Justify};
use quadra_tree::{PointerKind, Role};
use quadra_view::{Div, Place, View, column, div, each, overlay, row, show, text};

use crate::button::{Size, Variant, button, icon_button};
use crate::display::Tone;
use crate::icon::icon;
use crate::popup::surface;
use crate::{icons, size, token};

type Act = Rc<dyn Fn()>;

/// A card of the stack of notifications.
#[derive(Clone)]
pub struct Notice {
    title: String,
    body: String,
    /// Where it came from, in 12 px under the body: `staging-db-01 · OSC 9`.
    note: String,
    tone: Tone,
    /// What its buttons say, whether each is the one to take, and what it does.
    actions: Vec<(String, bool, Act)>,
    lasts: Option<Duration>,
    toast: bool,
}

impl Notice {
    pub fn new(title: impl Into<String>) -> Notice {
        let (title, body, tone) = (title.into(), String::new(), Tone::Info);
        Notice { title, body, note: String::new(), tone, actions: Vec::new(), lasts: None, toast: false }
    }

    /// A line or two under the title.
    pub fn body(mut self, words: impl Into<String>) -> Self {
        self.body = words.into();
        self
    }

    /// A line in 12 px under the body: where it came from.
    pub fn note(mut self, words: impl Into<String>) -> Self {
        self.note = words.into();
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
/// The icon of a notice, in the colour of its tone: 16 px on a card, 14 px in a toast. One
/// that only informs has the accent on a card, and is quiet in a toast.
fn mark(tone: Tone, toast: bool) -> impl View {
    let (size, informs) = if toast { (14.0, token::TEXT_2) } else { (16.0, token::ACCENT_TEXT) };
    icon(tone.icon()).size(size).color(if tone == Tone::Info { informs } else { tone.role() })
}

/// What a window is told to show, and what it shows. Copies are the same one.
#[derive(Clone, Copy)]
pub struct Notices {
    queue: Queue<Notice>,
    /// How far above the bottom of the window the stacks stand: the height of a status bar.
    lift: f32,
    /// The cards stand this far under the top of the window, at its right, instead.
    top: Option<f32>,
    /// How far from the right edge of the window the cards stand.
    right: f32,
}

/// The notifications of a window: make it where the window's view is built.
pub fn notices() -> Notices {
    Notices { queue: Queue::new(), lift: 0.0, top: None, right: 12.0 }
}

impl Notices {
    /// Lifts the stacks over what is at the bottom of the window: 30 for a status bar.
    pub fn above(mut self, px: f32) -> Self {
        self.lift = px;
        self
    }

    /// Stands the cards at the top right, this far under the top of the window: under an
    /// app's title bar.
    pub fn below(mut self, px: f32) -> Self {
        self.top = Some(px);
        self
    }

    /// Stands the cards this far from the right edge of the window (12 unless told): clear of
    /// a panel at the right.
    pub fn right(mut self, px: f32) -> Self {
        self.right = px;
        self
    }

    /// Shows a card, under those that show already, and returns what [`Notices::close`]
    /// takes.
    pub fn notify(&self, notice: Notice) -> u64 {
        let lasts = notice.lasts;
        self.queue.push(notice, lasts)
    }

    /// Shows a line at the bottom centre for ten seconds.
    pub fn toast(&self, tone: Tone, words: impl Into<String>) -> u64 {
        self.notify(Notice { toast: true, ..Notice::new(words).tone(tone).lasts(TOAST) })
    }

    pub fn close(&self, id: u64) {
        self.queue.close(id)
    }

    fn find(&self, id: u64) -> Option<Notice> {
        self.queue.find(id)
    }

    /// The ids of the cards, or of the toasts, the oldest first.
    fn ids(&self, toasts: bool) -> Vec<u64> {
        self.queue.ids(|notice| notice.toast == toasts)
    }

    /// A card: 448 px on the panel's plane at the level of dialogs.
    fn card(&self, id: u64) -> Div {
        let (me, Some(notice)) = (*self, self.find(id)) else { return div() };
        let words = column().grow(1.0).child(text(notice.title.clone()));
        let words = match notice.body.is_empty() {
            true => words,
            false => words.child(div().text_color(token::TEXT_2).child(text(notice.body))),
        };
        let words = match notice.note.is_empty() {
            true => words,
            false => words.child(text(notice.note).size(size::TEXT_SM.size).color(token::TEXT_2)),
        };
        let close = icon_button(icons::X, "Dismiss notification").on_click(move || me.close(id));
        let top = row()
            .gap(8.0)
            .child(div().padding_top(3.0).child(mark(notice.tone, false)))
            .child(words)
            .child(close);
        let card = surface(2).width(448.0).padding(12.0).gap(8.0);
        let card = me.held(card, id).role(Role::Group).label(notice.title).child(top);
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
        let line = line.child(mark(notice.tone, true)).child(text(notice.title.clone()));
        let line = surface(2).role(Role::Group).label(notice.title).child(line);
        self.held(line, id)
    }

    /// What lasts a while waits while the pointer is on it.
    fn held(&self, face: Div, id: u64) -> Div {
        let (on, off) = (self.queue, self.queue);
        let face = face.on_pointer(PointerKind::Enter, move |_| on.hold(id, true));
        face.on_pointer(PointerKind::Leave, move |_| off.hold(id, false))
    }

    /// What shows the notifications: mount it once, anywhere in the window. It takes no room.
    pub fn host(&self) -> impl View + use<> {
        let (me, lift, top, right) = (*self, self.lift, self.top, self.right);
        let cards = move || {
            let stack = each(move || me.ids(false), |id| *id, move |id| me.card(id.get_untracked()));
            let place = match top {
                Some(down) => Place::TopRight(right, 12.0 + down),
                None => Place::BottomRight(right, 12.0 + lift),
            };
            overlay(place, stack.container(column().gap(8.0)))
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
