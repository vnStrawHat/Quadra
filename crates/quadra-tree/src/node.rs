//! Nodes: what the tree keeps for each element across frames (specs/04, "Storage").

use std::rc::Rc;

use quadra_a11y::Semantics;
use quadra_layout::LayoutId;
use quadra_platform::Cursor;
use quadra_prim::{Color, Corners};
use quadra_reactive::Scope;

use crate::inherit::TextValues;
use crate::input::Handler;
use crate::layers::LayerState;
use crate::style::{State, Styled};
use crate::widget::Widget;

/// A node of a [`crate::Tree`]: an index and a generation, so a stale id is detected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

/// Why a node takes part in the next paint pass.
pub(crate) mod flag {
    /// Its own instances must be painted again.
    pub const PAINT: u8 = 1;
    /// Everything under it too: it moved, or it is new.
    pub const SUBTREE: u8 = 2;
    /// A descendant has a flag: the pass walks down through it.
    pub const CHILD: u8 = 4;
    /// Its list of children changed: ranges move, so the layer is rebuilt.
    pub const RELIST: u8 = 8;
}

/// The paint part of a node's style: what the tree draws behind the node's content.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PaintStyle {
    pub background: Color,
    pub border_color: Color,
    pub radii: Corners,
    /// A line of one pixel along the node's edge, over what the node paints and taking no
    /// room: a mark of the focus.
    pub outline: Color,
    /// How far above the window the node is: 0 casts no shadow, 1 that of a menu, 2 that of
    /// a dialog.
    pub elevation: u8,
}

/// A node's style. Border widths are layout values, so they are in `layout`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    pub layout: quadra_layout::Style,
    pub paint: PaintStyle,
    /// What the node says of the text in it and under it.
    pub text: TextValues,
}

/// No scratch range: the node was not painted in this pass.
pub(crate) const NOT_PAINTED: (u32, u32) = (u32::MAX, u32::MAX);

pub(crate) struct Node {
    pub generation: u32,
    pub alive: bool,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub layout: LayoutId,
    pub flags: u8,
    pub paint: PaintStyle,
    /// The text values the node says, when it says any (specs/13).
    pub text: Option<Box<TextValues>>,
    /// The tree's count of changes to text values when the widget was last given those
    /// that hold here: it is given them again only after another change.
    pub inherited_at: u32,
    pub widget: Option<Box<dyn Widget>>,
    /// Its range in the layer's display list: `start` from its parent's start, `len` with
    /// everything under it, `own` the instances it painted itself, which come first
    /// (specs/07, "Display lists").
    pub start: u32,
    pub len: u32,
    pub own: u32,
    /// Where this pass's instances are in the scratch buffer, or `NOT_PAINTED`.
    pub fresh: (u32, u32),
    /// Set when the node is a layer: it and what is under it have a display list of their own.
    pub layer: Option<Box<LayerState>>,
    /// The scope that ends with this node: its bindings, handlers and tasks.
    pub scope: Option<Scope>,
    /// What runs when pointer events reach the node (specs/09).
    pub handlers: Vec<Handler>,
    /// It can have the focus (specs/09, "Focus").
    pub focusable: bool,
    /// Its place in the order Tab walks: above zero before the others, lowest first; below
    /// zero not at all.
    pub tab_index: i16,
    /// Tab and Shift+Tab stay among what is under it: a dialog, a menu.
    pub focus_scope: bool,
    /// The shape the pointer has over it and what is under it (specs/09).
    pub cursor: Option<Cursor>,
    /// Its name as a context of key bindings (specs/09).
    pub key_context: Option<Rc<str>>,
    /// What it is to a user, when that was said (specs/12).
    pub semantics: Option<Box<Semantics>>,
    /// The states that are set (specs/04, "State bits").
    pub state: State,
    /// Its own style and what its states change of it, when it has variants.
    pub styled: Option<Box<Styled>>,
}
