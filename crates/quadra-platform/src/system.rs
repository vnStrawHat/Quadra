//! What the system tells an app besides input (specs/10): its appearance, and what another
//! program drops on a window.

use std::path::PathBuf;

use quadra_prim::Color;

/// How the user set the system to look.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Appearance {
    /// Apps are asked to be dark.
    pub dark: bool,
    pub high_contrast: bool,
    /// The accent colour the user chose, if the system has one.
    pub accent: Option<Color>,
    /// The colours of the system's own controls, if it says them: under high contrast,
    /// those of the contrast the user chose.
    pub colors: Option<SystemColors>,
    /// How much larger the user wants text, 1 at the system's own size (specs/12, "Text
    /// size").
    pub text_scale: f32,
}

impl Default for Appearance {
    fn default() -> Appearance {
        Appearance { dark: false, high_contrast: false, accent: None, colors: None, text_scale: 1.0 }
    }
}

/// How the system writes the keys of a shortcut, which what shows one follows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyNames {
    /// Words joined by "-": "Ctrl-Shift-P" (Windows).
    #[default]
    Words,
    /// The Mac's signs, the key last: "⇧⌘P".
    Signs,
}

impl KeyNames {
    /// As the system the program is built for writes them.
    pub const fn of_system() -> KeyNames {
        if cfg!(target_os = "macos") { KeyNames::Signs } else { KeyNames::Words }
    }
}

/// The colours a system paints its own controls with.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SystemColors {
    /// The background of a window, and its text.
    pub window: Color,
    pub text: Color,
    /// What is selected, and its text.
    pub highlight: Color,
    pub highlight_text: Color,
    /// A link.
    pub link: Color,
    /// Text that cannot be used.
    pub disabled: Color,
    /// The face of a button, and its text.
    pub button: Color,
    pub button_text: Color,
}

/// What is dragged over a window from another program: files when there are any, otherwise
/// text.
#[derive(Clone, Debug, PartialEq)]
pub enum Dropped {
    Files(Vec<PathBuf>),
    Text(String),
}

/// Files dragged from another program, as the payload of a drag in the tree.
#[derive(Clone, Debug, PartialEq)]
pub struct DroppedFiles(pub Vec<PathBuf>);

/// Text dragged from another program, as the payload of a drag in the tree.
#[derive(Clone, Debug, PartialEq)]
pub struct DroppedText(pub String);
