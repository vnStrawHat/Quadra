//! Helpers shared by the layout tests.
#![allow(dead_code)]

use std::collections::HashMap;

use quadra_layout::{Holds, Known, LayoutId, LayoutTree, Length, Measure, RootSize, Style};
use quadra_prim::{Rect, Size};

/// Fake text: 8 px per character in 16 px lines, wrapping at any character. Like a real text
/// it says which available widths its size holds for, and that it never reads the height.
#[derive(Default)]
pub struct Text {
    pub chars: HashMap<LayoutId, usize>,
    /// The available widths the last measure holds for.
    holds: (f32, f32),
}

impl Measure for Text {
    fn measure(&mut self, node: LayoutId, known: Known, avail: Size) -> Size {
        let chars = self.chars[&node];
        let full = chars as f32 * 8.0;
        let limit = known.width.unwrap_or(avail.width);
        if !limit.is_finite() || full <= limit {
            self.holds = (full, f32::INFINITY);
            return Size::new(full, 16.0);
        }
        let per_line = ((limit / 8.0) as usize).max(1);
        self.holds = ((per_line as f32 * 8.0).min(limit), limit);
        Size::new(per_line as f32 * 8.0, chars.div_ceil(per_line) as f32 * 16.0)
    }

    fn holds(&mut self, _: LayoutId) -> Option<Holds> {
        Some(Holds { width: self.holds, any_height: true })
    }
}

/// The same text, saying nothing of what its sizes hold for: every request is measured as
/// asked. What a layout with [`Text`] must match.
pub struct Exact<'a>(pub &'a mut Text);

impl Measure for Exact<'_> {
    fn measure(&mut self, node: LayoutId, known: Known, avail: Size) -> Size {
        self.0.measure(node, known, avail)
    }
}

impl Text {
    /// Adds a text leaf of `chars` characters.
    pub fn leaf(&mut self, tree: &mut LayoutTree, chars: usize) -> LayoutId {
        let id = tree.insert_leaf(Style::default());
        self.chars.insert(id, chars);
        id
    }
}

/// A container with its children.
pub fn node(tree: &mut LayoutTree, style: Style, children: &[LayoutId]) -> LayoutId {
    let id = tree.insert(style);
    tree.set_children(id, children);
    id
}

pub fn px(w: f32, h: f32) -> Style {
    Style { width: Length::Px(w), height: Length::Px(h), ..Style::default() }
}

pub fn fill(w: f32, h: f32) -> RootSize {
    RootSize::Fill(Size::new(w, h))
}

pub fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::new(x, y, w, h)
}
