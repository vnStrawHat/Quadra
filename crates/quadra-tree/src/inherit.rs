//! Text values (specs/13, "Style values"): the family, size, weight, line height and colour
//! of text are values of a node's style that what is under the node has too, unless a node
//! on the way says otherwise. A text has no style of its own: it is laid out and painted
//! with what its node and the nodes around it say.

use std::rc::Rc;

use quadra_prim::Color;
use quadra_text::TextStyle;

use crate::node::{Node, NodeId};
use crate::tree::Tree;

/// The bits of the values that shape text: a change of any other only paints.
const SHAPES: u16 = 0b11111;

/// What a node says of the text in it and under it. A value that is not set is the one of
/// the node around it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextValues {
    pub family: Option<Rc<str>>,
    /// In pixels.
    pub size: Option<f32>,
    pub weight: Option<u16>,
    pub italic: Option<bool>,
    /// A multiple of the size.
    pub line_height: Option<f32>,
    pub color: Option<Color>,
    /// What stands where text is to be typed. Unset: the text's colour at half strength.
    pub placeholder: Option<Color>,
    /// Behind selected text.
    pub selection: Option<Color>,
    /// The caret of a field. Unset: the text's colour.
    pub caret: Option<Color>,
}

impl TextValues {
    pub(crate) fn is_set(&self) -> bool {
        *self != TextValues::default()
    }

    /// Takes from a node further out what is not set yet.
    fn fill(&mut self, outer: &TextValues) {
        self.family = self.family.take().or_else(|| outer.family.clone());
        self.size = self.size.or(outer.size);
        self.weight = self.weight.or(outer.weight);
        self.italic = self.italic.or(outer.italic);
        self.line_height = self.line_height.or(outer.line_height);
        self.color = self.color.or(outer.color);
        self.placeholder = self.placeholder.or(outer.placeholder);
        self.selection = self.selection.or(outer.selection);
        self.caret = self.caret.or(outer.caret);
    }

    /// A bit for each value that is set: the five that shape text first (`SHAPES`), then
    /// the colours.
    fn set_bits(&self) -> u16 {
        let bits = [
            self.family.is_some(),
            self.size.is_some(),
            self.weight.is_some(),
            self.italic.is_some(),
            self.line_height.is_some(),
            self.color.is_some(),
            self.placeholder.is_some(),
            self.selection.is_some(),
            self.caret.is_some(),
        ];
        bits.iter().enumerate().fold(0, |all, (i, &set)| all | (set as u16) << i)
    }

    /// A bit for each value that is not the same in `other`.
    fn differs(&self, other: &TextValues) -> u16 {
        let bits = [
            self.family != other.family,
            self.size != other.size,
            self.weight != other.weight,
            self.italic != other.italic,
            self.line_height != other.line_height,
            self.color != other.color,
            self.placeholder != other.placeholder,
            self.selection != other.selection,
            self.caret != other.caret,
        ];
        bits.iter().enumerate().fold(0, |all, (i, &set)| all | (set as u16) << i)
    }

    fn complete(&self) -> bool {
        self.set_bits() == (1 << 9) - 1
    }
}

/// The text values that hold at a node: what a widget lays out and paints its text with.
#[derive(Clone, Debug, PartialEq)]
pub struct Inherited {
    pub style: TextStyle,
    pub color: Color,
    pub placeholder: Color,
    pub selection: Color,
    pub caret: Color,
}

/// Behind selected text where no node says another colour.
const SELECTION: Color = Color::rgba(0x3366FF4D);

impl Default for Inherited {
    fn default() -> Inherited {
        let (style, color) = (TextStyle::default(), Color::BLACK);
        Inherited { style, color, placeholder: color.alpha(0.5), selection: SELECTION, caret: color }
    }
}

