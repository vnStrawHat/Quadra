//! The text view.

use quadra_prim::Color;
use quadra_text::TextStyle;
use quadra_tree::{NodeId, Style, Text};

use crate::prop::{IntoProp, Prop};
use crate::view::{MountCx, View};

pub struct TextView {
    content: Prop<String>,
    style: TextStyle,
    color: Color,
}

/// Text: a constant, or something reactive whose changes set the node's text.
pub fn text<M>(content: impl IntoProp<String, M>) -> TextView {
    TextView { content: content.into_prop(), style: TextStyle::default(), color: Color::BLACK }
}

impl TextView {
    /// The font size in pixels.
    pub fn size(mut self, px: f32) -> Self {
        self.style.size = px;
        self
    }

    pub fn weight(mut self, weight: u16) -> Self {
        self.style.weight = weight;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl View for TextView {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (initial, read) = match self.content {
            Prop::Value(text) => (text, None),
            Prop::Reactive(read) => (String::new(), Some(read)),
        };
        let widget = Text::new(initial, self.style, self.color);
        let id = cx.tree.insert_widget(Style::default(), Box::new(widget));
        if let Some(read) = read {
            let writer = cx.handle.clone();
            cx.handle.bind(move || writer.set_text(id, read()));
        }
        id
    }
}
