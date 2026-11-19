//! Markdown (specs/21): an app that opens `.md` files to read them, and edits them.

mod bar;
mod model;
mod panes;

use std::path::PathBuf;

use quadra::{Action, Keymap, View, effect, use_context};

pub use model::{Model, PREVIEW, RAW, SIDE_BY_SIDE};

/// Shows a layout: Raw, Side by side or Preview (Ctrl+1, 2, 3).
#[derive(Clone, Copy, Debug)]
pub struct ShowLayout(pub usize);

impl Action for ShowLayout {
    const NAME: &'static str = "markdown::ShowLayout";
}

/// The app's keys, the kit's and the editor's.
pub fn keymap() -> Keymap {
    let mut map = quadra_design::keymap();
    quadra_code::bind_keys(&mut map);
    for (keys, layout) in [("ctrl-1", RAW), ("ctrl-2", SIDE_BY_SIDE), ("ctrl-3", PREVIEW)] {
        map.bind(keys, ShowLayout(layout)).expect("the app's bindings are well formed");
    }
    map
}

/// What the window shows.
pub struct MdApp {
    pub model: Model,
}

impl MdApp {
    /// The window's model: the file given, opened in Preview, or an untitled document. Made
    /// where the window's view is built.
    pub fn new(file: Option<PathBuf>) -> MdApp {
        let model = Model::new();
        if let Some(path) = file {
            model.open(&path);
        }
        MdApp { model }
    }

    pub fn view(&self) -> impl View + use<> {
        let m = self.model.clone();
        // The window's title is the file's name.
        if let Some(window) = use_context::<quadra::Window>() {
            let named = m.clone();
            effect(move || window.set_title(format!("{} - Markdown", named.name())));
        }
        let layout = m.layout;
        quadra_design::root()
            .on_action::<ShowLayout>(move |&ShowLayout(chosen)| layout.set(chosen))
            .child(bar::bar(&m))
            .child(panes::body(&m))
            .child(m.notices.host())
    }
}
