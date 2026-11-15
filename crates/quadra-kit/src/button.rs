//! The button (specs/behaviour/15, "Components"): a label that does something when it is
//! clicked, or when Enter or Space is pressed while it has the focus. A ghost has no fill
//! until the pointer is over it; an outlined one has a hairline border; a tinted one a fill
//! at low opacity, a border of its colour and an icon in it; a link is accent text.

use quadra_base::press;
use quadra_layout::{Align, Display, Justify};
use quadra_platform::Cursor;
use quadra_reactive::memo;
use quadra_tree::{NodeId, Paint, State};
use quadra_ui::{Div, IntoProp, MountCx, Prop, View, by_text, div, row, show, text};

use crate::icon::icon;
use crate::progress::spinner;
use crate::{size, token};

/// What a button looks like.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variant {
    /// Tinted with the accent: the action a view is for.
    Primary,
    /// Outlined.
    #[default]
    Secondary,
    /// Outlined, with nothing more when pressed.
    Outline,
    /// No fill until the pointer is over it.
    Ghost,
    /// Tinted with the colour of errors: what destroys something.
    Danger,
    /// Accent text.
    Link,
}

/// The height of a control: 18, 22, 28 or 32 px.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Size {
    Xs,
    Sm,
    #[default]
    Md,
    Lg,
}

impl Size {
    pub fn height(self) -> f32 {
        [size::H_XS, size::H_SM, size::H_MD, size::H_LG][self as usize]
    }
}

pub struct Button {
    /// Without one the button is its icon, and `name` is what a reader calls it.
    label: Option<Prop<String>>,
    name: Option<&'static str>,
    icon: Option<&'static str>,
    variant: Variant,
    size: Size,
    disabled: Prop<bool>,
    toggled: Option<Prop<bool>>,
    full: bool,
    muted: bool,
    loading: Option<Prop<bool>>,
    test_id: Option<&'static str>,
    click: Option<Box<dyn FnMut()>>,
}

/// A button that is only an icon: a ghost, 22 px square. `name` is what a reader calls it.
pub fn icon_button(body: &'static str, name: &'static str) -> Button {
    let plain = button("").variant(Variant::Ghost).size(Size::Sm).icon(body);
    Button { label: None, name: Some(name), ..plain }
}

/// A button with a label: a constant, or something reactive.
pub fn button<M>(label: impl IntoProp<String, M>) -> Button {
    Button {
        label: Some(label.into_prop()),
        name: None,
        icon: None,
        variant: Variant::default(),
        size: Size::default(),
        disabled: Prop::Value(false),
        toggled: None,
        full: false,
        muted: false,
        loading: None,
        test_id: None,
        click: None,
    }
}

/// The colours of a variant: its fill, its label, its fill under the pointer and while
/// pressed, its border, and its icon. Transparent: there is none, or the icon is the label's.
fn colors(variant: Variant) -> [Paint; 6] {
    use token::*;
    let none = Paint::default();
    let (fill, label, hover, press, border, mark) = match variant {
        Variant::Primary => (
            ACCENT_TINT.into(),
            TEXT,
            ACCENT_TINT_HOVER.into(),
            ACCENT_TINT_HOVER.into(),
            ACCENT.alpha(0.42),
            ACCENT_TEXT.into(),
        ),
        Variant::Danger => (
            ERROR_TINT.into(),
            TEXT,
            ERROR_TINT_HOVER.into(),
            ERROR_TINT_HOVER.into(),
            ERROR.alpha(0.42),
            ERROR.into(),
        ),
        Variant::Secondary => (none, TEXT, PANEL_HOVER.into(), PRESS.into(), LINE_STRONG.into(), none),
        Variant::Outline => (none, TEXT, PANEL_HOVER.into(), PANEL_HOVER.into(), LINE_STRONG.into(), none),
        Variant::Ghost => (none, TEXT, PANEL_HOVER.into(), PRESS.into(), none, none),
        Variant::Link => (none, ACCENT_TEXT, none, none, none, none),
    };
    [fill, label.into(), hover, press, border, mark]
}

impl Button {
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// A disabled button is not clicked, does not take the focus, and shows it.
    pub fn disabled<M>(mut self, disabled: impl IntoProp<bool, M>) -> Self {
        self.disabled = disabled.into_prop();
        self
    }

    /// Makes the button one that is on or off, which its owner says: its label turns the
    /// accent while it is on, and it is told to a reader as pressed or not.
    pub fn toggled<M>(mut self, on: impl IntoProp<bool, M>) -> Self {
        self.toggled = Some(on.into_prop());
        self
    }

    /// As wide as the box it is in.
    pub fn full(mut self) -> Self {
        self.full = true;
        self
    }

    /// While `loading` is true the button shows a spinner before its label and its click
    /// does nothing: what it started is under way.
    pub fn loading<M>(mut self, loading: impl IntoProp<bool, M>) -> Self {
        self.loading = Some(loading.into_prop());
        self
    }

