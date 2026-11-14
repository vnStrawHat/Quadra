//! What the system tells an app besides input (specs/10): its appearance, and what another
//! program drops on a window.

use std::path::PathBuf;

use quadra_prim::Color;

/// How the user set the system to look.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Appearance {
    /// Apps are asked to be dark.
    pub dark: bool,
    pub high_contrast: bool,
    /// The accent colour the user chose, if the system has one.
    pub accent: Option<Color>,
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
