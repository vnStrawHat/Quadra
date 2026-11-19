//! The document in its layout (specs/21, "Layouts"): the editor, the preview, or both side by
//! side.

use std::rc::Rc;

use quadra::{
    AnyView, Display, Div, Justify, Length, PointerKind, Role, Scope, View, column, div, dynamic, row,
    scroll, untrack,
};
use quadra_code::editor;
use quadra_design::{split, token};
use quadra_md::markdown;

use crate::model::{Model, RAW, SIDE_BY_SIDE};
use crate::sync::{self, Driver, PAD};

/// What a link does with its target, Ctrl held.
pub(crate) type Follow = Rc<dyn Fn(&str)>;

/// The layout chosen, built again when it changes or another document opens.
pub(crate) fn body(model: &Model, follow: Follow) -> Div {
    let m = model.clone();
    let shown = dynamic(move || {
        m.opened.get();
        // The editor's face and wrapping: it is built again, its document kept.
        m.prefs.editor_font();
        m.prefs.wrap.get();
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
    let ((size, line), wrap) = untrack(|| (m.prefs.editor_font(), m.prefs.wrap.get()));
    let code = editor(text)
        .path("document.md")
        .wrap(wrap)
        .font(size, line)
        .base(saved)
        .changed(m.changed)
        .edited(m.edited)
        .handle(m.editor.clone())
        .keep(m.doc.clone())
        .position(m.pos)
        .caret_line(m.caret)
        .caret(m.place)
        .on_link(move |target| follow(target));
    let pane = column().grow(1.0).min_width(0.0).min_height(0.0).role(Role::Group).label("Source");
    drives(pane, m, Driver::Editor).child(code)
}

/// The document rendered, in a column of the width the settings say, in the middle.
// ponytail: every block is mounted, so the panes know where each heading is; a document of
// thousands of blocks would want `Markdown::scrolls` and the headings' places kept apart.
fn preview(m: &Model, follow: &Follow) -> impl View + use<> {
    let (text, reveal, follow, scope, chooses) =
        (m.text, m.reveal, follow.clone(), Scope::current(), m.clone());
    let doc = markdown(move || text.get()).on_link(move |target| follow(target)).reveal(move || reveal.get());
    let doc = doc.places(m.places).on_heading(move |line| sync::chosen(&chooses, line, scope));
    let mut doc = doc.find(m.search.find());
    if let Some(folder) = untrack(|| m.folder()) {
        doc = doc.base(folder);
    }
    let prefs = m.prefs;
    let widest = move || prefs.column().map_or(Length::Auto, Length::Px);
    let text = column().grow(1.0).min_width(0.0).max_width(widest).child(tint(m)).child(empty(m)).child(doc);
    let page = row().justify(Justify::Center).padding_x(32.0).padding_y(PAD).child(text);
    let pane = column().grow(1.0).min_width(0.0).min_height(0.0).bg(token::CANVAS);
    let pane = drives(pane.role(Role::Group).label("Rendered"), m, Driver::Preview);
    pane.child(scroll(m.preview_y, page)).child(crate::search::bar(&m.search))
}

/// What an empty document shows: the logo, and how to start.
fn empty(m: &Model) -> Div {
    let text = m.text;
    let words = "Open a file (Ctrl+O), drop one here, or start writing in Raw (Ctrl+1).";
    column()
        .align_items(quadra::Align::Center)
        .gap(16.0)
        .padding_top(96.0)
        .text_color(token::TEXT_3)
        .display(move || if text.with(String::is_empty) { Display::Flex } else { Display::None })
        .child(crate::logo::logo(96.0))
        .child(quadra::text(words))
}

/// A pane the wheel, a press or a key in it makes the one that drives.
fn drives(pane: Div, m: &Model, side: Driver) -> Div {
    let (driver, touched) = (m.driver, m.touched);
    let moved = move || {
        driver.set(side);
        touched.update(|n| *n += 1);
    };
    pane.on_mount(move |tree, id| {
        for kind in [PointerKind::Wheel, PointerKind::Down] {
            tree.on_pointer_capture(id, kind, move |_| moved());
        }
        tree.on_key_capture(id, true, move |_| moved());
    })
}

/// The heading chosen, tinted for a moment behind its text.
fn tint(m: &Model) -> Div {
    let (flash, places) = (m.flash, m.places);
    let place =
        move || flash.get().and_then(|line| places.with(|p| p.iter().find(|it| it.line == line).copied()));
    div()
        .absolute()
        .left(0.0)
        .right(0.0)
        .top(move || Length::Px(place().map_or(0.0, |it| it.top)))
        .height(move || Length::Px(place().map_or(0.0, |it| it.height)))
        .radius(4.0)
        .bg(token::SELECTION)
        .display(move || if place().is_some() { Display::Flex } else { Display::None })
}
