//! The bar along the top (specs/21, "Window"): the file's name, the unsaved mark, the layout.

use quadra::{Align, Display, Div, div, row, text};
use quadra_design::{Size, segmented, token};

use crate::model::Model;

pub(crate) fn bar(model: &Model) -> Div {
    let (m, changed) = (model.clone(), model.changed);
    let name = div().font_weight(600).text_size(13.0).child(text(move || m.name()));
    let dot = div()
        .size(6.0, 6.0)
        .radius(3.0)
        .bg(token::ACCENT)
        .label("Unsaved changes")
        .role(quadra::Role::Image)
        .display(move || if changed.get() { Display::Flex } else { Display::None });
    let layout = segmented(["Raw", "Side by side", "Preview"], model.layout).size(Size::Sm);
    row()
        .height(40.0)
        .shrink(0.0)
        .align_items(Align::Center)
        .gap(8.0)
        .padding_x(12.0)
        .bg(token::PANEL)
        .border_bottom(1.0)
        .border_color(token::LINE)
        .child(name)
        .child(dot)
        .child(div().grow(1.0))
        .child(layout)
}
