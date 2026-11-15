//! 1Term's settings (specs/19, "Settings"): each a signal of the app's `Settings`, saved to
//! `settings.json` as it changes (specs/15). A choice is kept as its words, so the file
//! reads as the page does.

use std::rc::Rc;

use quadra::{Appearance, Settings, Signal, Theme, effect, signal, untrack, use_context};

/// The choices of the settings that are one of a few.
pub const THEMES: [&str; 3] = ["System", "Dark", "Light"];
pub const CURSORS: [&str; 3] = ["Block", "Bar", "Underline"];
pub const STARTS: [&str; 2] = ["The active space's directory", "Home"];
pub const STATUS_SHOWN: [&str; 3] = ["On the tab and the header", "On the tab only", "Ignored"];
pub const BELLS: [&str; 2] = ["A dot and a count", "Ignored"];
pub const NOTICES: [&str; 3] = ["A card for 6 s", "A card until dismissed", "Silent"];

/// The settings, as signals: writing one saves it.
#[derive(Clone, Copy)]
pub struct Prefs {
    pub theme: Signal<usize>,
    pub font_size: Signal<f64>,
    pub cursor: Signal<usize>,
    pub blink: Signal<bool>,
    pub scrollback: Signal<f64>,
    /// The name of the shell Ctrl+T and New terminal start; empty: the OS's.
    pub shell: Signal<String>,
    pub start: Signal<usize>,
    pub vertical: Signal<bool>,
    pub status_shown: Signal<usize>,
    pub toast_blocked: Signal<bool>,
    pub progress_tab: Signal<bool>,
    pub progress_bar: Signal<bool>,
    pub bell: Signal<usize>,
    pub notices: Signal<usize>,
    pub show_git: Signal<bool>,
    pub show_usage: Signal<bool>,
    pub show_clock: Signal<bool>,
    /// The master password's hash; empty: none, and no lock.
    pub password: Signal<String>,
    pub lock_after: Signal<f64>,
    /// Saved hosts and recent targets, rows of words (`hosts.rs`).
    pub hosts: Signal<Vec<Vec<String>>>,
    pub recent: Signal<Vec<Vec<String>>>,
}

impl Prefs {
    /// The app's settings, from the context; in memory where there are none (a test).
    pub fn load() -> Prefs {
        let settings = use_context::<Rc<Settings>>().unwrap_or_else(Settings::memory);
        let s = &*settings;
        Prefs {
            theme: choice(s, "appearance.theme", &THEMES),
            font_size: s.value("terminal.font_size", 13.0),
            cursor: choice(s, "terminal.cursor", &CURSORS),
            blink: s.value("terminal.cursor_blink", false),
            scrollback: s.value("terminal.scrollback", 10_000.0),
            shell: s.value("terminal.shell", String::new()),
            start: choice(s, "terminal.start_in", &STARTS),
            vertical: s.value("tabs.vertical", true),
            status_shown: choice(s, "signals.status", &STATUS_SHOWN),
            toast_blocked: s.value("signals.status_toast", true),
            progress_tab: s.value("signals.progress_tab", true),
            progress_bar: s.value("signals.progress_status_bar", true),
            bell: choice(s, "signals.bell", &BELLS),
            notices: choice(s, "signals.notifications", &NOTICES),
            show_git: s.value("status_bar.git", true),
            show_usage: s.value("status_bar.usage", true),
            show_clock: s.value("status_bar.clock", true),
            password: s.value("security.password", String::new()),
            lock_after: s.value("security.lock_after_minutes", 10.0),
            hosts: s.value("ssh.hosts", Vec::new()),
            recent: s.value("ssh.recent", Vec::new()),
        }
    }
}

/// The theme a choice of `THEMES` (as its words) gives under the system's appearance.
pub fn theme(words: &str, appearance: &Appearance) -> Theme {
    match THEMES.iter().position(|it| *it == words) {
        Some(1) => quadra_design::dark(),
        Some(2) => quadra_design::light(),
        _ => quadra_design::system_theme(appearance),
    }
    .theme()
}

/// A setting that is one of `options`, kept as its words and given as its index: the two
/// follow each other. Words the app does not know are the first option.
fn choice(settings: &Settings, key: &str, options: &'static [&'static str]) -> Signal<usize> {
    let words = settings.value(key, options[0].to_string());
    let at = move || options.iter().position(|o| words.with(|w| o == w)).unwrap_or(0);
    let index = signal(untrack(at));
    effect(move || {
        let now = at();
        if untrack(|| index.get()) != now {
            index.set(now);
        }
    });
    effect(move || {
        let at = index.get();
        if untrack(|| words.get()) != options[at] {
            words.set(options[at].to_string());
        }
    });
    index
}