    /// In the second text colour: a button of a status bar, which is not to draw the eye.
    pub fn muted(mut self) -> Self {
        self.muted = true;
        self
    }

    /// A name for tests and tools that does not change with the label.
    pub fn test_id(mut self, id: &'static str) -> Self {
        self.test_id = Some(id);
        self
    }

    /// An icon before the label: one of [`crate::icons`].
    pub fn icon(mut self, body: &'static str) -> Self {
        self.icon = Some(body);
        self
    }

    /// What the button does: on a click, and on Enter or Space while it has the focus.
    pub fn on_click(mut self, f: impl FnMut() + 'static) -> Self {
        self.click = Some(Box::new(f));
        self
    }

    fn build(self) -> Div {
        let [fill, label, hover, press, border, mark] = colors(self.variant);
        let off = match self.disabled {
            Prop::Value(off) => memo(move || off),
            Prop::Reactive(read) => memo(read),
        };
        let bordered = border != Paint::default();
        let small = matches!(self.size, Size::Xs | Size::Sm);
        let face = row().align_items(Align::Center).justify(Justify::Center).gap(4.0).shrink(0.0);
        // An icon alone sits in a square; a label has 4 or 8 px beside it.
        let face = match self.label {
            Some(_) => face.height(by_text(self.size.height())).padding_x(if small { 4.0 } else { 8.0 }),
            None => face.size(self.size.height(), self.size.height()),
        };
        let label = if self.muted { token::TEXT_2.into() } else { label };
        let face = face.radius(token::RADIUS_MD).bg(fill).text_color(label);
        let face = face.cursor(Cursor::Pointer);
        let face = match self.size {
            Size::Xs => face.text_size(size::TEXT_SM.size).line_height(size::TEXT_SM.line_height()),
            _ => face,
        };
        // A focus that came by the keyboard turns the border the focus colour; without a
        // border it is a line of a pixel along the edge, which takes no room.
        let face = match bordered {
            true => face
                .border(1.0)
                .border_color(border)
                .hover(|s| s.bg(hover))
                .focus_visible(|s| s.border_color(token::BORDER_FOCUSED)),
            false => face.focus_visible(|s| s.outline(token::BORDER_FOCUSED)).hover(|s| s.bg(hover)),
        };
        let face = face.active(|s| s.bg(press));
        // On: the accent for the label, and the selected step for a button with no fill
        // that has a label; an icon alone turns the accent and takes no fill.
        let face = match self.toggled {
            Some(_) => {
                let filled = fill != Paint::default() || self.label.is_none();
                let pressed = if filled { fill } else { token::PRESS.into() };
                face.when(State::CHECKED, |s| s.text_color(token::ACCENT_TEXT).bg(pressed))
            }
            None => face,
        };
        // Disabled: as it is at rest, whatever the pointer does, in the colour of what is off.
        let quiet = |s: quadra_ui::Refine| s.bg(fill).border_color(border).text_color(token::TEXT_DISABLED);
        let face = face.when(State::DISABLED, quiet);
        // Busy looks as it did, with a spinner where an icon goes; its click does nothing.
        let busy = self.loading.map(|loading| match loading {
            Prop::Value(busy) => memo(move || busy),
            Prop::Reactive(read) => memo(read),
        });
        let face = if self.full { face.grow(1.0) } else { face };
        let face = match self.test_id {
            Some(id) => face.test_id(id),
            None => face,
        };
        // What it does, by the pointer and the keyboard alike: the base's.
        let pressed = press::press(face).disabled(move || off.get());
        let pressed = match busy {
            Some(busy) => pressed.busy(busy),
            None => pressed,
        };
        let pressed = match self.toggled {
            Some(on) => pressed.toggled(on),
            None => pressed,
        };
        let face = match self.click {
            Some(click) => pressed.on_click(click).build(),
            None => pressed.build(),
        };
        let face = match self.name {
            Some(name) => face.label(name),
            None => face,
        };
        let face = match busy {
            Some(busy) => {
                let turning =
                    show(move || busy.get(), move || spinner().size(if small { 14.0 } else { 16.0 }));
                let room = move || if busy.get() { Display::Flex } else { Display::None };
                face.child(turning.container(div().display(room)))
            }
            None => face,
        };
        // The icon is 14 px, 16 in the two larger sizes, in the colour of the label, or of
        // the tint while the button is not disabled.
        let face = match self.icon {
            Some(body) => {
                let glyph = icon(body).size(if small { 14.0 } else { 16.0 });
                match mark == Paint::default() {
                    true => face.child(glyph),
                    false => {
                        let color = move || if off.get() { token::TEXT_DISABLED.into() } else { mark };
                        face.child(div().text_color(color).child(glyph))
                    }
                }
            }
            None => face,
        };
        match self.label {
            Some(label) => face.child(text(label)),
            None => face,
        }
    }
}

impl View for Button {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        self.build().mount(cx)
    }
}
