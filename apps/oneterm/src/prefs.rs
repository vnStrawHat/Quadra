//! 1Term's settings (specs/19, "Settings"): each a signal of the app's `Settings`, saved to
//! `settings.json` as it changes (specs/15). A choice is kept as its words, so the file
//! reads as the page does.

use std::rc::Rc;

use quadra::{Appearance, Settings, Signal, Theme, effect, signal, untrack, use_context};

mod look;

pub use look::{ANSI, COLORS, LookPrefs, PADDING, WEIGHTS};

/// The choices of the settings that are one of a few.
pub const THEMES: [&str; 3] = ["System", "Dark", "Light"];
pub const CURSORS: [&str; 3] = ["Block", "Bar", "Underline"];
pub const STARTS: [&str; 2] = ["The active space's directory", "Home"];
pub const STATUS_SHOWN: [&str; 3] = ["On the tab and the header", "On the tab only", "Ignored"];
pub const BELLS: [&str; 2] = ["A dot and a count", "Ignored"];
pub const TITLES: [&str; 2] = ["The program's title", "Terminal, or the host"];
pub const NOTICES: [&str; 3] = ["A card for 6 s", "A card until dismissed", "Silent"];

/// The settings, as signals: writing one saves it.
#[derive(Clone, Copy)]
pub struct Prefs {
    pub theme: Signal<usize>,
    /// A colour theme's name, an index of `COLOUR_THEMES`; 0 the kit's.
    pub colour_theme: Signal<usize>,
    /// The UI's text, in percent of the kit's sizes.
    pub ui_text: Signal<f64>,
    /// How terminals look: their text, cells, room and colours.
    pub look: LookPrefs,
    pub cursor: Signal<usize>,
    pub blink: Signal<bool>,
    pub scrollback: Signal<f64>,
    /// The pointer and the wheel in a space (`TermSettings`), and its menu.
    pub context_menu: Signal<bool>,
    pub copy_on_select: Signal<bool>,
    pub middle_paste: Signal<bool>,
    pub scroll_speed: Signal<f64>,
    pub alternate_scroll: Signal<bool>,
    /// The name of the shell Ctrl+T and New terminal start; empty: the OS's; `CUSTOM`: the
    /// program typed, with its arguments.
    pub shell: Signal<String>,
    pub shell_program: Signal<String>,
    pub shell_args: Signal<String>,
    pub start: Signal<usize>,
    /// A new local shell gets `FORCE_HYPERLINK=1`.
    pub force_links: Signal<bool>,
    pub vertical: Signal<bool>,
    /// The rail's width, and whether it is folded to its icons.
    pub rail_width: Signal<f64>,
    pub rail_folded: Signal<bool>,
    /// Whether the SFTP dock is open while a local tab shows, and while an SSH tab does.
    pub dock_local: Signal<bool>,
    pub dock_ssh: Signal<bool>,
    /// The dock's width, dragged by its left edge.
    pub dock_width: Signal<f64>,
    /// What names a tab not renamed: the program's title (OSC 0/2) or a fixed label.
    pub tab_title: Signal<usize>,
    pub keepalive_on: Signal<bool>,
    /// New local shells and new SSH spaces are logged; where, and after or over a file.
    pub log_local: Signal<bool>,
    pub log_ssh: Signal<bool>,
    pub log_folder: Signal<String>,
    pub log_mode: Signal<usize>,
    pub keepalive_secs: Signal<f64>,
    pub keepalive_max: Signal<f64>,
    pub status_shown: Signal<usize>,
    pub toast_blocked: Signal<bool>,
    pub progress_tab: Signal<bool>,
    pub progress_bar: Signal<bool>,
    pub bell: Signal<usize>,
    pub notices: Signal<usize>,
    pub show_git: Signal<bool>,
    pub show_usage: Signal<bool>,
    pub show_clock: Signal<bool>,
    pub show_fps: Signal<bool>,
    /// The master password's hash; empty: none, and no lock.
    pub password: Signal<String>,
    pub lock_after: Signal<f64>,
    /// Programs may read the clipboard by OSC 52.
    pub clipboard_read: Signal<bool>,
    /// Saved hosts and recent targets, rows of words (`hosts.rs`).
    pub hosts: Signal<Vec<Vec<String>>>,
    pub recent: Signal<Vec<Vec<String>>>,
    /// The saved passwords' vault, and each host's sealed secret (`vault.rs`).
    pub vault: Signal<String>,
    pub secrets: Signal<Vec<Vec<String>>>,
    /// Each saved host's startup command: its name, the command.
    pub startup: Signal<Vec<Vec<String>>>,
    /// Where downloads go; empty: the user's Downloads folder.
    pub downloads: Signal<String>,
    /// What edits a remote file: the system's app for it, or a program with arguments; and
    /// the size, in KB, edited without asking (0: any).
    pub editor: Signal<usize>,
    pub editor_program: Signal<String>,
    pub editor_args: Signal<String>,
    pub edit_limit_kb: Signal<f64>,
}

impl Prefs {
    /// The app's settings, from the context; in memory where there are none (a test).
    pub fn load() -> Prefs {
        Prefs::of(&use_context::<Rc<Settings>>().unwrap_or_else(Settings::memory))
    }

    /// What each setting is where nothing was written: what Reset puts back.
    pub fn defaults() -> Prefs {
        Prefs::of(&Settings::memory())
    }

