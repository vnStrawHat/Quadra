//! Markdown (specs/21): an app that opens `.md` files to read them, and edits them.

mod bar;
mod files;
mod format;
mod logo;
mod model;
mod panes;
mod sync;
mod toolbar;

use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use quadra::{Action, DroppedFiles, Keymap, View, debounce, effect, use_context};
use quadra_shell::Opener;

pub use files::{NewFile, OpenFile, Save, SaveAs};
pub use format::{Change, Format, format};
pub use logo::{LOGO, logo};
pub use model::{Model, PREVIEW, RAW, SIDE_BY_SIDE};
pub use sync::Driver;
pub use toolbar::ApplyFormat;

use files::Files;

/// The wait after the last edit before the preview shows the text.
const PREVIEW_AFTER: Duration = Duration::from_millis(120);

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
    let well_formed = "the app's bindings are well formed";
    for (keys, layout) in [("ctrl-1", RAW), ("ctrl-2", SIDE_BY_SIDE), ("ctrl-3", PREVIEW)] {
        map.bind(keys, ShowLayout(layout)).expect(well_formed);
    }
    map.bind("ctrl-n", NewFile).expect(well_formed);
    map.bind("ctrl-o", OpenFile).expect(well_formed);
    map.bind("ctrl-s", Save).expect(well_formed);
    map.bind("ctrl-shift-s", SaveAs).expect(well_formed);
    for (keys, what) in toolbar::keys() {
        map.bind(&keys, ApplyFormat(what)).expect(well_formed);
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
        let files = Files::new(&m);
        // The window's title is the file's name.
        let window = use_context::<quadra::Window>();
        if let Some(window) = window.clone() {
            let named = m.clone();
            effect(move || window.set_title(format!("{} - Markdown", named.name())));
        }
        // The preview follows the editor once it stands still.
        let (still, shown) = (debounce(m.edited, PREVIEW_AFTER), m.clone());
        effect(move || {
            still.get();
            shown.refresh();
        });
        sync::follow(&m);
        let layout = m.layout;
        let (f, s, a, o, c) = (files.clone(), files.clone(), files.clone(), files.clone(), files.clone());
        let (drops, formats) = (files.clone(), m.clone());
        quadra_design::root()
            .on_action::<ShowLayout>(move |&ShowLayout(chosen)| layout.set(chosen))
            .on_action::<ApplyFormat>(move |&ApplyFormat(what)| toolbar::apply(&formats, what))
            .on_action::<NewFile>(move |_| f.new_file())
            .on_action::<OpenFile>(move |_| o.open())
            .on_action::<Save>(move |_| s.save(|| {}))
            .on_action::<SaveAs>(move |_| a.save_as(|| {}))
            .on_drop(move |dropped: &DroppedFiles, _| {
                if let Some(path) = dropped.0.iter().find(|it| files::readable(it)) {
                    drops.open_path(path.clone());
                }
            })
            // Closing with unsaved work asks first, and closes once it is answered.
            .on_close(move || {
                let unsaved = c.unsaved();
                if unsaved && let Some(window) = window.clone() {
                    c.then_unsaved(move || window.close_now());
                }
                !unsaved
            })
            .child(bar::bar(&m, &files))
            .child(
                quadra::show(move || layout.get() != PREVIEW, {
                    let m = m.clone();
                    move || toolbar::toolbar(&m)
                })
                .container(quadra::column().shrink(0.0)),
            )
            .child(panes::body(&m, follow(&m, &files)))
            .child(m.notices.host())
    }
}

/// What a link does, Ctrl held: a heading of this document is brought into view, an address
/// opens with the OS, another document beside this one opens here.
fn follow(m: &Model, files: &Files) -> panes::Follow {
    let (m, files, opener) = (m.clone(), files.clone(), use_context::<Rc<dyn Opener>>());
    Rc::new(move |target: &str| {
        if let Some(anchor) = target.strip_prefix('#') {
            let blocks = quadra_md::parse(&m.text.get_untracked());
            let found = quadra_md::outline(&blocks).into_iter().find(|it| it.anchor == anchor);
            m.reveal.set(found.map(|it| it.line));
        } else if quadra_text::url_openable(target) {
            if let Some(opener) = &opener {
                opener.open(target);
            }
        } else if let Some(folder) = m.folder() {
            let path = folder.join(target);
            if files::readable(&path) {
                files.open_path(path);
            }
        }
    })
}
