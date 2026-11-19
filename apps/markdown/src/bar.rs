//! The bar along the top (specs/21, "Window"): the file's name, the unsaved mark, the layout.

use quadra::{Align, Display, Div, div, row, text};
use quadra_design::{Size, icon_button, icons, segmented, token, tooltip};

use crate::files::Files;
use crate::model::Model;

pub(crate) fn bar(model: &Model, files: &Files) -> Div {
    let (opens, saves) = (files.clone(), files.clone());
    let open = icon_button(icons::FOLDER_OPEN, "Open").on_click(move || opens.open());
    let save = icon_button(icons::SAVE, "Save").on_click(move || saves.save(|| {}));
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
        .child(crate::logo::logo(20.0))
        .child(name)
        .child(dot)
        .child(div().grow(1.0))
        .child(layout)
        .child(tooltip("Open (Ctrl+O)", open))
        .child(tooltip("Save (Ctrl+S)", save))
}
