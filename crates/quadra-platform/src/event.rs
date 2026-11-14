//! What a window reports. Positions and sizes are in logical pixels.

use std::ops::{BitOr, BitOrAssign};

use quadra_a11y::SemanticAction;
use quadra_prim::{Point, Size};

use crate::keys::Key;
use crate::system::{Appearance, Dropped};

/// The modifier keys held down.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Mods(u8);

impl Mods {
    pub const NONE: Mods = Mods(0);
    pub const SHIFT: Mods = Mods(1);
    pub const CTRL: Mods = Mods(2);
    pub const ALT: Mods = Mods(4);
    pub const SUPER: Mods = Mods(8);

    /// Whether every modifier of `other` is held.
    pub fn contains(self, other: Mods) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for Mods {
    type Output = Mods;
    fn bitor(self, other: Mods) -> Mods {
        Mods(self.0 | other.0)
    }
}

impl BitOrAssign for Mods {
    fn bitor_assign(&mut self, other: Mods) {
        self.0 |= other.0;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// A move or a leave: no button changed.
    #[default]
    None,
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerKind {
    Move,
    Down,
    Up,
    /// The pointer left the window; the position means nothing.
    Leave,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlatformEvent {
    Pointer {
        kind: PointerKind,
        button: MouseButton,
        at: Point,
        mods: Mods,
    },
    /// `delta` is in lines unless `precise`, then in logical pixels. Positive `y` is the wheel
    /// turned away from the user (towards the start of the content); positive `x` is to the
    /// right.
    Wheel {
        delta: Point,
        precise: bool,
        at: Point,
        mods: Mods,
    },
    /// `code` is the OS's own key code, for diagnostics only.
    Key {
        key: Key,
        code: u32,
        mods: Mods,
        down: bool,
        repeat: bool,
    },
    /// Committed text. Never a control character.
    Text(String),
    /// The text an input method is composing, not committed yet, and the caret in it as a
    /// byte offset. Empty: the composition is over (what it committed came as `Text`).
    Composition {
        text: String,
        caret: usize,
    },
    Focus(bool),
    /// The size or the scale of the window changed.
    Resize {
        size: Size,
        scale: f32,
    },
    /// The user asked to close the window. The app decides: dropping the window closes it.
    CloseRequested,
    /// Assistive technology started reading the window: from now on it is told the window's
    /// semantic tree with [`crate::PlatformWindow::update_semantics`], all of it first.
    SemanticsWanted,
    /// Assistive technology asks a node of the semantic tree to do something.
    Semantic {
        node: u64,
        action: SemanticAction,
    },
    /// The system's appearance changed: this is the new one.
    Appearance(Appearance),
    /// Something another program drags came over the window, or moved over it, or left it,
    /// or was dropped on it (where the pointer was last).
    DragEnter {
        at: Point,
        what: Dropped,
    },
    DragOver {
        at: Point,
    },
    DragLeave,
    Drop {
        at: Point,
    },
}
