//! What the pages are built from: the page itself, a framed demo, and a few rows of text.

use quadra::{Align, Div, TextView, View, column, div, row, text};
use quadra_kit::{fonts, heading, label, size, token};

use crate::registry::PAGES;

/// A page: the group it is in, its title, what the component is for, and its demos.
pub fn page(title: &str, about: &str, demos: impl IntoIterator<Item = Div>) -> Div {
    let group = PAGES.iter().find(|page| page.title == title).map_or("Overview", |page| page.group);
    let about = div().max_width(640.0).child(text(about.to_string()).color(token::TEXT_2).selectable());
    let head = column().gap(6.0).child(label(group)).child(heading(title, 1)).child(about);
    let body = column().gap(20.0).padding_x(32.0).padding_y(28.0).max_width(1000.0);
    body.child(head).children(demos.into_iter().collect::<Vec<_>>())
}

/// A framed demo: a strip that names it, the thing itself on the canvas, and how it is
/// written.
pub fn demo(title: &str, content: impl View + 'static, code: &str) -> Div {
    let strip = || row().align_items(Align::Center).padding_x(12.0).bg(token::PANEL);
    let head = strip().height(28.0).border_bottom(1.0).border_color(token::LINE).child(label(title));
    let body = column().padding(20.0).bg(token::CANVAS).child(content);
    let framed = column().border(1.0).border_color(token::LINE).child(head).child(body);
    if code.is_empty() {
        return framed;
    }
    let code = text(code.to_string()).size(size::TEXT_SM.size).color(token::TEXT_2).selectable();
    let foot = strip().padding_y(7.0).border_top(1.0).border_color(token::LINE).font_family(fonts::MONO);
    framed.child(foot.child(code))
}

/// Things side by side, 12 px apart, on as many lines as it takes.
pub fn line() -> Div {
    row().wrap(true).align_items(Align::Center).gap(12.0)
}

/// A line of small text that says less.
pub fn hint(words: impl Into<String>) -> TextView {
    text(words.into()).size(size::TEXT_SM.size).color(token::TEXT_3)
}

/// A value in the mono face.
pub fn mono(words: impl Into<String>) -> Div {
    div().font_family(fonts::MONO).child(text(words.into()).color(token::TEXT_2).selectable())
}
