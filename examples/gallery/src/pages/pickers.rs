//! What is chosen in a popover or a dialog: the colour picker, and the file and folder pickers.

use std::cell::Cell;
use std::path::PathBuf;

use quadra::{Div, signal};
use quadra_design::{color_picker, dark, file_picker, folder_picker, token};

use crate::common::{demo, hint, line, page, stack};

pub fn color_picker_page() -> Div {
    let (accent, tag, cursor) = (signal("#61afef".to_string()), signal(String::new()), signal(String::new()));
    // The colours of the tags, from the theme's roles: a preset is a colour.
    let theme = dark();
    let tags = [
        ("Red", token::HUE_RED),
        ("Orange", token::HUE_ORANGE),
        ("Yellow", token::HUE_YELLOW),
        ("Green", token::HUE_GREEN),
        ("Cyan", token::HUE_CYAN),
        ("Blue", token::HUE_BLUE),
        ("Purple", token::HUE_PURPLE),
    ];
    let presets = tags.map(|(name, role)| (name, theme.color(role)));
    let set = line()
        .child(color_picker(accent).label("Accent").presets(presets).test_id("cp-accent"))
        .child(hint(move || accent.get()));
    let host = line()
        .child(
            color_picker(tag)
                .label("Host tag")
                .fallback(token::HUE_GREEN)
                .placeholder("Group's")
                .presets(presets),
        )
        .child(hint(
            move || if tag.get().is_empty() { "empty: the group's tag".to_string() } else { tag.get() },
        ));
    let fallback = line()
        .child(color_picker(cursor).label("Cursor").fallback(token::EDITOR_CURSOR).placeholder("Theme's"))
        .child(hint("empty: the theme's cursor"));
    page(
        "Color picker",
        "A colour chosen by its saturation and brightness, its hue, its hex or a swatch, in a popover. \
         Empty is the default: the swatch shows the colour that stands instead.",
        [
            demo("A colour and presets", set, "color_picker(accent).presets(tags)"),
            demo("The default and presets", host, "color_picker(tag).fallback(..).presets(tags)"),
            demo("The theme's colour", fallback, "color_picker(cursor).fallback(token::EDITOR_CURSOR)"),
        ],
    )
}

/// What the gallery's "Browse…" answers in place of the system's dialog: sample paths in turn,
/// so the page never opens a real one.
fn sample(paths: &'static [&'static str]) -> impl Fn(&str) -> Option<PathBuf> {
    let next = Cell::new(0);
    move |_| {
        let at = next.get();
        next.set((at + 1) % paths.len());
        Some(PathBuf::from(paths[at]))
    }
}

pub fn file_picker_page() -> Div {
    let (key, logs) = (signal(String::new()), signal("D:\\work\\session-logs".to_string()));
    let keys = &["C:\\Users\\me\\.ssh\\id_ed25519", "C:\\Users\\me\\.ssh\\id_rsa"];
    let file = stack()
        .child(
            file_picker(key, sample(keys))
                .placeholder("~/.ssh/id_ed25519")
                .label("Identity file")
                .width(320.0),
        )
        .child(hint(move || format!("value: {}", key.get())));
    let folder =
        folder_picker(logs, sample(&["D:\\work\\session-logs", "E:\\logs"])).label("Logs").width(320.0);
    page(
        "File and folder pickers",
        "A path typed, or chosen by Browse… in the system's dialog, which the app gives the picker. \
         Here it answers with sample paths.",
        [
            demo("A file", file, "file_picker(key, move |_| dialogs.open_files(&ask).pop())"),
            demo("A folder", folder, "folder_picker(logs, move |at| dialogs.pick_folder(&ask))"),
        ],
    )
}
