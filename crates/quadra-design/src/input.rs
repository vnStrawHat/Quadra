//! The text input of the kit (specs/behaviour/15, "Text input"): the field of `quadra-view` in
//! a frame. The frame's border says where the focus is and whether the value is accepted.

use quadra_layout::Align;
use quadra_platform::Cursor;
use quadra_reactive::Signal;
use quadra_tree::{NodeId, Role, State};
use quadra_view::{Div, IntoProp, MountCx, Prop, View, row, show, text, text_input};

use crate::icon::icon;
use crate::{icons, token};

pub struct Input {
    value: Signal<String>,
    placeholder: String,
    password: bool,
    invalid: Prop<bool>,
    disabled: Prop<bool>,
    width: f32,
    lead: Option<&'static str>,
    clears: bool,
    readonly: bool,
    test_id: Option<&'static str>,
    submit: Option<Submit>,
}

/// What runs with the text when Enter is pressed.
type Submit = Box<dyn FnMut(&str)>;

/// A field of one line that edits `value`, 240 px wide unless told otherwise.
pub fn input(value: Signal<String>) -> Input {
    Input {
        value,
        placeholder: String::new(),
        password: false,
        invalid: Prop::Value(false),
        disabled: Prop::Value(false),
        width: 240.0,
        lead: None,
        clears: false,
        readonly: false,
        test_id: None,
        submit: None,
    }
}

impl Input {
    /// What stands in the field while it is empty, and what a reader calls it.
    pub fn placeholder(mut self, text: &str) -> Self {
        self.placeholder = text.to_string();
        self
    }

    /// A dot for each character, and nothing of it copied.
    pub fn password(mut self) -> Self {
        self.password = true;
        self
    }

    /// The value is not accepted: the border says so, with the focus or without.
    pub fn invalid<M>(mut self, invalid: impl IntoProp<bool, M>) -> Self {
        self.invalid = invalid.into_prop();
        self
    }

    pub fn disabled<M>(mut self, disabled: impl IntoProp<bool, M>) -> Self {
        self.disabled = disabled.into_prop();
        self
    }

    pub fn width(mut self, px: f32) -> Self {
        self.width = px;
        self
    }

    /// An icon before the text: one of [`crate::icons`].
    pub fn icon(mut self, body: &'static str) -> Self {
        self.lead = Some(body);
        self
    }

    /// The value shows, and can be selected and copied, and not typed over.
    pub fn readonly(mut self) -> Self {
        self.readonly = true;
        self
    }

    /// A cross at the end of the field while it holds text: a click on it empties the field.
    pub fn clearable(mut self) -> Self {
        self.clears = true;
        self
    }

    pub fn test_id(mut self, id: &'static str) -> Self {
        self.test_id = Some(id);
        self
    }

    /// Runs `f` with the text when Enter is pressed in the field.
    pub fn on_submit(mut self, f: impl FnMut(&str) + 'static) -> Self {
        self.submit = Some(Box::new(f));
        self
    }

    pub(crate) fn build(self) -> Div {
        let field = text_input(self.value).placeholder(&self.placeholder);
        let field = if self.password { field.password() } else { field };
        let field = match self.submit {
            Some(submit) => field.on_submit(submit),
            None => field,
        };
        // The name for tests is the field's: the frame means nothing to a reader.
        let field = match self.test_id {
            Some(id) => field.test_id(id),
            None => field,
        };
        let frame = row().align_items(Align::Center).gap(4.0).width(self.width).min_height(32.0);
        let frame = frame.padding_x(8.0).padding_y(6.0).radius(token::RADIUS_LG).border(1.0);
        let frame = frame.bg(token::INPUT_BG).border_color(token::LINE_STRONG).cursor(Cursor::Text);
        let frame = frame.selection_color(token::SELECTION);
        // The border is where the focus shows; a value that is not accepted keeps the
        // colour of errors with the focus or without; a disabled field is a step back.
        let frame = frame.focus_within(|s| s.border_color(token::BORDER_FOCUSED));
        let frame = frame.disabled(self.disabled);
        let frame = frame.when(State::DISABLED, |s| s.bg(token::PANEL).text_color(token::TEXT_DISABLED));
        let frame = frame.invalid(self.invalid).when(State::INVALID, |s| s.border_color(token::ERROR));
        let frame = match self.lead {
            Some(body) => frame.child(icon(body).color(token::TEXT_2)),
            None => frame,
        };
        let value = self.value;
        let frame = match self.readonly {
            true => frame.child(text(move || value.get()).selectable()),
            false => frame.child(field),
        };
        if !self.clears {
            return frame;
        }
        let cross = move || {
            let cross = row().align_items(Align::Center).cursor(Cursor::Pointer).text_color(token::TEXT_2);
            let cross =
                cross.role(Role::Button).label("Clear".to_string()).hover(|s| s.text_color(token::TEXT));
            cross.on_click(move |_| value.set(String::new())).child(icon(icons::X).size(12.0))
        };
        frame.child(show(move || !value.with(String::is_empty), cross))
    }
}

impl View for Input {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        self.build().mount(cx)
    }
}
