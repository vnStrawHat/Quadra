//! Markdown's settings (specs/21, "Settings"): each a signal of the app's `Settings`, saved
//! to `settings.json` as it changes; and the window's state kept beside them.

use std::rc::Rc;

use quadra::{Appearance, Settings, Signal, Theme, use_context};

use crate::model::{PREVIEW, RAW, SIDE_BY_SIDE};

pub const THEMES: [&str; 3] = ["System", "Dark", "Light"];
pub const WIDTHS: [&str; 4] = ["Narrow", "Normal", "Wide", "Full"];
/// The layouts as the setting names them, and the layout each is.
pub const OPENS: [&str; 3] = ["Preview", "Side by side", "Raw"];
const OPENED: [usize; 3] = [PREVIEW, SIDE_BY_SIDE, RAW];

/// The settings, as signals: writing one saves it.
#[derive(Clone, Copy)]
pub struct Prefs {
    pub theme: Signal<usize>,
    /// The editor's text, in pixels.
    pub text_size: Signal<f64>,
    pub wrap: Signal<bool>,
    /// The preview's text column, one of `WIDTHS`.
    pub width: Signal<usize>,
    /// The layout a file opens in, one of `OPENS`.
    pub opens_in: Signal<usize>,
    /// Side by side, the panes follow each other.
    pub together: Signal<bool>,
    /// The window's state: the outline shows, and the editor's share of the width.
    pub outline: Signal<bool>,
    pub split: Signal<f32>,
}

impl Prefs {
    /// The app's settings, from the context; in memory where there are none (a test).
    pub fn load() -> Prefs {
        Prefs::of(&use_context::<Rc<Settings>>().unwrap_or_else(Settings::memory))
    }

    /// What each setting is where nothing was written: what a row's reset puts back.
    pub fn defaults() -> Prefs {
        Prefs::of(&Settings::memory())
    }

    pub fn of(s: &Settings) -> Prefs {
        Prefs {
            theme: s.choice("appearance.theme", &THEMES),
            text_size: s.value("editor.text_size", 14.0),
            wrap: s.value("editor.wrap", true),
            width: s.choice("preview.width", &WIDTHS),
            opens_in: s.choice("layout.opens_in", &OPENS),
            together: s.value("layout.scroll_together", true),
            outline: s.value("window.outline", true),
            split: s.value("window.split", 0.5),
        }
    }

    /// The layout a file opens in.
    pub fn opening(&self) -> usize {
        OPENED[self.opens_in.get_untracked().min(OPENED.len() - 1)]
    }

    /// The preview's text column at most, in pixels; `None`: the pane's width.
    pub fn column(&self) -> Option<f32> {
        [Some(640.0), Some(760.0), Some(960.0), None][self.width.get().min(3)]
    }

    /// The editor's text and line, in whole pixels: the line 11/7 of the text, 22 for 14.
    pub fn editor_font(&self) -> (f32, f32) {
        let size = self.text_size.get().clamp(12.0, 20.0).round() as f32;
        (size, (size * 11.0 / 7.0).round())
    }
}

/// The theme a choice of `THEMES` gives under the system's appearance.
pub fn theme(chosen: usize, appearance: &Appearance) -> Theme {
    match chosen {
        1 => quadra_design::dark(),
        2 => quadra_design::light(),
        _ => quadra_design::system_theme(appearance),
    }
    .theme()
}
