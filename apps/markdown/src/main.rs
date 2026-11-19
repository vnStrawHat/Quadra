//! Markdown in a real window:
//!
//!     tools/cargo.sh run -p markdown -- [file.md]
//!
//! With `MARKDOWN_HOME=.tmp/markdown` its settings and log go there instead of the user's
//! folders.

#![windows_subsystem = "windows"]

use std::path::PathBuf;

use markdown::{MdApp, Prefs, keymap};
use quadra::{AppDirs, Feature, WindowOptions};

fn main() -> quadra::Result<()> {
    let mut app = quadra::app()?;
    let home = std::env::var_os("MARKDOWN_HOME").map(|home| AppDirs::under(std::path::Path::new(&home)));
    if let Some(dirs) = home.or_else(|| AppDirs::for_app("Markdown")) {
        quadra_shell::crash_reports(&dirs.data.join("crashes"));
        app = app.dirs(dirs);
    }
    // The file to open: the first argument, as the OS gives it ("Open with").
    let file = app.args().first().map(PathBuf::from);
    // The theme the settings name, the system's by default.
    let prefs = Prefs::of(&app.settings());
    let first = quadra_design::system_theme(&app.appearance());
    let mut app =
        quadra_design::install(app, &first)?.theme_by(move |seen| markdown::theme(prefs.theme.get(), seen));
    // The source reads as it is typed: `##`, `|---|` and `->` are not drawn as one sign.
    app.font_features(quadra_design::fonts::MONO, &[Feature::off(b"calt"), Feature::off(b"liga")]);
    let app = quadra_shell::install(app);
    let options = WindowOptions::new("Markdown")
        .size(1280.0, 860.0)
        .fit(0.85)
        .min_size(560.0, 400.0)
        .centered()
        .remember("main");
    app.keymap(keymap()).window(options, move || MdApp::new(file.clone()).view())?.run()
}