    fn of(s: &Settings) -> Prefs {
        Prefs {
            theme: choice(s, "appearance.theme", &THEMES),
            colour_theme: choice(s, "appearance.colour_theme", &COLOUR_THEMES),
            ui_text: s.value("appearance.ui_text_percent", 100.0),
            look: LookPrefs::of(s),
            cursor: choice(s, "terminal.cursor", &CURSORS),
            blink: s.value("terminal.cursor_blink", false),
            scrollback: s.value("terminal.scrollback", 10_000.0),
            context_menu: s.value("mouse.context_menu", true),
            copy_on_select: s.value("mouse.copy_on_select", true),
            middle_paste: s.value("mouse.middle_click_paste", true),
            scroll_speed: s.value("scroll.multiplier", 1.0),
            alternate_scroll: s.value("scroll.alternate", true),
            shell: s.value("terminal.shell", String::new()),
            shell_program: s.value("terminal.shell_program", String::new()),
            shell_args: s.value("terminal.shell_args", String::new()),
            start: choice(s, "terminal.start_in", &STARTS),
            force_links: s.value("terminal.force_hyperlink", true),
            vertical: s.value("tabs.vertical", true),
            rail_width: s.value("tabs.rail_width", 248.0),
            rail_folded: s.value("tabs.rail_folded", false),
            dock_local: s.value("sftp.dock_open_local", false),
            dock_ssh: s.value("sftp.dock_open_ssh", true),
            dock_width: s.value("sftp.dock_width", 300.0),
            tab_title: choice(s, "tabs.title", &TITLES),
            keepalive_on: s.value("ssh.keepalive", true),
            log_local: s.value("logging.local", false),
            log_ssh: s.value("logging.ssh", false),
            log_folder: s.value("logging.folder", String::new()),
            log_mode: choice(s, "logging.write_mode", &crate::logs::MODES),
            keepalive_secs: s.value("ssh.keepalive_interval_secs", 30.0),
            keepalive_max: s.value("ssh.keepalive_max", 3.0),
            status_shown: choice(s, "signals.status", &STATUS_SHOWN),
            toast_blocked: s.value("signals.status_toast", true),
            progress_tab: s.value("signals.progress_tab", true),
            progress_bar: s.value("signals.progress_status_bar", true),
            bell: choice(s, "signals.bell", &BELLS),
            notices: choice(s, "signals.notifications", &NOTICES),
            show_git: s.value("status_bar.git", true),
            show_usage: s.value("status_bar.usage", true),
            show_clock: s.value("status_bar.clock", true),
            show_fps: s.value("status_bar.fps", false),
            password: s.value("security.password", String::new()),
            lock_after: s.value("security.lock_after_minutes", 10.0),
            clipboard_read: s.value("security.allow_clipboard_read", false),
            hosts: s.value("ssh.hosts", Vec::new()),
            recent: s.value("ssh.recent", Vec::new()),
            vault: s.value("ssh.vault", String::new()),
            secrets: s.value("ssh.secrets", Vec::new()),
            startup: s.value("ssh.startup", Vec::new()),
            downloads: s.value("sftp.downloads", String::new()),
            editor: choice(s, "sftp.editor", &crate::sftp::EDITORS),
            editor_program: s.value("sftp.editor_program", String::new()),
            editor_args: s.value("sftp.editor_args", String::new()),
            edit_limit_kb: s.value("sftp.edit_max_kb", 1024.0),
        }
    }
}

impl Prefs {
    /// How a terminal looks, read where it is called.
    pub fn look(&self) -> quadra_term::TermLook {
        self.look.look()
    }

    /// The SSH keepalive a new connection asks for.
    pub fn keepalive(&self) -> (Option<std::time::Duration>, usize) {
        let every =
            std::time::Duration::from_secs(self.keepalive_secs.get_untracked().clamp(5.0, 3600.0) as u64);
        let on = self.keepalive_on.get_untracked();
        (on.then_some(every), self.keepalive_max.get_untracked().clamp(1.0, 20.0) as usize)
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

/// The theme a choice of `THEMES` (as its words) gives under the system's appearance; a
/// colour theme named (one of `crate::themes::THEMES`) wins over it.
pub fn theme(words: &str, named: &str, appearance: &Appearance) -> Theme {
    let file = crate::themes::THEMES.iter().find(|(name, _)| *name == named);
    if let Some(theme) = file.and_then(|(_, text)| quadra_design::theme_from_json(text).ok()) {
        return theme.theme();
    }
    match THEMES.iter().position(|it| *it == words) {
        Some(1) => quadra_design::dark(),
        Some(2) => quadra_design::light(),
        _ => quadra_design::system_theme(appearance),
    }
    .theme()
}

/// The colour themes as a setting offers them: the kit's first.
pub static COLOUR_THEMES: std::sync::LazyLock<Vec<&'static str>> = std::sync::LazyLock::new(|| {
    std::iter::once(KIT_THEME).chain(crate::themes::THEMES.iter().map(|(name, _)| *name)).collect()
});
/// What stands for no colour theme: the kit's, as the mode says.
pub const KIT_THEME: &str = "The kit's";

/// A setting that is one of `options`, kept as its words and given as its index: the two
/// follow each other. Words the app does not know are the first option.
pub(crate) fn choice(settings: &Settings, key: &str, options: &'static [&'static str]) -> Signal<usize> {
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

#[cfg(test)]
#[test]
fn every_colour_theme_reads() {
    for (name, text) in crate::themes::THEMES {
        let theme = quadra_design::theme_from_json(text).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_ne!(theme.theme().color(quadra_design::token::CANVAS), quadra::Color::TRANSPARENT, "{name}");
    }
    assert_eq!(COLOUR_THEMES.len(), crate::themes::THEMES.len() + 1);
}
