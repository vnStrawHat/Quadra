//! The drawing view: a vector drawing shown as a mask in the text colour, an icon.

use std::rc::Rc;

use quadra_layout::Length;
use quadra_tree::{Drawing, NodeId, Paint, Patch, Style};
use quadra_vector::Svg;

use crate::view::{MountCx, View};

pub struct DrawingView {
    svg: Rc<Svg>,
    side: f32,
    color: Option<Paint>,
}

/// A drawing, 16 px square, in the colour of the text around it.
pub fn drawing(svg: Rc<Svg>) -> DrawingView {
    DrawingView { svg, side: 16.0, color: None }
}

impl DrawingView {
    /// The side of the square the drawing is fitted into, in pixels.
    pub fn size(mut self, px: f32) -> Self {
        self.side = px;
        self
    }

    /// A colour of its own, or a role of the theme.
    pub fn color(mut self, color: impl Into<Paint>) -> Self {
        self.color = Some(color.into());
        self
    }
}

impl View for DrawingView {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        // It does not shrink: an icon beside a long label keeps its size.
        let side = Length::Px(self.side);
        let layout = quadra_layout::Style { width: side, height: side, shrink: 0.0, ..Default::default() };
        let id =
            cx.tree.insert_widget(Style { layout, ..Style::default() }, Box::new(Drawing::new(self.svg)));
        if let Some(color) = self.color {
            cx.tree.patch_style(id, Patch::TextColor(color));
        }
        id
    }
}
