//! The document in its layout (specs/21, "Layouts"): the editor, the preview, or both side by
//! side.

use quadra::{AnyView, Div, Justify, Role, View, column, dynamic, row, scroll, signal, untrack};
use quadra_code::editor;
use quadra_design::{split, token};
use quadra_md::markdown;

use crate::model::{Model, RAW, SIDE_BY_SIDE};

/// The preview's text column at most.
const COLUMN: f32 = 760.0;

/// The layout chosen, built again when it changes or another document opens.
pub(crate) fn body(model: &Model) -> Div {
    let m = model.clone();
    let shown = dynamic(move || {
        m.opened.get();
        match m.layout.get() {
            RAW => AnyView::new(source(&m)),
            SIDE_BY_SIDE => AnyView::new(split(source(&m), preview(&m), m.share)),
            _ => AnyView::new(preview(&m)),
        }
    });
    column().grow(1.0).min_height(0.0).child(shown.container(column().grow(1.0).min_height(0.0)))
}

/// The editor over the text: prose wrapped, coloured as Markdown, its document kept while it
/// is not shown.
fn source(m: &Model) -> Div {
    let text = untrack(|| m.text.get());
    let code =
        editor(text).path("document.md").wrap(true).font(14.0, 22.0).changed(m.changed).keep(m.doc.clone());
    column().grow(1.0).min_width(0.0).min_height(0.0).role(Role::Group).label("Source").child(code)
}

/// The document rendered, in a column of at most 760 px in the middle.
fn preview(m: &Model) -> impl View + use<> {
    let text = m.text;
    let mut doc = markdown(move || text.get());
    if let Some(folder) = untrack(|| m.folder()) {
        doc = doc.base(folder);
    }
    let page = row().justify(Justify::Center).padding_x(32.0).padding_y(24.0);
    let page = page.child(column().grow(1.0).min_width(0.0).max_width(COLUMN).child(doc));
    let pane = column()
        .grow(1.0)
        .min_width(0.0)
        .min_height(0.0)
        .bg(token::CANVAS)
        .role(Role::Group)
        .label("Rendered");
    pane.child(scroll(signal(0.0), page))
}
