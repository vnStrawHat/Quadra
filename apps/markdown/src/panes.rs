//! The document in its layout (specs/21, "Layouts"): the editor, the preview, or both side by
//! side.

use std::rc::Rc;

use quadra::{AnyView, Div, Justify, Role, View, column, dynamic, row, scroll, signal, untrack};
use quadra_code::editor;
use quadra_design::{split, token};
use quadra_md::markdown;

use crate::model::{Model, RAW, SIDE_BY_SIDE};

/// The preview's text column at most.
const COLUMN: f32 = 760.0;

/// What a link does with its target, Ctrl held.
pub(crate) type Follow = Rc<dyn Fn(&str)>;

/// The layout chosen, built again when it changes or another document opens.
pub(crate) fn body(model: &Model, follow: Follow) -> Div {
    let m = model.clone();
    let shown = dynamic(move || {
        m.opened.get();
        match m.layout.get() {
            RAW => AnyView::new(source(&m, &follow)),
            SIDE_BY_SIDE => AnyView::new(split(source(&m, &follow), preview(&m, &follow), m.share)),
            _ => AnyView::new(preview(&m, &follow)),
        }
    });
    column().grow(1.0).min_height(0.0).child(shown.container(column().grow(1.0).min_height(0.0)))
}

/// The editor over the text: prose wrapped, coloured as Markdown, its changes marked against
/// the text saved, its document kept while it is not shown.
fn source(m: &Model, follow: &Follow) -> Div {
    // The marks compare lines without their ends.
    let saved = untrack(|| m.saved.get()).replace("\r\n", "\n");
    let (text, follow) = (untrack(|| m.text.get()), follow.clone());
    let code = editor(text)
        .path("document.md")
        .wrap(true)
        .font(14.0, 22.0)
        .base(saved)
        .changed(m.changed)
        .edited(m.edited)
        .handle(m.editor.clone())
        .keep(m.doc.clone())
        .on_link(move |target| follow(target));
    column().grow(1.0).min_width(0.0).min_height(0.0).role(Role::Group).label("Source").child(code)
}

/// The document rendered, in a column of at most 760 px in the middle.
fn preview(m: &Model, follow: &Follow) -> impl View + use<> {
    let (text, reveal, follow) = (m.text, m.reveal, follow.clone());
    let mut doc =
        markdown(move || text.get()).on_link(move |target| follow(target)).reveal(move || reveal.get());
    if let Some(folder) = untrack(|| m.folder()) {
        doc = doc.base(folder);
    }
    let page = row().justify(Justify::Center).padding_x(32.0).padding_y(24.0);
    let page = page.child(column().grow(1.0).min_width(0.0).max_width(COLUMN).child(doc));
    let pane = column().grow(1.0).min_width(0.0).min_height(0.0).bg(token::CANVAS);
    pane.role(Role::Group).label("Rendered").child(scroll(signal(0.0), page))
}