/// What holds at a node: its own values, then those of each node around it for what is
/// still not set, then the defaults. It stops as soon as everything is set.
pub(crate) fn inherit(nodes: &[Node], index: usize, root: Option<u32>) -> Inherited {
    let (mut values, mut at) = (TextValues::default(), Some(index));
    while let Some(i) = at.filter(|_| !values.complete()) {
        if let Some(own) = &nodes[i].text {
            values.fill(own);
        }
        // What has no parent and is not the window's root is above it: an overlay. The text
        // values of the window hold there too.
        let above = root.map(|root| root as usize).filter(|&root| root != i);
        at = nodes[i].parent.map(|parent| parent.index as usize).or(above);
    }
    let base = Inherited::default();
    let color = values.color.unwrap_or(base.color);
    Inherited {
        style: TextStyle {
            family: values.family.unwrap_or(base.style.family),
            size: values.size.unwrap_or(base.style.size),
            weight: values.weight.unwrap_or(base.style.weight),
            italic: values.italic.unwrap_or(base.style.italic),
            line_height: values.line_height.unwrap_or(base.style.line_height),
        },
        color,
        placeholder: values.placeholder.unwrap_or(color.alpha(color.a * 0.5)),
        selection: values.selection.unwrap_or(base.selection),
        caret: values.caret.unwrap_or(color),
    }
}

/// What a pass needs to give widgets their text values: the tree's count of changes to
/// them, and the window's root, whose values hold in overlays too.
#[derive(Clone, Copy)]
pub(crate) struct TextAt {
    pub epoch: u32,
    pub root: Option<u32>,
}

/// Gives a node's widget the text values that hold at the node, if they may have changed
/// since it was last given them.
pub(crate) fn refresh(nodes: &mut [Node], index: usize, at: TextAt) {
    if nodes[index].inherited_at != at.epoch && nodes[index].widget.is_some() {
        let inherited = inherit(nodes, index, at.root);
        let node = &mut nodes[index];
        node.inherited_at = at.epoch;
        if let Some(widget) = &mut node.widget {
            widget.inherit(&inherited);
        }
    }
}

impl Tree {
    /// The text values a node says itself.
    pub fn text_values(&self, id: NodeId) -> TextValues {
        self.get(id).and_then(|node| node.text.as_deref().cloned()).unwrap_or_default()
    }

    /// What the passes give widgets their text values with.
    pub(crate) fn text_at(&self) -> TextAt {
        TextAt { epoch: self.text_epoch, root: self.root.map(|root| root.index) }
    }

    /// The text values that hold at a node.
    pub fn inherited(&self, id: NodeId) -> Inherited {
        let root = self.root.map(|root| root.index);
        if self.is_alive(id) { inherit(&self.nodes, id.index as usize, root) } else { Inherited::default() }
    }

    /// Gives a node the text values it says. The widgets in it and under it are laid out
    /// again when what shapes text changed, and only painted again when the colour did.
    pub(crate) fn put_text(&mut self, id: NodeId, values: TextValues) {
        let Some(node) = self.get_mut(id) else { return };
        let old = node.text.as_deref().cloned().unwrap_or_default();
        if old == values {
            return;
        }
        // Which values changed: only texts that have them from this node are touched.
        let changed = old.differs(&values);
        node.text = values.is_set().then(|| Box::new(values));
        self.text_epoch += 1;
        // The values of the window's root hold in the overlays too.
        let above: Vec<NodeId> = if self.root == Some(id) { self.overlays().collect() } else { Vec::new() };
        let mut under: Vec<(NodeId, u16)> =
            std::iter::once(id).chain(above).map(|at| (at, changed)).collect();
        while let Some((at, changed)) = under.pop() {
            let node = &self.nodes[at.index as usize];
            // A node on the way that says a value itself keeps it, with what is under it.
            let own = node.text.as_deref().filter(|_| at != id).map_or(0, TextValues::set_bits);
            let changed = changed & !own;
            if changed == 0 {
                continue;
            }
            under.extend(node.children.iter().map(|&child| (child, changed)));
            if node.widget.is_some() {
                if changed & SHAPES != 0 {
                    self.mark_layout(at);
                }
                self.mark_paint(at);
            }
        }
    }

    /// A node left its parent, for another or for none: the text in it and under it may be
    /// shaped otherwise where it comes to be, so it is measured again there.
    pub(crate) fn text_moved(&mut self, top: NodeId) {
        self.text_epoch += 1;
        let mut under = vec![top];
        while let Some(at) = under.pop() {
            let node = &self.nodes[at.index as usize];
            under.extend(&node.children);
            if node.widget.is_some() {
                self.layout.mark_dirty(node.layout);
            }
        }
    }
}
