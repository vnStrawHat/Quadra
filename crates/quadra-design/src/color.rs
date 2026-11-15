//! The colour input of the kit (specs/13, `color_input`): the kit's input for `#rrggbb`, with
//! a swatch of the colour typed before the text.

use quadra_prim::Color;
use quadra_reactive::Signal;
use quadra_tree::{NodeId, Paint};
use quadra_view::{AnyView, MountCx, View, div};

use crate::input::input;
use crate::token;

/// A field for a colour; see [`color_input`].
pub struct ColorInput {
    value: Signal<String>,
    placeholder: String,
    width: f32,
    fallback: Paint,
}

/// A field, 120 px wide, for a colour written `#rrggbb` (or `#rrggbbaa`) in `value`.
pub fn color_input(value: Signal<String>) -> ColorInput {
    ColorInput { value, placeholder: String::new(), width: 120.0, fallback: Paint::default() }
}

impl ColorInput {
    /// What stands in the field while it is empty, and what a reader calls it.
    pub fn placeholder(mut self, text: &str) -> Self {
        self.placeholder = text.to_string();
        self
    }

    pub fn width(mut self, px: f32) -> Self {
        self.width = px;
        self
    }

    /// What the swatch shows while no colour is typed: the one that stands when none is set.
    pub fn fallback(mut self, paint: impl Into<Paint>) -> Self {
        self.fallback = paint.into();
        self
    }
}

impl View for ColorInput {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (value, fallback) = (self.value, self.fallback);
        let typed = move || value.with(|it| Color::parse_hex(it.trim()));
        let swatch = div().size(14.0, 14.0).shrink(0.0).radius(token::RADIUS_SM).border(1.0);
        let swatch =
            swatch.border_color(token::LINE_STRONG).bg(move || typed().map_or(fallback, Paint::from));
        let invalid = move || typed().is_none() && !value.with(|it| it.trim().is_empty());
        let mut field = input(value).placeholder(&self.placeholder).width(self.width).invalid(invalid);
        field.before = Some(AnyView::new(swatch));
        field.mount(cx)
    }
}
