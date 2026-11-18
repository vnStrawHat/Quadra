//! Desktop notifications (specs/18, "Signals an app reads"): what a program wants the user
//! told, by OSC 9, 777 or 99, and what an OSC 99 program asks of the ones shown.

use std::sync::Arc;

use oneterm_vt::{
    EventBatch, NotificationExpiry, NotificationOccasion, NotificationRef, NotificationUrgency,
};

use crate::session::Session;

/// A notification: its words, its buttons, and what the program asks of it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Notice {
    /// The id the program gave it (OSC 99 `i`), empty without; reports name it.
    pub id: String,
    pub title: String,
    pub body: String,
    /// The labels of its buttons, the first first.
    pub buttons: Vec<String>,
    pub urgency: Option<NotificationUrgency>,
    pub expiry: NotificationExpiry,
    /// When it wants to show: always, or only while its window lacks the keyboard or is unseen.
    pub occasion: NotificationOccasion,
    /// The sound it names (`s`); empty when it named none.
    pub sound: String,
    /// The names of icons to try, the first first (`error`, `info`, an application's).
    pub icon_names: Vec<String>,
    /// The id its icon's data is kept under (`g`), and that data as sent (a PNG, JPEG or GIF
    /// file), empty without: with no data, the data kept under the id earlier.
    pub icon_id: String,
    pub icon: Arc<[u8]>,
    /// Opening it shows where it came from.
    pub focus: bool,
    /// The program wants to hear it opened or a button pressed (`Activated`, `Button`).
    pub report_activation: bool,
    /// The program wants to hear it closed (`Closed`).
    pub report_close: bool,
}

impl Notice {
    /// A notice of words alone, as OSC 9 sends.
    pub fn text(title: impl Into<String>, body: impl Into<String>) -> Notice {
        Notice { title: title.into(), body: body.into(), focus: true, ..Notice::default() }
    }

    pub(crate) fn of(batch: &EventBatch, at: NotificationRef) -> Notice {
        let said = batch.notification(at);
        Notice {
            id: batch.str(said.id).into(),
            title: batch.str(said.title).into(),
            body: batch.str(said.body).into(),
            buttons: batch.strs(said.buttons).map(str::to_string).collect(),
            urgency: said.urgency,
            expiry: said.expiry,
            occasion: said.occasion,
            sound: batch.str(said.sound).into(),
            icon_names: batch.strs(said.icon_names).map(str::to_string).collect(),
            icon_id: batch.str(said.icon_id).into(),
            icon: batch.bytes(said.icon).into(),
            focus: said.focus_on_activation,
            report_activation: said.report_activation,
            report_close: said.report_close,
        }
    }
}

/// What a program says of notifications.
pub(crate) enum Told {
    Shown(Notice),
    /// Close the one of this id.
    Close(String),
    /// Which still show? The id to answer with.
    Alive(String),
}

impl Session {
    /// Runs `f` with each notification.
    pub fn on_notification(&self, f: impl Fn(&Notice) + 'static) {
        self.on_told(move |told| {
            if let Told::Shown(notice) = told {
                f(notice)
            }
        });
    }

    /// Runs `f` with the id of each notification the program asks to close.
    pub fn on_notice_close(&self, f: impl Fn(&str) + 'static) {
        self.on_told(move |told| {
            if let Told::Close(id) = told {
                f(id)
            }
        });
    }

    /// Runs `f` with the id of each question of which notifications still show; the answer is
    /// `vt::input::NotificationReport::Alive`, written to the program.
    pub fn on_notice_alive(&self, f: impl Fn(&str) + 'static) {
        self.on_told(move |told| {
            if let Told::Alive(id) = told {
                f(id)
            }
        });
    }
}
