//! The bar along the top (specs/21, "Window"): the kit's title bar of the window's own frame,
//! with the logo, the file's name, the unsaved mark, the layout and the app's buttons.

use quadra::{Display, View, div, text};
use quadra_design::{Size, icon_button, icons, segmented, titlebar, token, tooltip};

use crate::files::Files;
use crate::model::Model;

pub(crate) fn bar(model: &Model, files: &Files) -> impl View + use<> {
    let (opens, saves) = (files.clone(), files.clone());
    let open = icon_button(icons::FOLDER_OPEN, "Open").on_click(move || opens.open());
    let save = icon_button(icons::SAVE, "Save").on_click(move || saves.save(|| {}));
    let outlined = model.outline;
    let (opens, windows) = (model.clone(), quadra::use_context::<quadra::Windows>());
    let settings = icon_button(icons::SETTINGS, "Settings").on_click(move || {
        if let Some(windows) = &windows {
            crate::settings::open_window(windows, &opens);
        }
    });
    let outline =
        icon_button(icons::PANEL_LEFT, "Outline").on_click(move || outlined.update(|on| *on = !*on));
    let (m, changed) = (model.clone(), model.changed);
    let name = div().font_weight(600).text_size(14.0).child(text(move || m.name()));
    let dot = div()
        .size(6.0, 6.0)
        .radius(4.0)
        .bg(token::ACCENT)
        .label("Unsaved changes")
        .role(quadra::Role::Image)
        .display(move || if changed.get() { Display::Flex } else { Display::None });
    let layout = segmented(["Raw", "Side by side", "Preview"], model.layout).size(Size::Sm);
    // The empty room between the name and the layout drags the window.
    titlebar("")
        .height(40.0)
        .control(div().padding_left(12.0).padding_right(4.0).child(crate::logo::logo(20.0)))
        .control(name)
        .control(dot)
        .control(div().grow(1.0))
        .control(layout)
        .control(tooltip("Outline (Ctrl+Shift+O)", outline))
        .control(tooltip("Open (Ctrl+O)", open))
        .control(tooltip("Save (Ctrl+S)", save))
        .control(div().padding_right(8.0).child(tooltip("Settings (Ctrl+,)", settings)))
}
