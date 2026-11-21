//! What a log view is told: its time, text, ground, levels, wrapping, following, header,
//! empty row, a line's menu and notes, and its sources' colours.

use std::rc::Rc;

use quadra_design::MenuItem;
use quadra_reactive::Signal;
use quadra_tree::{Paint, Token};
use quadra_view::{IntoProp, Prop};

use super::LogView;

impl LogView {
    /// Whether the time of each line is before it; switched, the list stays where it is.
    pub fn timestamps<M>(mut self, shown: impl IntoProp<bool, M>) -> Self {
        match shown.into_prop() {
            Prop::Value(shown) => self.look.timestamps.set(shown),
            Prop::Reactive(shown) => self.stamps = Some(shown),
        }
        self
    }

    /// Whether a long line goes on to the rows under it, each line as tall as it needs.
    pub fn wrap<M>(mut self, wraps: impl IntoProp<bool, M>) -> Self {
        match wraps.into_prop() {
            Prop::Value(wraps) => (self.look.wrap.set(wraps), self.wrapping = wraps),
            Prop::Reactive(wraps) => (self.wraps = Some(wraps), self.wrapping = true),
        };
        self
    }

    /// Whether the bar is over the lines: an app with tools of its own filters the `Log`.
    pub fn bar(mut self, shown: bool) -> Self {
        self.bar = shown;
        self
    }

    /// The size of the text and the height of a row: 12 on 18 unless told.
    pub fn text(mut self, size: f32, row: f32) -> Self {
        (self.look.size, self.look.row) = (size, row);
        self
    }

    /// The lines on the canvas, a radius of 6, 6 by 12 px in.
    pub fn ground(mut self) -> Self {
        self.look.ground = true;
        self
    }

    /// The time to the millisecond (`09:12:41.208`, unless told), or to the second.
    pub fn millis(mut self, shown: bool) -> Self {
        self.look.millis = shown;
        self
    }

    pub fn time_color(mut self, color: Token) -> Self {
        self.look.time = color;
        self
    }

    /// The time shown at this offset from UTC, in seconds: the app's local time.
    pub fn offset(mut self, seconds: i64) -> Self {
        self.look.offset = seconds * 1000;
        self
    }

    /// A column with each line's level, and a warning's or an error's words in its colour.
    pub fn levels(mut self) -> Self {
        self.look.levels = true;
        self
    }

    /// Following, read and written: the app's switch turns it on and off, scrolling away
    /// turns it off and back to the end on.
    pub fn follow(mut self, on: Signal<bool>) -> Self {
        self.follow = Some(on);
        self
    }

    /// A first row that is no line: never filtered, selected or copied.
    pub fn header(mut self, words: impl Fn() -> String + 'static) -> Self {
        self.header = Some(Rc::new(words));
        self
    }

    /// What the one row says when the log has no line.
    pub fn empty(mut self, words: &str) -> Self {
        self.empty = Some(words.to_string());
        self
    }

    /// A menu at the pointer on a click with the other button on a line, the items for the
    /// line at that place; the line is tinted while it is open.
    pub fn line_menu(mut self, items: impl Fn(usize) -> Vec<MenuItem> + 'static) -> Self {
        self.menu = Some(Rc::new(items));
        self
    }

    /// A short word at the end of the line of a place, in the accent: `Some((place, "Copied"))`.
    pub fn notes(mut self, notes: Signal<Option<(usize, String)>>) -> Self {
        self.notes = Some(notes);
        self
    }

    /// The colour of each source's dot: the hues unless told.
    pub fn source_colors(mut self, color: impl Fn(u8) -> Paint + 'static) -> Self {
        self.colors = Rc::new(color);
        self
    }
}
