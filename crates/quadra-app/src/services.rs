//! What the app gives beyond windows (specs/15): its folders, settings, log and crash reports,
//! and its command line.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use quadra_platform::Appearance;
use quadra_reactive::{effect, provide};
use quadra_render::Renderer;
use quadra_tree::Theme;

use crate::i18n::I18n;
use crate::settings::Settings;
use crate::{App, crash, log};

/// Where an app keeps its files: its settings in `config`, its logs and crash reports in
/// `data`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppDirs {
    pub config: PathBuf,
    pub data: PathBuf,
}

impl AppDirs {
    /// The folders of the user's account for an app of this name: on Windows under
    /// `%APPDATA%` and `%LOCALAPPDATA%`, on macOS under `~/Library/Application Support` and
    /// `~/Library/Logs`. None where the environment does not say them.
    pub fn for_app(name: &str) -> Option<AppDirs> {
        AppDirs::from(name, cfg!(target_os = "macos"), |var| std::env::var_os(var).map(PathBuf::from))
    }

    /// The folders as a system's convention has them, of what its environment says.
    fn from(name: &str, mac: bool, said: impl Fn(&str) -> Option<PathBuf>) -> Option<AppDirs> {
        if mac {
            let library = said("HOME")?.join("Library");
            let config = library.join("Application Support").join(name);
            return Some(AppDirs { config, data: library.join("Logs").join(name) });
        }
        Some(AppDirs { config: said("APPDATA")?.join(name), data: said("LOCALAPPDATA")?.join(name) })
    }

    /// Both under one folder: tests, and an app that is carried on a stick.
    pub fn under(folder: &Path) -> AppDirs {
        AppDirs { config: folder.join("config"), data: folder.join("data") }
    }

    fn crashes(&self) -> PathBuf {
        self.data.join("crashes")
    }
}

impl<R: Renderer> App<R> {
    /// Gives the app its folders: from now on the log is written to `data/logs`, a panic
    /// writes a report to `data/crashes`, and the settings are `config/settings.json`.
    /// Without them nothing is written (specs/15, "Where files go").
    pub fn dirs(mut self, dirs: AppDirs) -> App<R> {
        log::to_folder(&dirs.data.join("logs"));
        crash::report_panics(&dirs.crashes());
        let settings = self.root.run(|| {
            // Views find where the app keeps its files.
            provide(Rc::new(dirs.clone()));
            let settings = Settings::open(dirs.config.join("settings.json"));
            provide(settings.clone());
            settings
        });
        self.settings = settings;
        self.dirs = Some(dirs);
        self
    }

    /// The app's settings, which views also find in their context. In memory until the app
    /// is given its folders.
    pub fn settings(&self) -> Rc<Settings> {
        self.settings.clone()
    }

    /// The newest crash report written since the app last asked: one to offer to send.
    pub fn last_crash(&self) -> Option<PathBuf> {
        crash::last_crash(&self.dirs.as_ref()?.crashes())
    }

    /// The app's strings and language: views find them in their context (`t`, `locale`).
    pub fn i18n(self, i18n: Rc<I18n>) -> App<R> {
        self.root.run(|| provide(i18n));
        self
    }

    /// Gives every view of the app `value` in its context: a service of the OS
    /// (`quadra-shell`), the app's own state.
    pub fn provide<T: Clone + 'static>(self, value: T) -> App<R> {
        self.root.run(|| provide(value));
        self
    }

    /// The arguments the app was started with, after the program's name.
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// Other arguments: a test's.
    pub fn with_args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> App<R> {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }

    /// Whether the app quits when its last window closes; not, for an app of the tray.
    pub fn quit_on_last_window(mut self, quits: bool) -> App<R> {
        self.quit_on_last = quits;
        self
    }
}

impl<R: Renderer> App<R> {
    /// Makes the app's theme follow the system's appearance: `theme` is asked for the one
    /// of an appearance now, and again whenever the user changes it.
    pub fn theme_by(self, theme: impl Fn(&Appearance) -> Theme + 'static) -> App<R> {
        let (appearance, themes) = (self.appearance, self.theme);
        self.root.run(|| effect(move || themes.set(Rc::new(theme(&appearance.get())))));
        self
    }

    /// The system's appearance as it is now.
    pub fn appearance(&self) -> Appearance {
        self.appearance.get_untracked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_app_s_folders_are_where_its_system_keeps_them() {
        let said = |var: &str| Some(PathBuf::from(format!("/{var}")));
        let mac = AppDirs::from("App", true, said).unwrap();
        assert_eq!(mac.config, Path::new("/HOME/Library/Application Support/App"));
        assert_eq!(mac.data, Path::new("/HOME/Library/Logs/App"));
        let win = AppDirs::from("App", false, said).unwrap();
        assert_eq!((win.config, win.data), ("/APPDATA/App".into(), "/LOCALAPPDATA/App".into()));
        assert_eq!(AppDirs::from("App", true, |_| None), None);
    }
}
