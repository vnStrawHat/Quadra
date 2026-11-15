//! What says something in the flow of a page (specs/behaviour/09, "Display"): an alert, what
//! an empty place shows, and terms with their values.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use quadra_async::{Timer, timeout};
use quadra_layout::{Align, Justify, Track};
use quadra_platform::{Clipboard, Cursor};
use quadra_reactive::{Scope, signal, use_context};
use quadra_tree::Role;
use quadra_view::{AnyView, Div, View, column, div, dynamic, row, text};

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
    let note = row().align_items(Align::Start).gap(8.0).padding(8.0).radius(token::RADIUS_MD);
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
/// values in the room that is left. A value that can be copied is [`copyable`].
pub fn description_list<S: Into<String>>(items: impl IntoIterator<Item = (S, AnyView)>) -> Div {
    let mut grid = div().grid([Track::max_content(), Track::fr(1.0)]).gap_x(24.0).gap_y(8.0);
    for (term, value) in items {
        grid = grid.child(text(term.into()).color(token::TEXT_2)).child(value);
    }
    grid
}

/// A value of a description list with a button at its end that copies it: an icon that is a
/// check, and a name that says "Copied", for a moment after a click. The clipboard is the app's (none in a view with none,
/// and then the button does nothing).
pub fn copyable(value: impl Into<String>) -> AnyView {
    let value: String = value.into();
    let (clipboard, owner) = (use_context::<Rc<dyn Clipboard>>(), Scope::current());
    let (done, waiting) = (signal(false), Rc::new(Cell::new(None::<Timer>)));
    let words = value.clone();
    let copy = move || {
        let Some(clipboard) = &clipboard else { return };
        clipboard.write_text(&words);
        done.set(true);
        let timer = owner.run(|| timeout(Duration::from_millis(1500), move || done.set(false)));
        if let Some(before) = waiting.replace(Some(timer)) {
            before.cancel();
        }
    };
    let mark = dynamic(move || {
        let (body, color) = if done.get() { (icons::CHECK, token::OK) } else { (icons::COPY, token::TEXT_3) };
        icon(body).size(12.0).color(color)
    });
    let button = row().size(20.0, 20.0).align_items(Align::Center).justify(Justify::Center).shrink(0.0);
    let button = button.radius(token::RADIUS_SM).cursor(Cursor::Pointer).hover(|s| s.bg(token::PANEL_HOVER));
    let button = button
        .role(Role::Button)
        .label(move || if done.get() { "Copied" } else { "Copy" }.to_string())
        .on_click(move |_| copy());
    let line = row().align_items(Align::Center).gap(4.0).min_width(0.0);
    AnyView::new(line.child(text(value).selectable()).child(button.child(mark)))
}
