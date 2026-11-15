//! The text input views (specs/04, "Widgets"; behaviour/09): text the user edits, on one line
//! or on several, showing a signal and writing it.

mod mount;

use std::time::Duration;

use quadra_reactive::Signal;
use quadra_tree::{NodeId, Paint, TextField, TextValues, Tree};

/// How long the caret is shown, and then hidden.
const CARET_BLINK: Duration = Duration::from_millis(530);

pub struct TextInput {
    value: Signal<String>,
    /// What the text says of itself; the rest is as the boxes around it say.
    values: TextValues,
    color: Option<Paint>,
    masked: bool,
    multiline: bool,
    fit: bool,
    placeholder: String,
    test_id: Option<&'static str>,
    submit: Option<Submit>,
}

/// What runs with the text when Enter is pressed.
type Submit = Box<dyn FnMut(&str)>;

/// A line of text that edits `value`: what is typed is written to it, and what is written to
/// it from elsewhere is shown. It takes the focus on a click or by Tab; the caret, the
/// selection and undo are its own.
pub fn text_input(value: Signal<String>) -> TextInput {
    let (values, color) = (TextValues::default(), None);
    let placeholder = String::new();
    let (masked, multiline) = (false, false);
    let fit = false;
    TextInput { value, values, color, masked, multiline, fit, placeholder, test_id: None, submit: None }
}

/// Text on as many lines as it takes at the width the area is given: Enter starts a line, and
/// the caret moves up and down keeping its place across the lines. The area is as tall as
/// its text; in a scroll view, the caret is kept in view. The rest is as [`text_input`].
pub fn text_area(value: Signal<String>) -> TextInput {
    TextInput { multiline: true, ..text_input(value) }
}

impl TextInput {
    /// The font size in pixels.
    pub fn size(mut self, px: f32) -> Self {
        self.values.size = Some(px);
        self
    }

    /// A colour, or a role of the theme.
    pub fn color(mut self, color: impl Into<Paint>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// A name that does not change with the language or the content, for tests and tools.
    pub fn test_id(mut self, id: &'static str) -> Self {
        self.test_id = Some(id);
        self
    }

    /// What stands in the input while it holds nothing: what to type there.
    pub fn placeholder(mut self, text: &str) -> Self {
        self.placeholder = text.to_string();
        self
    }

    /// A password: a dot shows for each character, and copy and cut do nothing.
    pub fn password(mut self) -> Self {
        self.masked = true;
        self
    }

    /// As wide as its text, not as the room it is given: a box around it can centre it.
    pub fn fit(mut self) -> Self {
        self.fit = true;
        self
    }

    /// Runs `f` with the value when Enter is pressed in the input; Shift-Enter is left to
    /// what is around it.
    pub fn on_submit(mut self, f: impl FnMut(&str) + 'static) -> Self {
        self.submit = Some(Box::new(f));
        self
    }
}

/// Text as a field takes it: its line breaks are line feeds where it has several lines, and
/// spaces where it has one.
fn lines_of(text: &str, multiline: bool) -> String {
    let breaks = if multiline { "\n" } else { " " };
    text.replace("\r\n", breaks).replace(['\r', '\n'], breaks)
}

/// Changes the field in place. `f` says whether it changed the text: then the value is
/// written and the text shaped again; in any case the field is painted again.
fn edit(tree: &mut Tree, id: NodeId, value: Signal<String>, f: impl FnOnce(&mut TextField) -> bool) {
    let fonts = tree.fonts();
    let Some(field) = tree.widget_mut::<TextField>(id) else { return };
    // The lines are those of the text as it is now: an edit before this one, in the same
    // turn, has not been laid out yet.
    field.sync(&mut fonts.borrow_mut().text);
    let changed = f(field).then(|| field.buffer.text().to_string());
    (field.caret_on, field.touched) = (true, true); // a caret in use does not blink away
    let follows = field.multiline;
    if let Some(text) = changed {
        tree.mark_layout(id);
        value.set(text);
    }
    tree.mark_paint(id);
    tree.update_semantics(id, |_| {}); // the caret and the selection, for a reader
    if follows {
        // Whatever scrolls around the area keeps the caret in view, once it is laid out.
        tree.after_layout(move |tree| {
            let caret = tree.widget_mut::<TextField>(id).and_then(|field| field.caret_rect());
            if let Some(caret) = caret {
                tree.scroll_rect_into_view(id, caret);
            }
        });
    }
}
