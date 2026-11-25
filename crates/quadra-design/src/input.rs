//! The text input of the kit (specs/design, "Text input"): the field of `quadra-view` in
//! a frame. The frame's border says where the focus is and whether the value is accepted.
//! Underlined, the frame is a line under the text and no box.

use quadra_layout::{Align, Length};
use quadra_platform::Cursor;
use quadra_reactive::{Signal, effect, signal, untrack};
use quadra_tree::{NodeId, Role, State};
use quadra_view::{AnyView, Div, IntoProp, MountCx, Prop, View, by_text, row, show, text, text_input};

use crate::button::icon_button;
use crate::icon::icon;
use crate::{fonts, icons, size, token};

pub struct Input {
    value: Signal<String>,
    placeholder: String,
    label: Option<Prop<String>>,
    password: bool,
    invalid: Prop<bool>,
    disabled: Prop<bool>,
    /// The width it was told, and its own.
    width: Option<f32>,
    own: f32,
    lead: Option<&'static str>,
    /// What stands before the text in place of an icon: a colour's swatch.
    pub(crate) before: Option<AnyView>,
    clears: bool,
    readonly: bool,
    /// The mono face, 12 px text, a fixed height, the room left in the row, an eye that shows
    /// a password.
    mono: bool,
    small: bool,
    height: Option<f32>,
    fill: bool,
    reveal: bool,
    underlined: bool,
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
        label: None,
        password: false,
        invalid: Prop::Value(false),
        disabled: Prop::Value(false),
        width: None,
        own: 240.0,
        lead: None,
        before: None,
        clears: false,
        readonly: false,
        mono: false,
        small: false,
        height: None,
        fill: false,
        reveal: false,
        underlined: false,
        test_id: None,
        submit: None,
    }
}

impl Input {
    /// What stands in the field while it is empty, and what a reader calls it without a label.
    pub fn placeholder(mut self, text: &str) -> Self {
        self.placeholder = text.to_string();
        self
    }

    /// What a reader calls it, a constant or words that change: drawn nowhere.
    pub fn label<M>(mut self, name: impl IntoProp<String, M>) -> Self {
        self.label = Some(name.into_prop());
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
        self.width = Some(px);
        self
    }

    /// The width of the control built on this field, unless it is told one or is in a form.
    pub(crate) fn own_width(mut self, px: f32) -> Self {
        self.own = px;
        self
    }

    /// An icon before the text: one of [`crate::icons`].
    pub fn icon(mut self, body: &'static str) -> Self {
        self.lead = Some(body);
        self
    }

    /// Words before the text in the second colour: `ssh` before a target.
    pub fn prefix(mut self, words: &str) -> Self {
        self.before =
            Some(AnyView::new(row().shrink(0.0).text_color(token::TEXT_2).child(text(words.to_string()))));
        self
    }

    /// In the kit's mono face: paths, targets, numbers.
    pub fn mono(mut self) -> Self {
        self.mono = true;
        self
    }

    /// Text of 12 px: a field in a panel's bar.
    pub fn small(mut self) -> Self {
        self.small = true;
        self
    }

    /// A fixed height, the text centred in it (the md control height, 28, unless told).
    pub fn height(mut self, px: f32) -> Self {
        self.height = Some(px);
        self
    }

    /// As wide as the room left in the row it is in.
    pub fn fill(mut self) -> Self {
        self.fill = true;
        self
    }

    /// A password with an eye at its end that shows and hides its characters.
    pub fn reveal(mut self) -> Self {
        self.reveal = true;
        self
    }

    /// The second look: no fill and no box, a line under the text, which says the focus
    /// and a value not accepted as the box's border does.
    pub fn underlined(mut self) -> Self {
        self.underlined = true;
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
        let (shown, value) = (signal(false), self.value);
        // A password emptied (sent, or taken back) hides again.
        if self.reveal {
            effect(move || {
                if value.with(String::is_empty) && untrack(|| shown.get()) {
                    shown.set(false);
                }
            });
        }
        let field = match (self.password, self.reveal) {
            (true, true) => field.password().reveal(shown),
            (true, false) => field.password(),
            _ => field,
        };
        // Read only, there is no field: the frame is a group a reader knows by the label.
        let (field, group) = match (self.label, self.readonly) {
            (Some(name), false) => (field.label(name), None),
            (name, _) => (field, name),
        };
        let field = match self.submit {
            Some(submit) => field.on_submit(submit),
            None => field,
        };
        // The name for tests is the field's: the frame means nothing to a reader.
        let field = match self.test_id {
            Some(id) => field.test_id(id),
            None => field,
        };
        let frame = row().align_items(Align::Center).gap(token::SPACE_XS);
        // Underlined, the text starts where the line does, and stands where a box's would:
        // the pixel over it is the border a box has there.
        let underlined = self.underlined;
        let frame = match underlined {
            true => frame.padding_top(1.0).border_bottom(1.0),
            false => {
                frame.padding_x(token::SPACE_MD).radius(token::RADIUS_MD).border(1.0).bg(token::INPUT_BG)
            }
        };
        let frame = match self.fill {
            true => frame.grow(1.0).width(0.0).min_width(Length::Px(0.0)),
            false => crate::layout::field_width(frame, self.width, self.own),
        };
        let frame = match self.height {
            Some(px) => frame.height(by_text(px)),
            None => frame.height(by_text(size::H_MD)),
        };
        let frame = if self.mono { frame.font_family(fonts::MONO) } else { frame };
        let frame = if self.small { frame.text_size(size::TEXT_SM.size) } else { frame };
        let frame = frame.border_color(token::LINE_STRONG).cursor(Cursor::Text);
        let frame = frame.selection_color(token::SELECTION);
        // The border is where the focus shows; a value that is not accepted keeps the
        // colour of errors with the focus or without; a disabled field is a step back.
        let frame = frame.focus_within(|s| s.border_color(token::BORDER_FOCUSED));
        let frame = frame.disabled(self.disabled);
        let frame = frame.when(State::DISABLED, move |s| {
            let s = s.text_color(token::TEXT_DISABLED);
            if underlined { s.border_color(token::LINE) } else { s.bg(token::PANEL) }
        });
        let frame = frame.invalid(self.invalid).when(State::INVALID, |s| s.border_color(token::ERROR));
        let frame = match (self.lead, self.before) {
            (Some(body), _) => frame.child(icon(body).color(token::TEXT_2)),
            (None, Some(before)) => frame.child(before),
            (None, None) => frame,
        };
        let value = self.value;
        let frame = match (self.readonly, group) {
            (true, Some(name)) => {
                frame.role(Role::Group).label(name).child(text(move || value.get()).selectable())
            }
            (true, None) => frame.child(text(move || value.get()).selectable()),
            (false, _) => frame.child(field),
        };
        // The eye: pressed while the characters show.
        let frame = match self.password && self.reveal {
            true => frame.child(
                icon_button(icons::EYE, "Show password")
                    .toggled(move || shown.get())
                    .on_click(move || shown.update(|on| *on = !*on)),
            ),
            false => frame,
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
