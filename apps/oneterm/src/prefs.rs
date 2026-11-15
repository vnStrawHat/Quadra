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
/// The colours a terminal may have over the theme's: their keys and names.
pub const COLORS: [(&str, &str); 4] = [
    ("terminal.colors.foreground", "Foreground"),
    ("terminal.colors.background", "Background"),
    ("terminal.colors.cursor", "Cursor"),
    ("terminal.colors.selection", "Selection"),
];

/// The settings, as signals: writing one saves it.
#[derive(Clone, Copy)]
pub struct Prefs {
    pub theme: Signal<usize>,
    pub font_size: Signal<f64>,
    /// A family of the system; empty, the kit's mono face.
    pub font_family: Signal<String>,
    pub line_height: Signal<f64>,
    /// Colours over the theme's, as `#rrggbb`; empty, the theme's.
    pub colors: [Signal<String>; 4],
    pub cursor: Signal<usize>,
    pub blink: Signal<bool>,
    pub scrollback: Signal<f64>,
    /// The pointer and the wheel in a space (`TermSettings`), and its menu.
    pub context_menu: Signal<bool>,
    pub copy_on_select: Signal<bool>,
    pub middle_paste: Signal<bool>,
    pub scroll_speed: Signal<f64>,
    pub alternate_scroll: Signal<bool>,
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
    /// Programs may read the clipboard by OSC 52.
    pub clipboard_read: Signal<bool>,
    /// Saved hosts and recent targets, rows of words (`hosts.rs`).
    pub hosts: Signal<Vec<Vec<String>>>,
    pub recent: Signal<Vec<Vec<String>>>,
    /// Where downloads go; empty: the user's Downloads folder.
    pub downloads: Signal<String>,
}

impl Prefs {
    /// The app's settings, from the context; in memory where there are none (a test).
    pub fn load() -> Prefs {
        let settings = use_context::<Rc<Settings>>().unwrap_or_else(Settings::memory);
        let s = &*settings;
        Prefs {
            theme: choice(s, "appearance.theme", &THEMES),
            font_size: s.value("terminal.font_size", 13.0),
            font_family: s.value("terminal.font_family", String::new()),
            line_height: s.value("terminal.line_height", 1.5),
            colors: COLORS.map(|(key, _)| s.value(key, String::new())),
            cursor: choice(s, "terminal.cursor", &CURSORS),
            blink: s.value("terminal.cursor_blink", false),
            scrollback: s.value("terminal.scrollback", 10_000.0),
            context_menu: s.value("mouse.context_menu", true),
            copy_on_select: s.value("mouse.copy_on_select", true),
            middle_paste: s.value("mouse.middle_click_paste", true),
            scroll_speed: s.value("scroll.multiplier", 1.0),
            alternate_scroll: s.value("scroll.alternate", true),
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
            clipboard_read: s.value("security.allow_clipboard_read", false),
            hosts: s.value("ssh.hosts", Vec::new()),
            recent: s.value("ssh.recent", Vec::new()),
            downloads: s.value("sftp.downloads", String::new()),
        }
    }
}

impl Prefs {
    /// How a terminal looks, read where it is called.
    pub fn look(&self) -> quadra_term::TermLook {
        let [foreground, background, cursor, selection] =
            self.colors.map(|it| it.with(|words| quadra::Color::parse_hex(words.trim())));
        quadra_term::TermLook {
            family: self.font_family.get(),
            size: self.font_size.get().clamp(6.0, 48.0) as f32,
            line_height: self.line_height.get().clamp(1.0, 2.5) as f32,
            colors: quadra_term::TermColors {
                foreground,
                background,
                cursor,
                selection,
                ..Default::default()
            },
            ..quadra_term::TermLook::default()
        }
    }

    /// What a terminal view is given of the settings, read where it is called.
    pub fn term(&self) -> quadra_term::TermSettings {
        quadra_term::TermSettings {
            copy_on_select: self.copy_on_select.get(),
            middle_paste: self.middle_paste.get(),
            scroll: self.scroll_speed.get() as f32,
            alternate_scroll: self.alternate_scroll.get(),
            clipboard_read: self.clipboard_read.get(),
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
