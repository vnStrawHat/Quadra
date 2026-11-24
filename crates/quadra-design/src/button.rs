//! The button (specs/design, "Components"): a label that does something when it is
//! clicked, or when Enter or Space is pressed while it has the focus. A ghost has no fill
//! until the pointer is over it; an outlined one has a hairline border; the primary one is
//! the accent, solid, the one thing in a view that is; a tinted one has a fill at low opacity,
//! a border of its colour and an icon in it; a link is accent text.

use quadra_layout::{Align, Display, Justify};
use quadra_platform::Cursor;
use quadra_reactive::memo;
use quadra_tree::{NodeId, Paint, State};
use quadra_view::{Div, IntoProp, MountCx, Prop, View, by_text, div, row, show, text};

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
    name: Option<Prop<String>>,
    /// Before the label, and after it.
    icons: [Option<&'static str>; 2],
    variant: Variant,
    size: Size,
    disabled: Prop<bool>,
    toggled: Option<Prop<bool>>,
    full: bool,
    muted: bool,
    loading: Option<Prop<bool>>,
    test_id: Option<&'static str>,
    shortcut: Option<&'static str>,
    click: Option<Box<dyn FnMut()>>,
}

/// A button that is only an icon: a ghost, 22 px square. `name` is what a reader calls it.
pub fn icon_button(body: &'static str, name: &'static str) -> Button {
    let plain = button("").variant(Variant::Ghost).size(Size::Sm).icon(body);
    Button { label: None, name: Some(Prop::Value(name.into())), ..plain }
}

/// A button with a label: a constant, or something reactive.
pub fn button<M>(label: impl IntoProp<String, M>) -> Button {
    Button {
        label: Some(label.into_prop()),
        name: None,
        icons: [None; 2],
        variant: Variant::default(),
        size: Size::default(),
        disabled: Prop::Value(false),
        toggled: None,
        full: false,
        muted: false,
        loading: None,
        test_id: None,
        shortcut: None,
        click: None,
    }
}

