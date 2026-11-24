//! The toolbar (specs/21, "Toolbar"): buttons that format the text and insert blocks, each
//! one change of the editor, the focus back in it after.

use quadra::{Action, Div};
use quadra_code::Edit;
use quadra_design::{Size, icon_button, icons, tooltip};

use crate::format::{Format, format};
use crate::model::Model;

/// Applies a format to the editor's selection (the toolbar's keys).
#[derive(Clone, Copy, Debug)]
pub struct ApplyFormat(pub Format);

impl Action for ApplyFormat {
    const NAME: &'static str = "markdown::Format";
}

/// The buttons: icon, name, key, format, in their groups.
type Button = (&'static str, &'static str, &'static str, Format);

const GROUPS: [&[Button]; 4] = [
    &[
        (icons::BOLD, "Bold", "Ctrl+B", Format::Bold),
        (icons::ITALIC, "Italic", "Ctrl+I", Format::Italic),
        (icons::STRIKETHROUGH, "Strikethrough", "Ctrl+Shift+X", Format::Strike),
        (icons::CODE, "Inline code", "Ctrl+E", Format::Code),
    ],
    &[
        (icons::HEADING_1, "Heading 1", "Ctrl+Alt+1", Format::Heading(1)),
        (icons::HEADING_2, "Heading 2", "Ctrl+Alt+2", Format::Heading(2)),
        (icons::HEADING_3, "Heading 3", "Ctrl+Alt+3", Format::Heading(3)),
    ],
    &[
        (icons::LIST, "Bulleted list", "Ctrl+Shift+8", Format::Bullet),
        (icons::LIST_ORDERED, "Numbered list", "Ctrl+Shift+7", Format::Numbered),
        (icons::LIST_TODO, "Task list", "", Format::Task),
        (icons::TEXT_QUOTE, "Quote", "Ctrl+Shift+.", Format::Quote),
    ],
    &[
        (icons::LINK, "Link", "Ctrl+K", Format::Link),
        (icons::IMAGE, "Image", "", Format::Image),
        (icons::SQUARE_CODE, "Code block", "", Format::CodeBlock),
        (icons::TABLE, "Table", "", Format::Table),
        (icons::MINUS, "Horizontal rule", "", Format::Rule),
    ],
];

/// The keys of the buttons that have them, as a keymap writes them.
pub(crate) fn keys() -> impl Iterator<Item = (String, Format)> {
    GROUPS.iter().flat_map(|group| group.iter()).filter(|b| !b.2.is_empty()).map(|&(_, _, key, what)| {
        let stroke = key.to_lowercase().replace('+', "-");
        (stroke, what)
    })
}

/// Makes `what` of the editor's selection as one change, then gives it the focus back.
pub(crate) fn apply(m: &Model, what: Format) {
    m.editor.edit(move |w| {
        let (text, sel) = (w.doc.text(), w.doc.primary().range());
        let change = format(&text, sel, what);
        let edits = change.edits.into_iter().map(|(range, text)| Edit { range, text, caret: None }).collect();
        w.doc.apply(edits, false);
        w.doc.set_cursor(change.selection.start, change.selection.end, false);
    });
    m.editor.focus();
}

pub(crate) fn toolbar(m: &Model) -> Div {
    let mut bar = quadra_design::toolbar().role(quadra::Role::Group).label("Format");
    for (n, group) in GROUPS.iter().enumerate() {
        if n > 0 {
            bar = bar.child(quadra_design::vdivider());
        }
        for &(svg, name, key, what) in group.iter() {
            let tip = if key.is_empty() { name.to_string() } else { format!("{name} ({key})") };
            let m = m.clone();
            bar = bar
                .child(tooltip(tip, icon_button(svg, name).size(Size::Md).on_click(move || apply(&m, what))));
        }
    }
    bar
}
