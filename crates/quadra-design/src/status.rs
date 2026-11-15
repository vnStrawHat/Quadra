//! The status of a workload (specs/behaviour/15, "Accent and feedback"): each has a colour
//! and a shape of its own, so it reads without colour too.

use quadra_layout::{Align, Justify};
use quadra_tree::Token;
use quadra_view::{AnyView, Div, div, row, text};

use crate::icon::icon;
use crate::{icons, token};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Status {
    Running,
    Pending,
    Failed,
    Succeeded,
    Terminating,
    #[default]
    Unknown,
}

impl Status {
    /// The status a word names (`Running`, `Completed`, `CrashLoopBackOff`); any other is
    /// unknown.
    pub fn of(word: &str) -> Status {
        match word {
            "Running" => Status::Running,
            "Pending" => Status::Pending,
            "Failed" | "Error" | "CrashLoopBackOff" => Status::Failed,
            "Succeeded" | "Completed" => Status::Succeeded,
            "Terminating" => Status::Terminating,
            _ => Status::Unknown,
        }
    }

    pub fn name(self) -> &'static str {
        ["Running", "Pending", "Failed", "Succeeded", "Terminating", "Unknown"][self as usize]
    }

    /// The role of the theme that is the status's colour.
    pub fn color(self) -> Token {
        use token::*;
        [RUNNING, PENDING, FAILED, SUCCEEDED, TERMINATING, UNKNOWN][self as usize]
    }
}

/// The shape of a status in a box of 12 px: a dot, a ring, a triangle, a check, a dash, a
/// square.
pub fn status_mark(status: Status) -> Div {
    let tone = status.color();
    let mark = match status {
        Status::Running => AnyView::new(div().size(8.0, 8.0).radius(4.0).bg(tone)),
        Status::Pending => AnyView::new(div().size(8.0, 8.0).radius(4.0).border(2.0).border_color(tone)),
        Status::Failed => AnyView::new(icon(icons::TRIANGLE_ALERT).size(12.0).color(tone)),
        Status::Succeeded => AnyView::new(icon(icons::CHECK).size(12.0).color(tone)),
        Status::Terminating => AnyView::new(div().size(8.0, 2.0).radius(1.0).bg(tone)),
        Status::Unknown => AnyView::new(div().size(7.0, 7.0).radius(1.0).bg(tone)),
    };
    let around = row().align_items(Align::Center).justify(Justify::Center).shrink(0.0).size(12.0, 12.0);
    around.child(mark)
}

/// A status as its mark and its name, in its colour. `words` takes the name's place unless
/// it is empty.
pub fn status_label(status: Status, words: &str) -> Div {
    let words = if words.is_empty() { status.name() } else { words };
    let line = row().align_items(Align::Center).gap(6.0).child(status_mark(status));
    line.child(text(words.to_string()).color(status.color()))
}
