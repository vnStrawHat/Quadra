//! What the pages are built from: the page itself, a framed demo, and a few rows of text.

use quadra::{Align, Div, IntoProp, TextView, View, column, div, row, text};
use quadra_design::{fonts, heading, label, size, token};

use crate::registry::PAGES;

thread_local! {
    /// What the pages built since `notes` was last called said of themselves: kept only once
    /// it has been called, for the site (`tools/website.py`).
    static NOTES: std::cell::RefCell<Option<Vec<(String, String)>>> = const { std::cell::RefCell::new(None) };
}

fn note(what: &str, words: &str) {
    NOTES.with_borrow_mut(|notes| notes.iter_mut().for_each(|notes| notes.push((what.into(), words.into()))));
}

/// Takes what the pages built since the last call said: `("", about)` for a page, then
/// `(title, code)` for each of its demos.
pub fn notes() -> Vec<(String, String)> {
    NOTES.with_borrow_mut(|notes| notes.replace(Vec::new()).unwrap_or_default())
}

/// A page: the group it is in, its title, what the component is for, and its demos.
pub fn page(title: &str, about: &str, demos: impl IntoIterator<Item = Div>) -> Div {
    let group = PAGES.iter().find(|page| page.title == title).map_or("Overview", |page| page.group);
    note("", about);
    let about = div().max_width(640.0).child(text(about.to_string()).color(token::TEXT_2).selectable());
    let head = column().gap(6.0).child(label(group)).child(heading(title, 1)).child(about);
    let body = column().gap(24.0).padding_x(32.0).padding_y(24.0).max_width(1000.0);
    body.child(head).children(demos.into_iter().collect::<Vec<_>>())
}

/// A framed demo: a strip that names it, the thing itself on the canvas, and how it is
/// written.
pub fn demo(title: &str, content: impl View + 'static, code: &str) -> Div {
    note(title, code);
    let strip = || row().align_items(Align::Center).padding_x(12.0).bg(token::PANEL);
    let head = strip().height(28.0).border_bottom(1.0).border_color(token::LINE).child(label(title));
    let body = column().padding(16.0).bg(token::CANVAS).child(content);
    let framed = column().border(1.0).border_color(token::LINE).child(head).child(body);
    if code.is_empty() {
        return framed;
    }
    let code = text(code.to_string()).size(size::TEXT_SM.size).color(token::TEXT_2).selectable();
    let foot = strip().padding_y(8.0).border_top(1.0).border_color(token::LINE).font_family(fonts::MONO);
    framed.child(foot.child(code))
}

/// Things side by side, 12 px apart, on as many lines as it takes.
pub fn line() -> Div {
    row().wrap(true).align_items(Align::Center).gap(12.0)
}

/// Things one under another, 12 px apart.
pub fn stack() -> Div {
    column().gap(12.0)
}

/// A row of settings: what it sets and a line about it at the left, its control at the right.
pub fn setting(title: &str, about: &str, control: impl View + 'static) -> Div {
    let words = column().gap(2.0).grow(1.0).child(text(title.to_string())).child(hint(about));
    let item = row().align_items(Align::Center).gap(16.0).padding_y(8.0);
    item.border_bottom(1.0).border_color(token::LINE).child(words).child(control)
}

/// A box with a hairline edge, for demos that need one: panes, lists, menus in place.
pub fn framed(content: impl View + 'static) -> Div {
    use quadra::Overflow;
    let frame = column().border(1.0).border_color(token::LINE);
    frame.overflow_x(Overflow::Hidden).overflow_y(Overflow::Hidden).child(content)
}

/// A line of small text that says less: a constant, or something reactive.
pub fn hint<M>(words: impl IntoProp<String, M>) -> TextView {
    text(words).size(size::TEXT_SM.size).color(token::TEXT_3)
}

/// A value in the mono face: a constant, or something reactive.
pub fn mono<M>(words: impl IntoProp<String, M>) -> Div {
    div().font_family(fonts::MONO).child(text(words).color(token::TEXT_2).selectable())
}
