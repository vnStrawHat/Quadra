//! Markdown in a real window:
//!
//!     tools/cargo.sh run -p markdown -- [file.md]
//!
//! With `MARKDOWN_HOME=.tmp/markdown` its settings and log go there instead of the user's
//! folders.

#![windows_subsystem = "windows"]

use std::path::PathBuf;

use markdown::{MdApp, keymap};
use quadra::{AppDirs, WindowOptions};

fn main() -> quadra::Result<()> {
    let mut app = quadra::app()?;
    let home = std::env::var_os("MARKDOWN_HOME").map(|home| AppDirs::under(std::path::Path::new(&home)));
    if let Some(dirs) = home.or_else(|| AppDirs::for_app("Markdown")) {
        quadra_shell::crash_reports(&dirs.data.join("crashes"));
        app = app.dirs(dirs);
    }
    // The file to open: the first argument, as the OS gives it ("Open with").
    let file = app.args().first().map(PathBuf::from);
    let app = quadra_shell::install(quadra_design::install_system(app)?);
    let options = WindowOptions::new("Markdown")
        .size(1280.0, 860.0)
        .fit(0.85)
        .min_size(560.0, 400.0)
        .centered()
        .remember("main");
    app.keymap(keymap()).window(options, move || MdApp::new(file.clone()).view())?.run()
}
