//! Forms (specs/behaviour/09, "Forms"): a control under its label, with a line of help or of
//! what is wrong; the rules a value is checked against; and the text area in its frame.

use quadra_layout::Align;
use quadra_platform::Cursor;
use quadra_reactive::{Signal, memo};
use quadra_tree::{NodeId, Role, State};
use quadra_view::{AnyView, IntoProp, MountCx, Prop, View, column, dynamic, row, text, text_area};

pub use quadra_headless::rules::{Rule, validate};

use crate::display::label;
use crate::icon::icon;
use crate::{icons, size, token};

pub struct Field {
    name: String,
    control: AnyView,
    help: String,
    error: Prop<String>,
    required: bool,
}

/// A control under its label.
pub fn field(name: impl Into<String>, control: impl View + 'static) -> Field {
    let (control, error) = (AnyView::new(control), Prop::Value(String::new()));
    Field { name: name.into(), control, help: String::new(), error, required: false }
}

impl Field {
    /// A line under the control that says what it is for.
    pub fn help(mut self, words: impl Into<String>) -> Self {
        self.help = words.into();
        self
    }

    /// What is wrong with the value: it takes the help's place while it is not empty.
    pub fn error<M>(mut self, words: impl IntoProp<String, M>) -> Self {
        self.error = words.into_prop();
        self
    }

    /// Marks the label of what must be filled in.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
}

impl View for Field {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let name = text(self.name.clone()).size(size::TEXT_SM.size).color(token::TEXT_2);
        let head = row().gap(token::SPACE_XS).child(name);
        let head = match self.required {
            true => head.child(text("*").size(size::TEXT_SM.size).color(token::ERROR)),
            false => head,
        };
        let (help, error) = (self.help, self.error);
        let wrong = memo(move || match &error {
            Prop::Value(words) => words.clone(),
            Prop::Reactive(read) => read(),
        });
        // The error, or the help, or nothing: 12 px under the control.
        let under = dynamic(move || {
            let wrong = wrong.get();
            let line = row().align_items(Align::Center).gap(token::SPACE_XS);
            if !wrong.is_empty() {
                let mark = icon(icons::CIRCLE_ALERT).size(12.0).color(token::ERROR);
                line.child(mark).child(text(wrong).size(size::TEXT_SM.size).color(token::ERROR))
            } else if !help.is_empty() {
                line.child(label(help.clone()))
            } else {
                line
            }
        });
        // A group named by the label: a reader says it on the way to the control. The control
        // keeps its own width (a segmented group, a switch); in a form it is the column's,
        // whatever it is wrapped in.
        let across = if crate::layout::fills() { Align::Stretch } else { Align::Start };
        let all = column().gap(token::SPACE_XS).align_items(across).role(Role::Group).label(self.name);
        all.child(head).child(self.control).child(under).mount(cx)
    }
}

pub struct Textarea {
    value: Signal<String>,
    rows: u32,
    width: Option<f32>,
    placeholder: String,
    label: Option<Prop<String>>,
}

/// A field of several lines that edits `value`, in the frame of an input: 240 px wide and
/// at least three lines tall unless told, growing with what it holds.
pub fn textarea(value: Signal<String>) -> Textarea {
    Textarea { value, rows: 3, width: None, placeholder: String::new(), label: None }
}

impl Textarea {
    /// The least number of lines it is tall.
    pub fn rows(mut self, rows: u32) -> Self {
        self.rows = rows.max(1);
        self
    }

    pub fn width(mut self, px: f32) -> Self {
        self.width = Some(px);
        self
    }

    /// What stands in it while it is empty, and what a reader calls it without a label.
    pub fn placeholder(mut self, words: impl Into<String>) -> Self {
        self.placeholder = words.into();
        self
    }

    /// What a reader calls it, a constant or words that change.
    pub fn label<M>(mut self, name: impl IntoProp<String, M>) -> Self {
        self.label = Some(name.into_prop());
        self
    }
}

impl View for Textarea {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        // ponytail: it grows without end; a most of rows, with scrolling inside, when wanted.
        let least = self.rows as f32 * size::TEXT_MD.line + 14.0;
        let frame = crate::layout::field_width(column(), self.width, 240.0);
        let frame = frame.min_height(least).padding_x(token::SPACE_MD).padding_y(token::SPACE_SM);
        let frame =
            frame.radius(token::RADIUS_MD).border(1.0).border_color(token::LINE_STRONG).bg(token::INPUT_BG);
        let frame = frame.cursor(Cursor::Text).selection_color(token::SELECTION).role(Role::Group);
        let frame = frame.focus_within(|s| s.border_color(token::BORDER_FOCUSED));
        let frame = frame.when(State::INVALID, |s| s.border_color(token::ERROR));
        let area = text_area(self.value).placeholder(&self.placeholder);
        let area = match self.label {
            Some(name) => area.label(name),
            None => area,
        };
        frame.child(area).mount(cx)
    }
}
