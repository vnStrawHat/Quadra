//! A node's style as data (specs/13, "Style values"; specs/05, "Style and states"): its own
//! values, and what each state changes of them. A state that changes on a node with a variant
//! for it flags the node for the style pass, which resolves the style again; the difference
//! decides whether layout or paint follows. No app code runs.

use std::ops::BitOr;

use quadra_layout::{Align, Direction, Justify, Length, Overflow};
use quadra_prim::{Color, Corners, Edges};

use crate::node::{NodeId, PaintStyle, Style, flag};
use crate::tree::Tree;

/// The states of a node that a style can follow. Dispatch sets the first four (specs/09); the
/// others are properties. Variants are applied in this order, a later one over an earlier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct State(u16);

impl State {
    pub const NONE: State = State(0);
    pub const HOVER: State = State(1);
    pub const FOCUS: State = State(2);
    pub const FOCUS_VISIBLE: State = State(4);
    pub const ACTIVE: State = State(8);
    pub const SELECTED: State = State(16);
    pub const CHECKED: State = State(32);
    pub const EXPANDED: State = State(64);
    pub const DROP_TARGET: State = State(128);
    pub const DISABLED: State = State(256);

    pub fn contains(self, other: State) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for State {
    type Output = State;
    fn bitor(self, other: State) -> State {
        State(self.0 | other.0)
    }
}

/// One value of a style: what a reactive property writes, and what a state variant is a list
/// of. Each is named after the setter of the box view that makes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Patch {
    Width(Length),
    Height(Length),
    MinWidth(Length),
    MinHeight(Length),
    MaxWidth(Length),
    MaxHeight(Length),
    Grow(f32),
    Shrink(f32),
    /// The space between children, on both axes.
    Gap(f32),
    /// The same on every side.
    Padding(f32),
    PaddingX(f32),
    PaddingY(f32),
    Margin(f32),
    /// The width of the border on every side.
    Border(f32),
    Direction(Direction),
    Justify(Justify),
    AlignItems(Align),
    Wrap(bool),
    OverflowY(Overflow),
    Background(Color),
    BorderColor(Color),
    /// The same radius on every corner.
    Radius(f32),
}

impl Patch {
    /// Sets the value in a style. Returns whether it is a layout value.
    pub fn apply(self, style: &mut Style) -> bool {
        let (l, p) = (&mut style.layout, &mut style.paint);
        match self {
            Patch::Background(v) => p.background = v,
            Patch::BorderColor(v) => p.border_color = v,
            Patch::Radius(v) => p.radii = Corners::all(v),
            layout => {
                match layout {
                    Patch::Width(v) => l.width = v,
                    Patch::Height(v) => l.height = v,
                    Patch::MinWidth(v) => l.min_width = v,
                    Patch::MinHeight(v) => l.min_height = v,
                    Patch::MaxWidth(v) => l.max_width = v,
                    Patch::MaxHeight(v) => l.max_height = v,
                    Patch::Grow(v) => l.grow = v,
                    Patch::Shrink(v) => l.shrink = v,
                    Patch::Gap(v) => (l.gap_row, l.gap_col) = (v, v),
                    Patch::Padding(v) => l.padding = Edges::all(v),
                    Patch::PaddingX(v) => (l.padding.left, l.padding.right) = (v, v),
                    Patch::PaddingY(v) => (l.padding.top, l.padding.bottom) = (v, v),
                    Patch::Margin(v) => l.margin = Edges::all(v),
                    Patch::Border(v) => l.border = Edges::all(v),
                    Patch::Direction(v) => l.direction = v,
                    Patch::Justify(v) => l.justify = v,
                    Patch::AlignItems(v) => l.align_items = v,
                    Patch::Wrap(v) => l.wrap = v,
                    Patch::OverflowY(v) => l.overflow_y = v,
                    Patch::Background(_) | Patch::BorderColor(_) | Patch::Radius(_) => {}
                }
                return true;
            }
        }
        false
    }

    fn is_layout(self) -> bool {
        !matches!(self, Patch::Background(_) | Patch::BorderColor(_) | Patch::Radius(_))
    }
}

/// What a node with state variants keeps: its own style, and what each state changes.
pub(crate) struct Styled {
    base: Style,
    /// In the order they are applied.
    variants: Vec<(State, Vec<Patch>)>,
}

impl Tree {
    fn own_style(&self, id: NodeId) -> Style {
        Style { layout: self.layout_style(id).clone(), paint: self.node(id).paint }
    }

