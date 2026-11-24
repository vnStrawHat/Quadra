//! The outline (specs/21, "Outline"): a sidebar of the document's headings, nested by level,
//! the row of the section at the top selected, a row chosen going to its heading.

use std::collections::HashSet;

use quadra::{Display, Div, Memo, Scope, column, div, effect, memo, signal, text, untrack};
use quadra_design::{TreeNode, token, tree};
use quadra_md::Heading;

use crate::model::{Model, PREVIEW, RAW};
use crate::sync::{self, Driver, PAD};

/// The sidebar's width.
pub(crate) const WIDTH: f32 = 240.0;

/// The headings nested: a heading holds those of deeper levels after it, up to the next of
/// its level or higher. A row is known by its heading's source line.
pub(crate) fn nest(headings: &[Heading]) -> Vec<TreeNode> {
    fn take(all: &[Heading], i: &mut usize, above: u8) -> Vec<TreeNode> {
        let mut out = Vec::new();
        while let Some(it) = all.get(*i).filter(|it| it.level > above) {
            *i += 1;
            let inside = take(all, i, it.level);
            out.push(TreeNode::new(it.line.to_string(), it.text.clone()).children(inside));
        }
        out
    }
    take(headings, &mut 0, 0)
}

/// The heading of the section at `line`: the last at or above it.
pub(crate) fn section(headings: &[Heading], line: f64) -> Option<usize> {
    // A heading a hair below the top, as where a pane was sent puts it, is at the top.
    headings.iter().rev().find(|it| it.line as f64 <= line + 0.05).map(|it| it.line)
}

/// The sidebar, shown while `m.outline` is.
pub(crate) fn outline(m: &Model, headings: Memo<Vec<Heading>>) -> Div {
    let all = sync::mapping(m, headings);
    let (layout, pos, y) = (m.layout, m.pos, m.preview_y);
    let reading = move || match layout.get() {
        RAW => pos.get(),
        _ => all.with(|all| sync::line_of(all, y.get())),
    };
    let here = memo(move || headings.with(|h| section(h, reading())));
    // The row chosen holds until the user moves a pane: a heading in the last screenful is
    // not at the top when the pane stops at its end.
    let (chosen, held, touched) = (signal(None::<String>), signal(None::<usize>), m.touched);
    effect(move || {
        touched.get();
        held.set(None);
    });
    // The panes move: the row of the section at the top is selected.
    effect(move || {
        let now = held.get().or(here.get()).map(|line| line.to_string());
        if untrack(|| chosen.get()) != now {
            chosen.set(now);
        }
    });
    // A row chosen that is not where the panes are: they go to its heading.
    let (goes, scope) = (m.clone(), Scope::current());
    effect(move || {
        let Some(line) = chosen.get().and_then(|id| id.parse::<usize>().ok()) else { return };
        if untrack(|| held.get().or(here.get())) != Some(line) {
            held.set(Some(line));
            untrack(|| go(&goes, line, scope));
        }
    });
    // Every heading that holds others is open when the headings change.
    let open = signal(HashSet::new());
    effect(move || {
        let ids = headings.with(|h| {
            h.windows(2).filter(|w| w[1].level > w[0].level).map(|w| w[0].line.to_string()).collect()
        });
        open.set(ids);
    });
    let none = move || if headings.with(Vec::is_empty) { Display::Flex } else { Display::None };
    let empty =
        div().padding(token::SPACE_LG).text_color(token::TEXT_3).display(none).child(text("No headings"));
    let rows = tree(move || headings.with(|h| nest(h)), chosen).open(open);
    let shown = m.outline;
    column()
        .width(WIDTH)
        .shrink(0.0)
        .min_height(0.0)
        .bg(token::PANEL)
        .border_right(1.0)
        .border_color(token::LINE_STRONG)
        .role(quadra::Role::Group)
        .label("Outline")
        .display(move || if shown.get() { Display::Flex } else { Display::None })
        .child(empty)
        .child(rows)
}

/// Goes to the heading of `line` in each pane that shows: the editor's caret to its line and
/// the line to its top, the heading to the preview's top, tinted.
fn go(m: &Model, line: usize, scope: Scope) {
    let layout = m.layout.get_untracked();
    if layout != PREVIEW {
        let (driver, pos) = (m.driver, m.pos);
        m.editor.edit(move |w| {
            let at = w.doc.buffer.line_start(line);
            w.doc.set_cursor(at, at, false);
            driver.set(Driver::Editor);
            pos.set(line as f64);
        });
    }
    if layout != RAW {
        let top = m.places.with_untracked(|p| p.iter().find(|it| it.line == line).map(|it| it.top + PAD));
        if let Some(top) = top {
            m.preview_y.set(top);
        }
        sync::tint(m, line, scope);
    }
}