/// A variant's fill, label, fills under the pointer and pressed, border, icon (clear: the label's).
fn colors(variant: Variant) -> [Paint; 6] {
    use token::*;
    let none = Paint::default();
    // A tinted one: its hover and pressed fills are one, its border its colour at 42 %.
    let over = ERROR_TINT_HOVER.into();
    let (fill, label, hover, press, border, mark) = match variant {
        Variant::Primary => {
            (ACCENT.into(), TEXT_ON_ACCENT, ACCENT_HOVER.into(), ACCENT_PRESS.into(), ACCENT.into(), none)
        }
        Variant::Danger => (ERROR_TINT.into(), TEXT, over, over, ERROR.alpha(0.42), ERROR.into()),
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

    /// On or off as its owner says: its label in the accent while on, pressed or not to a reader.
    pub fn toggled<M>(mut self, on: impl IntoProp<bool, M>) -> Self {
        self.toggled = Some(on.into_prop());
        self
    }

    /// As wide as the box it is in.
    pub fn full(mut self) -> Self {
        self.full = true;
        self
    }

    /// While `loading` is true: a spinner before its label, and its click does nothing.
    pub fn loading<M>(mut self, loading: impl IntoProp<bool, M>) -> Self {
        self.loading = Some(loading.into_prop());
        self
    }

    /// In the second text colour: a button of a status bar, which is not to draw the eye.
    pub fn muted(mut self) -> Self {
        self.muted = true;
        self
    }

    /// What a reader calls it if not its label ("Reset Copy" of a few "Reset"); drawn nowhere.
    pub fn label<M>(mut self, name: impl IntoProp<String, M>) -> Self {
        self.name = Some(name.into_prop());
        self
    }

    /// A name for tests and tools that does not change with the label.
    pub fn test_id(mut self, id: &'static str) -> Self {
        self.test_id = Some(id);
        self
    }

    /// The key that does the same (`Enter`), after the label in 12 px and the second colour.
    pub fn shortcut(mut self, key: &'static str) -> Self {
        self.shortcut = Some(key);
        self
    }

    /// An icon before the label: one of [`crate::icons`].
    pub fn icon(mut self, body: &'static str) -> Self {
        self.icons[0] = Some(body);
        self
    }

    /// An icon after the label, as one before it is: a chevron to where the button leads.
    pub fn icon_end(mut self, body: &'static str) -> Self {
        self.icons[1] = Some(body);
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
        let face = row().align_items(Align::Center).justify(Justify::Center).gap(token::SPACE_XS).shrink(0.0);
        // An icon alone sits in a square; a label has 4 or 8 px beside it.
        let beside = if small { token::SPACE_XS } else { token::SPACE_MD };
        let face = match self.label {
            Some(_) => face.height(by_text(self.size.height())).padding_x(beside),
            None => face.size(self.size.height(), self.size.height()),
        };
        let label = if self.muted { token::TEXT_2.into() } else { label };
        let face = face.radius(token::RADIUS_MD).bg(fill).text_color(label).cursor(Cursor::Pointer);
        let face = match self.size {
            Size::Xs => face.text_size(size::TEXT_SM.size).line_height(size::TEXT_SM.line_height()),
            _ => face,
        };
        // A focus that came by the keyboard turns the border the focus colour (the text's on
        // the solid one, whose border is that colour already); without a border it is a line
        // of a pixel along the edge, which takes no room.
        let solid = self.variant == Variant::Primary;
        let ring = if solid { token::TEXT } else { token::BORDER_FOCUSED };
        let face = match bordered {
            true => face.border(1.0).border_color(border).focus_visible(|s| s.border_color(ring)),
            false => face.focus_visible(|s| s.outline(token::BORDER_FOCUSED)),
        };
        let face = face.hover(|s| s.bg(hover)).active(|s| s.bg(press));
        // On: the accent for the label (the solid one keeps its own), and the accent's tint for
        // a button with no fill that has a label; an icon alone turns the accent and takes no fill.
        let face = match self.toggled {
            Some(_) => {
                let filled = fill != Paint::default() || self.label.is_none();
                let pressed = if filled { fill } else { token::ACCENT_TINT.into() };
                let on = if solid { label } else { token::ACCENT_TEXT.into() };
                face.when(State::CHECKED, |s| s.text_color(on).bg(pressed))
            }
            None => face,
        };
        // Disabled: as it is at rest, whatever the pointer does, in the colour of what is off;
        // the solid one is its tint, which says less.
        let tint = [token::ACCENT_TINT.into(), token::ACCENT.alpha(0.42)];
        let [rest, edge] = if solid { tint } else { [fill, border] };
        let quiet = |s: quadra_view::Refine| s.bg(rest).border_color(edge).text_color(token::TEXT_DISABLED);
        let face = face.when(State::DISABLED, quiet);
        // Busy looks as it did, with a spinner where an icon goes; its click does nothing.
        let busy = self.loading.map(|loading| match loading {
            Prop::Value(busy) => memo(move || busy),
            Prop::Reactive(read) => memo(read),
        });
        let face = if self.full { face.grow(1.0) } else { face };
        let face = self.test_id.into_iter().fold(face, |face, id| face.test_id(id));
        // What it does, by the pointer and the keyboard alike: the headless crate's.
        let pressed = quadra_headless::button(face).disabled(move || off.get());
        let pressed = match busy {
            Some(busy) => pressed.busy(busy),
            None => pressed,
        };
        let pressed = self.toggled.into_iter().fold(pressed, |pressed, on| pressed.toggled(on));
        let face = match self.click {
            Some(click) => pressed.on_click(click).build(),
            None => pressed.build(),
        };
        // With a key after the label, the label alone names it.
        let face = match (self.name, &self.label, self.shortcut) {
            (Some(name), _, _) => face.label(name),
            (None, Some(Prop::Value(words)), Some(_)) => face.label(words.clone()),
            _ => face,
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
        // An icon is 14 px, 16 in the two larger sizes, in the colour of the label, or of
        // the tint while the button is not disabled.
        let glyph = move |face: Div, body: Option<&'static str>| {
            let Some(body) = body else { return face };
            let glyph = icon(body).size(if small { 14.0 } else { 16.0 });
            if mark == Paint::default() {
                return face.child(glyph);
            }
            let color = move || if off.get() { token::TEXT_DISABLED.into() } else { mark };
            face.child(div().text_color(color).child(glyph))
        };
        let face = glyph(face, self.icons[0]);
        let face = match self.label {
            Some(label) => glyph(face.child(text(label)), self.icons[1]),
            None => face,
        };
        let Some(key) = self.shortcut else { return face };
        // On the solid one the key is the label's colour, dimmed; elsewhere, and off, muted.
        let dim =
            move || if solid && !off.get() { token::TEXT_ON_ACCENT.alpha(0.7) } else { token::TEXT_2.into() };
        face.child(div().text_color(dim).child(text(key).size(size::TEXT_SM.size)))
    }
}

impl View for Button {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        self.build().mount(cx)
    }
}
