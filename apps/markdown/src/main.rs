//! Markdown in a real window:
//!
//!     tools/cargo.sh run -p markdown -- [file.md]
//!
//! With `MARKDOWN_HOME=.tmp/markdown` its settings and log go there instead of the user's
//! folders.

#![windows_subsystem = "windows"]

use std::path::PathBuf;

use markdown::{MdApp, Prefs, keymap};
use quadra::{AppDirs, WindowOptions};

fn main() -> quadra::Result<()> {
    let mut app = quadra::app()?;
    let home = std::env::var_os("MARKDOWN_HOME").map(|home| AppDirs::under(std::path::Path::new(&home)));
    let home_set = home.is_some();
    if let Some(dirs) = home.or_else(|| AppDirs::for_app("Markdown")) {
        quadra_shell::crash_reports(&dirs.data.join("crashes"));
        app = app.dirs(dirs);
    }
    // The files to open: the arguments, as the OS gives them ("Open with"). A later start
    // hands them to the first, which opens them as tabs, and ends.
    let args: Vec<String> = app.args().to_vec();
    let id = if home_set { "markdown-trial" } else { "markdown" };
    let launches = match quadra_shell::single_instance(id, &args) {
        quadra_shell::Instance::First(launches) => launches,
        quadra_shell::Instance::Second => return Ok(()),
    };
    let files: Vec<PathBuf> = args.iter().map(PathBuf::from).collect();
    // The theme the settings name, the system's by default.
    let prefs = Prefs::of(&app.settings());
    let first = quadra_design::system_theme(&app.appearance());
    let mut app =
        quadra_design::install(app, &first)?.theme_by(move |seen| markdown::theme(prefs.theme.get(), seen));
    markdown::plain_code(&mut app);
    let app = quadra_shell::install(app);
    let options = WindowOptions::new("Markdown")
        .size(1280.0, 860.0)
        .fit(0.85)
        .min_size(560.0, 400.0)
        .centered()
        .remember("main");
    app.keymap(keymap()).window(options, move || MdApp::with_files(files).listen(launches).view())?.run()
}
