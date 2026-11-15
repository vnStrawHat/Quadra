//! 1Term in a real window:
//!
//!     tools/cargo.sh run -p oneterm

#![windows_subsystem = "windows"]

use std::rc::Rc;

use oneterm::{NewSession, OneTerm, keymap, local_shell, theme};
use quadra::{AppDirs, WindowOptions};

fn main() -> quadra::Result<()> {
    let mut app = quadra::app()?;
    if let Some(dirs) = AppDirs::for_app("1Term") {
        app = app.dirs(dirs);
    }
    // The theme the settings name, the system's by default.
    let chosen = app.settings().value("appearance.theme", "System".to_string());
    let first = quadra_design::system_theme(&app.appearance());
    let app = quadra_design::install(app, &first)?.theme_by(move |seen| theme(&chosen.get(), seen));
    let options =
        WindowOptions::new("1Term").size(1440.0, 900.0).fit(0.85).centered().custom_frame().remember("main");
    let shells: NewSession = Rc::new(local_shell);
    app.keymap(keymap()).window(options, move || OneTerm::new(shells).view())?.run()
}