    /// Declares what a state changes of a node's style. The values are applied over the
    /// node's own while the state is set, a later state over an earlier one.
    pub fn set_variant(&mut self, id: NodeId, state: State, patches: Vec<Patch>) {
        if self.get(id).is_none() {
            return;
        }
        let base = self.own_style(id);
        let node = &mut self.nodes[id.index as usize];
        let styled = node.styled.get_or_insert_with(|| Box::new(Styled { base, variants: Vec::new() }));
        styled.variants.retain(|(s, _)| *s != state);
        styled.variants.push((state, patches));
        styled.variants.sort_by_key(|(s, _)| *s);
        if node.state.contains(state) {
            self.restyle.push(id);
        }
    }

    /// The states that are set on a node.
    pub fn state(&self, id: NodeId) -> State {
        self.get(id).map_or(State::NONE, |node| node.state)
    }

    /// Sets or clears a state of a node. Only a node with a variant for it is styled again:
    /// a state that changes nothing costs nothing.
    pub fn set_state(&mut self, id: NodeId, state: State, on: bool) {
        let Some(node) = self.get_mut(id) else { return };
        if node.state.contains(state) == on {
            return;
        }
        node.state = State(if on { node.state.0 | state.0 } else { node.state.0 & !state.0 });
        if node.styled.as_ref().is_some_and(|s| s.variants.iter().any(|(of, _)| *of == state)) {
            self.restyle.push(id);
        }
    }

    /// Changes one value of a node's own style.
    pub fn patch_style(&mut self, id: NodeId, patch: Patch) {
        let Some(node) = self.get_mut(id) else { return };
        if let Some(styled) = &mut node.styled {
            patch.apply(&mut styled.base);
            return self.resolve(id);
        }
        if patch.is_layout() {
            let mut style = self.own_style(id);
            patch.apply(&mut style);
            self.put_layout(id, style.layout);
        } else {
            let mut style = Style { paint: node.paint, ..Style::default() };
            patch.apply(&mut style);
            self.put_paint(id, style.paint);
        }
    }

    /// Replaces a node's own layout style. Nothing happens if it is equal or the node is gone.
    pub fn set_layout_style(&mut self, id: NodeId, style: quadra_layout::Style) {
        match self.get_mut(id).map(|node| &mut node.styled) {
            Some(Some(styled)) => {
                styled.base.layout = style;
                self.resolve(id);
            }
            Some(None) => self.put_layout(id, style),
            None => {}
        }
    }

    /// Replaces a node's own paint style. Nothing happens if it is equal or the node is gone.
    pub fn set_paint_style(&mut self, id: NodeId, style: PaintStyle) {
        match self.get_mut(id).map(|node| &mut node.styled) {
            Some(Some(styled)) => {
                styled.base.paint = style;
                self.resolve(id);
            }
            Some(None) => self.put_paint(id, style),
            None => {}
        }
    }

    /// The style pass: nodes whose state changed, and that have a variant for it, get their
    /// style resolved again.
    pub(crate) fn style_pass(&mut self) {
        for id in std::mem::take(&mut self.restyle) {
            if self.get(id).is_some_and(|node| node.styled.is_some()) {
                self.resolve(id);
                self.stats.nodes_styled += 1;
            }
        }
    }

    /// The node's own style with the variants of its states over it, in order. What differs
    /// from the style it has flags layout or paint; what is equal flags nothing.
    fn resolve(&mut self, id: NodeId) {
        let node = &self.nodes[id.index as usize];
        let Some(styled) = &node.styled else { return };
        let mut style = styled.base.clone();
        for (_, patches) in styled.variants.iter().filter(|(state, _)| node.state.contains(*state)) {
            for patch in patches {
                patch.apply(&mut style);
            }
        }
        self.put_layout(id, style.layout);
        self.put_paint(id, style.paint);
    }

    /// Gives a node the layout style it is laid out with.
    pub(crate) fn put_layout(&mut self, id: NodeId, style: quadra_layout::Style) {
        let Some(node) = self.get(id) else { return };
        let layout = node.layout;
        let old = self.layout.style(layout);
        if *old != style {
            // Border widths and the frame the content sits in are painted from this style.
            // Showing or hiding changes how many instances the subtree has: ranges move.
            let shown = if old.display != style.display { flag::RELIST } else { 0 };
            self.layout.set_style(layout, style);
            self.relayout = true;
            self.mark(id, flag::PAINT | flag::SUBTREE | shown);
        }
    }

    /// Gives a node the paint style it is painted with.
    pub(crate) fn put_paint(&mut self, id: NodeId, style: PaintStyle) {
        let Some(node) = self.get_mut(id) else { return };
        if node.paint != style {
            node.paint = style;
            self.mark(id, flag::PAINT);
        }
    }
}
