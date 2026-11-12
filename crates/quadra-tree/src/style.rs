//! A node's style as data (specs/13, "Style values"; specs/05, "Style and states"): its own
//! values, and what each state changes of them. A state that changes on a node with a variant
//! for it flags the node for the style pass, which resolves the style again; the difference
//! decides whether layout or paint follows. No app code runs.

use std::ops::BitOr;

use crate::inherit::TextValues;
use crate::node::{NodeId, PaintStyle, Style, flag};
use crate::patch::{Part, Patch};
use crate::tree::Tree;

/// The states of a node that a style can follow. Dispatch sets the first five (specs/09); the
/// others are properties. Variants are applied in this order, a later one over an earlier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct State(u16);

impl State {
    pub const NONE: State = State(0);
    pub const HOVER: State = State(1);
    /// The focus is on the node or on something under it.
    pub const FOCUS_WITHIN: State = State(2);
    pub const FOCUS: State = State(4);
    /// The focus is on the node and came by the keyboard: it shows a ring.
    pub const FOCUS_VISIBLE: State = State(8);
    pub const ACTIVE: State = State(16);
    pub const SELECTED: State = State(32);
    pub const CHECKED: State = State(64);
    pub const EXPANDED: State = State(128);
    pub const DROP_TARGET: State = State(256);
    pub const DISABLED: State = State(512);
    /// What the node holds is not accepted: a field whose value is wrong.
    pub const INVALID: State = State(1024);

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

/// What a node whose style is more than its values keeps: its own style, the values of it
/// that are roles of the theme, and what each state changes.
pub(crate) struct Styled {
    base: Style,
    /// Applied over `base` with the theme of the moment.
    roles: Vec<Patch>,
    /// In the order they are applied.
    variants: Vec<(State, Vec<Patch>)>,
}

impl Styled {
    fn of(base: Style) -> Styled {
        Styled { base, roles: Vec::new(), variants: Vec::new() }
    }

    /// Whether a state changes the style.
    pub fn has_variants(&self) -> bool {
        !self.variants.is_empty()
    }

    /// Whether the theme has a say in the style.
    pub fn themed(&self) -> bool {
        !self.roles.is_empty() || self.variants.iter().any(|(_, patches)| patches.iter().any(|p| p.themed()))
    }
}

impl Tree {
    fn own_style(&self, id: NodeId) -> Style {
        let (layout, node) = (self.layout_style(id).clone(), self.node(id));
        Style { layout, paint: node.paint, pose: self.pose(id), text: self.text_values(id) }
    }

    /// Makes a node keep its own style apart from the one it shows: for what resolves it.
    pub(crate) fn keep_own_style(&mut self, id: NodeId) {
        if self.nodes[id.index as usize].styled.is_none() {
            let base = self.own_style(id);
            self.nodes[id.index as usize].styled = Some(Box::new(Styled::of(base)));
        }
    }

    /// A node whose style may say a pose is a layer before it does: a pose that comes with
    /// a state, or changes, then paints nothing.
    fn poses(&mut self, id: NodeId, patches: &[Patch]) {
        if patches.iter().any(|patch| patch.part() == Part::Pose) {
            self.make_layer(id, false);
        }
    }

    /// Declares what a state changes of a node's style. The values are applied over the
    /// node's own while the state is set, a later state over an earlier one.
    pub fn set_variant(&mut self, id: NodeId, state: State, patches: Vec<Patch>) {
        if self.get(id).is_none() {
            return;
        }
        self.poses(id, &patches);
        self.keep_own_style(id);
        let node = &mut self.nodes[id.index as usize];
        let Some(styled) = &mut node.styled else { return };
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

    /// Changes one value of a node's own style. A value that is a role of the theme is
    /// kept as one: the node gets another colour when the theme changes.
    pub fn patch_style(&mut self, id: NodeId, patch: Patch) {
        if self.get(id).is_none() {
            return;
        }
        self.poses(id, &[patch]);
        if patch.themed() {
            self.keep_own_style(id);
        }
        let theme = self.theme.clone();
        let node = &mut self.nodes[id.index as usize];
        if let Some(styled) = &mut node.styled {
            // The last value given for a property is the one that counts, role or not.
            styled.roles.retain(|role| std::mem::discriminant(role) != std::mem::discriminant(&patch));
            match patch.themed() {
                true => styled.roles.push(patch),
                false => patch.apply(&mut styled.base, &theme),
            }
            return self.resolve(id);
        }
        // Only the part of the style the value is of is read, changed and put back.
        let part = patch.part();
        let mut style = match part {
            Part::Layout => Style { layout: self.layout_style(id).clone(), ..Style::default() },
            Part::Paint => Style { paint: node.paint, ..Style::default() },
            Part::Text => Style { text: self.text_values(id), ..Style::default() },
            Part::Pose => Style { pose: self.pose(id), ..Style::default() },
        };
        patch.apply(&mut style, &theme);
        match part {
            Part::Layout => self.put_layout(id, style.layout),
            Part::Paint => self.put_paint(id, style.paint),
            Part::Text => self.put_text(id, style.text),
            Part::Pose => self.put_pose(id, style.pose),
        }
    }

    /// Replaces the text values a node says: what the text in it and under it is like,
    /// where a node on the way does not say otherwise.
    pub fn set_text_values(&mut self, id: NodeId, values: TextValues) {
        match self.get_mut(id).map(|node| &mut node.styled) {
            Some(Some(styled)) => {
                styled.base.text = values;
                self.resolve(id);
            }
            Some(None) => self.put_text(id, values),
            None => {}
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
        let states = styled.variants.iter().filter(|(state, _)| node.state.contains(*state));
        for patch in styled.roles.iter().chain(states.flat_map(|(_, patches)| patches)) {
            patch.apply(&mut style, &self.theme);
        }
        self.ease(id, &mut style);
        self.put_layout(id, style.layout);
        self.put_paint(id, style.paint);
        self.put_pose(id, style.pose);
        self.put_text(id, style.text);
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
            let around = node.parent.filter(|_| shown != 0);
            self.layout.set_style(layout, style);
            self.relayout = true;
            self.mark(id, flag::PAINT | flag::SUBTREE | shown);
            // Shown or hidden, it joins or leaves what its parent holds for a listener.
            if let Some(parent) = around {
                self.semantics_changed(parent);
            }
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
