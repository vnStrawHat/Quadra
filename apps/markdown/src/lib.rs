//! Markdown (specs/21): an app that opens `.md` files to read them, and edits them.

mod bar;
mod files;
mod format;
mod logo;
mod model;
mod outline;
mod panes;
mod prefs;
mod settings;
mod status;
mod sync;
mod tabs;
mod toolbar;
mod view;

use std::path::PathBuf;

use quadra::{Action, Inbox, Keymap, View};
use quadra_shell::Launch;

pub use files::{CloseTab, NewFile, NextTab, OpenFile, PreviousTab, Save, SaveAs};
pub use format::{Change, Format, format};
pub use logo::{LOGO, LOGO_SMALL, logo};
pub use model::{Model, PREVIEW, RAW, SIDE_BY_SIDE};
pub use prefs::{Prefs, theme};
pub use settings::{file_types, page as settings_page};
pub use sync::Driver;
pub use tabs::Tabs;
pub use toolbar::ApplyFormat;

/// Shows a layout: Raw, Side by side or Preview (Ctrl+1, 2, 3).
#[derive(Clone, Copy, Debug)]
pub struct ShowLayout(pub usize);

impl Action for ShowLayout {
    const NAME: &'static str = "markdown::ShowLayout";
}

/// Opens the settings (Ctrl+,).
#[derive(Clone, Copy, Debug)]
pub struct OpenSettings;

impl Action for OpenSettings {
    const NAME: &'static str = "markdown::OpenSettings";
}

/// Shows or hides the outline (Ctrl+Shift+O).
#[derive(Clone, Copy, Debug)]
pub struct ToggleOutline;

impl Action for ToggleOutline {
    const NAME: &'static str = "markdown::ToggleOutline";
}

/// The code face without its ligatures, in every window: the source reads as it is typed
/// (`##`, `|---|` and `->` are not drawn as one sign). Called before text is shown.
pub fn plain_code(app: &mut quadra::App) {
    let off = [quadra::Feature::off(b"calt"), quadra::Feature::off(b"liga")];
    app.font_features(quadra_design::fonts::MONO, &off);
}

/// The app's keys, the kit's and the editor's.
pub fn keymap() -> Keymap {
    let mut map = quadra_design::keymap();
    quadra_code::bind_keys(&mut map);
    let well_formed = "the app's bindings are well formed";
    for (keys, layout) in [("ctrl-1", RAW), ("ctrl-2", SIDE_BY_SIDE), ("ctrl-3", PREVIEW)] {
        map.bind(keys, ShowLayout(layout)).expect(well_formed);
    }
    map.bind("ctrl-n", NewFile).expect(well_formed);
    map.bind("ctrl-t", NewFile).expect(well_formed);
    map.bind("ctrl-w", CloseTab).expect(well_formed);
    map.bind("ctrl-tab", NextTab).expect(well_formed);
    map.bind("ctrl-shift-tab", PreviousTab).expect(well_formed);
    map.bind("ctrl-o", OpenFile).expect(well_formed);
    map.bind("ctrl-s", Save).expect(well_formed);
    map.bind("ctrl-shift-s", SaveAs).expect(well_formed);
    map.bind("ctrl-shift-o", ToggleOutline).expect(well_formed);
    map.bind("ctrl-,", OpenSettings).expect(well_formed);
    for (keys, what) in toolbar::keys() {
        map.bind(&keys, ApplyFormat(what)).expect(well_formed);
    }
    map
}

/// What the window shows: its tabs.
pub struct MdApp {
    /// The first tab's document.
    pub model: Model,
    pub tabs: Tabs,
    launches: Option<Inbox<Launch>>,
}

impl MdApp {
    /// The window's tabs: the file given, opened in Preview, or an untitled document. Made
    /// where the window's view is built.
    pub fn new(file: Option<PathBuf>) -> MdApp {
        MdApp::with_files(file.into_iter().collect())
    }

    /// A tab for each file, the first showing; an untitled one without any.
    pub fn with_files(files: Vec<PathBuf>) -> MdApp {
        let model = Model::new();
        let tabs = Tabs::new(model.clone());
        for path in &files {
            tabs.open(path);
        }
        if let Some(first) = tabs.order().first() {
            tabs.active.set(first.clone());
        }
        MdApp { model, tabs, launches: None }
    }

    /// Later starts of the app hand their files here (`quadra_shell::single_instance`).
    pub fn listen(mut self, launches: Inbox<Launch>) -> MdApp {
        self.launches = Some(launches);
        self
    }

    pub fn view(self) -> impl View + use<> {
        view::window(&self.tabs, self.launches)
    }
}
