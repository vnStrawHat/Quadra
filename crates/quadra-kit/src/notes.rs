//! What says something in the flow of a page (specs/behaviour/09, "Display"): an alert, what
//! an empty place shows, and terms with their values.

use quadra_layout::{Align, Justify, Track};
use quadra_tree::Role;
use quadra_ui::{AnyView, Div, View, column, div, row, text};

use crate::display::Tone;
use crate::icon::icon;
use crate::{icons, size, token};

impl Tone {
    /// The icon that goes with a tone.
    pub(crate) fn icon(self) -> &'static str {
        match self {
            Tone::Neutral | Tone::Hue(_) => icons::BELL,
            Tone::Primary | Tone::Info => icons::INFO,
            Tone::Success => icons::CIRCLE_CHECK,
            Tone::Warning => icons::TRIANGLE_ALERT,
            Tone::Danger => icons::CIRCLE_X,
        }
    }
}

/// A message in the flow of the page, tinted by its tone: its icon, a title, and a smaller
/// muted line that says more. Either of the two may be empty.
pub fn alert(tone: Tone, title: &str, more: &str) -> Div {
    let role = tone.role();
    let mut words = column().gap(2.0).grow(1.0);
    if !title.is_empty() {
        words = words.child(text(title.to_string()));
    }
    if !more.is_empty() {
        words = words.child(text(more.to_string()).size(size::TEXT_SM.size).color(token::TEXT_2));
    }
    let mark = row().height(23.0).align_items(Align::Center).child(icon(tone.icon()).color(role));
    let note = row().align_items(Align::Start).gap(8.0).padding(8.0).radius(size::RADIUS_MD);
    let note = note.bg(role.alpha(0.1)).border(1.0).border_color(role.alpha(0.2));
    let name = if title.is_empty() { more } else { title };
    note.role(Role::Group).label(name.to_string()).child(mark).child(words)
}

/// What an empty place shows: a muted icon, why it is empty, a hint, and something to do
/// about it.
pub fn empty_state(body: &'static str, title: &str, hint: &str, action: impl View + 'static) -> Div {
    let all = column().align_items(Align::Center).gap(4.0).padding(32.0).grow(1.0);
    let all = all.child(icon(body).size(16.0).color(token::TEXT_3)).child(text(title.to_string()));
    let all = match hint.is_empty() {
        true => all,
        false => all.child(text(hint.to_string()).size(size::TEXT_SM.size).color(token::TEXT_2)),
    };
    all.child(row().justify(Justify::Center).padding_top(8.0).child(action))
}

/// Terms and their values in two columns: the terms muted and as wide as the widest, the
/// values in the room that is left.
pub fn description_list<S: Into<String>>(items: impl IntoIterator<Item = (S, AnyView)>) -> Div {
    let mut grid = div().grid([Track::max_content(), Track::fr(1.0)]).gap_x(24.0).gap_y(8.0);
    for (term, value) in items {
        grid = grid.child(text(term.into()).color(token::TEXT_2)).child(value);
    }
    grid
}
