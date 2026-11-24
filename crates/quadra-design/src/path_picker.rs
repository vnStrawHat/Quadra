//! The file and folder pickers of the kit (specs/13, `file_picker`; specs/design, "File and
//! folder picker"): a path in an input, and a button that asks the system's dialog for one.
//! The dialog is the app's (`quadra-shell`'s), given as `browse`: the kit asks no OS itself.

use std::path::PathBuf;
use std::rc::Rc;

use quadra_layout::Align;
use quadra_reactive::Signal;
use quadra_tree::NodeId;
use quadra_view::{IntoProp, MountCx, Prop, View, row};

use crate::button::{Variant, button};
use crate::input::input;
use crate::{icons, token};

/// What asks the dialog, given the path the field holds: the path chosen, or none.
type Browse = Rc<dyn Fn(&str) -> Option<PathBuf>>;

/// A path in a field and a button that browses for one; see [`file_picker`].
pub struct PathPicker {
    value: Signal<String>,
    browse: Browse,
    folder: bool,
    placeholder: String,
    label: Option<Prop<String>>,
    width: Option<f32>,
    fill: bool,
    test_id: Option<&'static str>,
}

/// A path of a file in `value`, typed or chosen by "Browse…", which runs `browse` with the
/// path the field holds: the app's open-file dialog, which answers the file chosen or `None`.
pub fn file_picker(value: Signal<String>, browse: impl Fn(&str) -> Option<PathBuf> + 'static) -> PathPicker {
    let browse: Browse = Rc::new(browse);
    PathPicker {
        value,
        browse,
        folder: false,
        placeholder: String::new(),
        label: None,
        width: None,
        fill: false,
        test_id: None,
    }
}

/// A path of a folder in `value`, as [`file_picker`]; `browse` is the app's pick-folder dialog.
pub fn folder_picker(
    value: Signal<String>,
    browse: impl Fn(&str) -> Option<PathBuf> + 'static,
) -> PathPicker {
    PathPicker { folder: true, ..file_picker(value, browse) }
}

impl PathPicker {
    /// What stands in the field while it is empty, and what a reader calls it without a label.
    pub fn placeholder(mut self, text: &str) -> Self {
        self.placeholder = text.to_string();
        self
    }

    /// What a reader calls the field, a constant or words that change.
    pub fn label<M>(mut self, name: impl IntoProp<String, M>) -> Self {
        self.label = Some(name.into_prop());
        self
    }

    /// The field's width: 240 px unless told.
    pub fn width(mut self, px: f32) -> Self {
        self.width = Some(px);
        self
    }

    /// The field takes the room left in its row, and the picker the room of its own.
    pub fn fill(mut self) -> Self {
        self.fill = true;
        self
    }

    /// The field's name for tests.
    pub fn test_id(mut self, id: &'static str) -> Self {
        self.test_id = Some(id);
        self
    }
}

impl View for PathPicker {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (value, browse) = (self.value, self.browse);
        let mut field = input(value).placeholder(&self.placeholder).mono();
        // In a form with no width told, the path takes the room the button leaves.
        let fill = self.fill || (self.width.is_none() && crate::layout::fills());
        field = if fill { field.fill() } else { field.width(self.width.unwrap_or(240.0)) };
        if let Some(name) = self.label {
            field = field.label(name);
        }
        if let Some(id) = self.test_id {
            field = field.test_id(id);
        }
        let (icon, name) = match self.folder {
            true => (icons::FOLDER, "Browse for a folder"),
            false => (icons::FILE, "Browse for a file"),
        };
        let pick = button("Browse\u{2026}").icon(icon).variant(Variant::Secondary).label(name);
        let pick = pick.on_click(move || {
            if let Some(chosen) = browse(&value.get_untracked()) {
                value.set(chosen.display().to_string());
            }
        });
        let whole = row().align_items(Align::Center).gap(token::SPACE_MD);
        let whole = match (self.fill, fill) {
            (true, _) => whole.grow(1.0).min_width(0.0),
            (false, true) => whole.width(quadra_layout::Length::Fill),
            (false, false) => whole.shrink(0.0),
        };
        whole.child(field).child(pick).mount(cx)
    }
}
