//! Quadra's retained node tree (specs/04-tree.md): each window owns one tree of nodes that
//! lives across frames. A frame is a sequence of passes, each over the nodes flagged for it.
//!
//! This crate joins the reactive runtime (bindings queue writes), the layout tree (rects kept
//! on nodes), paragraphs (text nodes) and layers (painted output kept and patched).

mod action;
mod dispatch;
mod drag;
mod drawing;
mod field;
mod focus;
mod fonts;
mod frame;
mod geometry;
mod inherit;
mod input;
mod keymap;
mod layers;
mod motion;
mod node;
mod overlay;
mod own;
mod paint;
mod patch;
mod semantics;
mod shared;
mod structure;
mod style;
mod tab;
mod text;
mod theme;
mod thumb;
mod tree;
mod verify;
mod widget;
mod window;

pub use action::{Action, FocusNext, FocusPrevious};
pub use dispatch::PointerInput;
pub use drawing::Drawing;
pub use field::{TextCommand, TextField};
pub use focus::KeyEvent;
pub use fonts::{Fonts, GlyphKey, GlyphMeta, GlyphTile, SUBPIXEL_STEPS, TileKey};
pub use frame::{FrameStats, PassTimes};
pub use inherit::{Inherited, TextValues};
pub use input::{PointerEvent, PointerKind};
pub use keymap::{Keymap, KeymapError, Keystroke, TEXT_AREA_CONTEXT, TEXT_CONTEXT, TEXT_SELECT_CONTEXT};
pub use motion::{Step, Timeline, Transition};
pub use node::{NodeId, PaintStyle, Pose, Style};
pub use overlay::{Along, Anchor, Side};
pub use patch::Patch;
pub use quadra_a11y::{Mirror, Role, SemanticAction, SemanticNode, Semantics, TreeUpdate};
pub use shared::Handle;
pub use style::State;
pub use text::Text;
pub use theme::{Paint, Theme, Token};
pub use thumb::ScrollThumb;
pub use tree::Tree;
pub use widget::{MeasureCx, PaintCx, Widget};
pub use window::WindowRequest;
