//! The status of a workload (specs/behaviour/15, "Accent and feedback"): each has a colour
//! and a shape of its own, so it reads without colour too.

use quadra_layout::{Align, Justify};
use quadra_tree::{Role, Token};
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
    /// No mark, its room kept: what needs no attention lines up with what does.
    None,
}

impl Status {
    /// The status a word names (`Running`, `Completed`), or a reason a container is in its
    /// state names (`OOMKilled`, `ImagePullBackOff`, `ContainerCreating`, `Init:0/2`); any
    /// other is unknown.
    pub fn of(word: &str) -> Status {
        match word {
            "Running" => Status::Running,
            "Pending" | "ContainerCreating" | "PodInitializing" => Status::Pending,
            "Failed" | "OOMKilled" | "Evicted" => Status::Failed,
            "Succeeded" | "Completed" => Status::Succeeded,
            "Terminating" => Status::Terminating,
            // `Init:0/2` counts the init containers done; `Init:Error` is the reason of one.
            _ if let Some(reason) = word.strip_prefix("Init:") => match reason.contains('/') {
                true => Status::Pending,
                false => Status::of(reason),
            },
            _ if word.ends_with("BackOff") || word.starts_with("Err") || word.ends_with("Error") => {
                Status::Failed
            }
            _ => Status::Unknown,
        }
    }

    pub fn name(self) -> &'static str {
        ["Running", "Pending", "Failed", "Succeeded", "Terminating", "Unknown", "None"][self as usize]
    }

    /// The role of the theme that is the status's colour.
    pub fn color(self) -> Token {
        use token::*;
        [RUNNING, PENDING, FAILED, SUCCEEDED, TERMINATING, UNKNOWN, UNKNOWN][self as usize]
    }
}

/// A filled triangle and a filled diamond, in the 24 units of an icon.
const TRIANGLE: &str = r#"<path d="M12 3.5 21.5 20H2.5Z" fill="currentColor" stroke="none"/>"#;
const DIAMOND: &str = r#"<path d="M12 2.5 21.5 12 12 21.5 2.5 12Z" fill="currentColor" stroke="none"/>"#;

/// The shape of a status in a box of 12 px: a filled square, a hollow one, a filled
/// triangle, a check, a dash, a filled diamond; nothing for `None`.
pub fn status_mark(status: Status) -> Div {
    status_mark_sized(status, 12.0)
}

/// A status's mark in a box of `side` px, its shape scaled with it.
pub fn status_mark_sized(status: Status, side: f32) -> Div {
    let (tone, k) = (status.color(), side / 12.0);
    let square = || div().size(7.0 * k, 7.0 * k).radius(k);
    let around = row().align_items(Align::Center).justify(Justify::Center).shrink(0.0).size(side, side);
    let mark = match status {
        Status::Running => AnyView::new(square().bg(tone)),
        Status::Pending => AnyView::new(square().border(2.0 * k).border_color(tone)),
        Status::Failed => AnyView::new(icon(TRIANGLE).size(side).color(tone)),
        Status::Succeeded => AnyView::new(icon(icons::CHECK).size(side).color(tone)),
        Status::Terminating => AnyView::new(div().size(8.0 * k, 2.0 * k).radius(k).bg(tone)),
        Status::Unknown => AnyView::new(icon(DIAMOND).size(side).color(tone)),
        Status::None => return around,
    };
    around.child(mark)
}

/// A status as its mark and its name, in its colour, on one line that ends with "…" where it
/// does not fit. `words` takes the name's place unless it is empty.
pub fn status_label(status: Status, words: &str) -> Div {
    status_label_sized(status, words, 12.0)
}

/// A status label whose mark is `side` px.
pub fn status_label_sized(status: Status, words: &str, side: f32) -> Div {
    let words = if words.is_empty() { status.name() } else { words };
    let mark = status_mark_sized(status, side);
    let line = row().align_items(Align::Center).gap(6.0).min_width(0.0).child(mark);
    line.child(text(words.to_string()).color(status.color()).ellipsis())
}

/// A row of marks of `side` px, `side / 2.5` apart: the last runs of a job, the pods of a
/// workload. To a reader an image named by how many there are of each ("8 succeeded, 2
/// failed").
pub fn status_dots(statuses: impl IntoIterator<Item = Status>, side: f32) -> Div {
    let statuses: Vec<Status> = statuses.into_iter().collect();
    let mut counts: Vec<(Status, usize)> = Vec::new();
    for &status in statuses.iter().filter(|&&s| s != Status::None) {
        match counts.iter_mut().find(|(s, _)| *s == status) {
            Some((_, n)) => *n += 1,
            None => counts.push((status, 1)),
        }
    }
    let said: Vec<String> = counts.iter().map(|(s, n)| format!("{n} {}", s.name().to_lowercase())).collect();
    let line = row().align_items(Align::Center).gap((side / 2.5).round()).shrink(0.0);
    let line = line.role(Role::Image).label(said.join(", "));
    line.children(statuses.into_iter().map(|status| status_mark_sized(status, side)))
}
