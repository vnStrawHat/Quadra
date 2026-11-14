//! What the app gives beyond windows (specs/15): its folders, settings, log and crash reports,
//! and its command line.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use quadra_reactive::provide;
use quadra_render::Renderer;

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
    /// The folders of the user's account for an app of this name: under `%APPDATA%` and
    /// `%LOCALAPPDATA%`. None where the environment does not say them.
    pub fn for_app(name: &str) -> Option<AppDirs> {
        let under = |var: &str| std::env::var_os(var).map(|root| PathBuf::from(root).join(name));
        Some(AppDirs { config: under("APPDATA")?, data: under("LOCALAPPDATA")? })
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
