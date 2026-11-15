//! What opens above the rest: a select with its menu, and a dialog.

use std::rc::Rc;

use quadra::{Along, Div, NodeRef, Place, Side, Signal, State, column, div, row, signal, text};
use quadra_headless::menu::{self, Ends, Line, Row};
use quadra_headless::{dialog as base, popup};

use crate::{HIGH, HOVER, INK, PAPER, button, framed, ring};

/// The plane of a popup: paper in a frame of ink.
fn plane() -> Div {
    column().border(1.0).border_color(INK).bg(PAPER)
}

/// One of `options`, chosen from a menu under the button that shows it.
pub fn select(options: &[&str], chosen: Signal<usize>) -> Div {
    let (open, opener) = (signal(false), NodeRef::new());
    let options: Rc<Vec<String>> = Rc::new(options.iter().map(|name| name.to_string()).collect());
    let names = options.clone();
    let says = move || names.get(chosen.get()).cloned().unwrap_or_default();
    let face = framed().width(160.0).height(HIGH).padding_x(8.0).gap(8.0).cursor(quadra::Cursor::Pointer);
    let face = face.hover(|s| s.bg(HOVER)).focus_visible(ring).node_ref(&opener).label(says.clone());
    let face = face.child(div().grow(1.0).child(text(says))).child(text("▾"));
    let face = quadra_headless::button(face)
        .on_click(move || open.update(|open| *open = !*open))
        .build()
        .expanded(open);
    let rows = move || {
        let on = chosen.get_untracked();
        let line = |(i, name): (usize, &String)| {
            let face = row()
                .height(24.0)
                .padding_x(8.0)
                .align_items(quadra::Align::Center)
                .child(text(name.clone()));
            let face = face.hover(|s| s.bg(HOVER)).when(State::SELECTED, |s| s.bg(HOVER));
            let mut row = Row::new(face, name.clone());
            let pick: Act = Rc::new(move || chosen.set(i));
            (row.checked, row.select) = (Some(i == on), Some(pick));
            Line::Row(Box::new(row))
        };
        let ends = Ends { done: Rc::new(move || open.set(false)), leave: None };
        menu::rows(column().width(160.0), options.iter().enumerate().map(line).collect(), ends)
    };
    let place = Place::Beside(opener.clone(), Side::Below, Along::Start, 2.0);
    column().child(face).child(popup::popup(open, place, Some(opener), plane, rows))
}

/// What an answer does.
pub type Act = Rc<dyn Fn()>;

/// A dialog under the top of the window while `open`: a title, what it says, and its answers
/// (a word and what it does), with a cross that gives it up.
pub fn dialog(open: Signal<bool>, title: &str, says: &str, answers: Vec<(&str, Act)>) -> impl quadra::View {
    let (title, says) = (title.to_string(), says.to_string());
    let answers: Rc<Vec<(String, Act)>> =
        Rc::new(answers.into_iter().map(|(word, act)| (word.to_string(), act)).collect());
    let content = move || {
        let cross = framed().size(20.0, 20.0).justify(quadra::Justify::Center).hover(|s| s.bg(HOVER));
        let cross = base::close(cross.child(text("×")), open);
        let head = row().align_items(quadra::Align::Center).child(div().grow(1.0).child(text(title.clone())));
        let foot = row().gap(8.0).justify(quadra::Justify::End);
        let foot =
            foot.children(answers.iter().map(|(word, act)| button(word, base::answer(open, act.clone()))));
        let page = column().width(320.0).padding(16.0).gap(12.0);
        base::page(page, title.clone()).child(head.child(cross)).child(text(says.clone())).child(foot)
    };
    popup::popup(open, Place::Top(80.0), None, plane, content)
}
