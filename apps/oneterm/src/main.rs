//! 1Term in a real window:
//!
//!     tools/cargo.sh run -p oneterm
//!
//! With `ONETERM_HOME=.tmp/1term` its settings and log go there instead of the user's folders.

#![windows_subsystem = "windows"]

use std::rc::Rc;

use oneterm::{NewSession, OneTerm, keymap, start, theme};
use quadra::{AppDirs, WindowOptions};

fn main() -> quadra::Result<()> {
    let mut app = quadra::app()?;
    // ONETERM_HOME puts the settings and the log in one folder (a trial run); else the user's.
    let home = std::env::var_os("ONETERM_HOME").map(|home| AppDirs::under(std::path::Path::new(&home)));
    if let Some(dirs) = home.or_else(|| AppDirs::for_app("1Term")) {
        app = app.dirs(dirs);
    }
    // The theme the settings name, the system's by default.
    let chosen = app.settings().value("appearance.theme", "System".to_string());
    let named = app.settings().value("appearance.colour_theme", String::new());
    let first = quadra_design::system_theme(&app.appearance());
    let app =
        quadra_design::install(app, &first)?.theme_by(move |seen| theme(&chosen.get(), &named.get(), seen));
    // The system's dialogs and what opens files, for SFTP's Edit.
    let app = quadra_shell::install(app);
    let options =
        WindowOptions::new("1Term").size(1440.0, 900.0).fit(0.85).centered().custom_frame().remember("main");
    let shells: NewSession = Rc::new(start);
    app.keymap(keymap()).window(options, move || OneTerm::new(shells).view())?.run()
}
