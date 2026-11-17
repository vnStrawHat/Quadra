//! The colour picker of the kit (specs/13, `color_picker`; behaviour/15, "Color picker"): a
//! field that shows a colour and opens a popover to choose another, by its saturation and
//! brightness, its hue, its hex or one of a few given.

use quadra_headless::disclose;
use quadra_headless::press::press;
use quadra_layout::Align;
use quadra_platform::Cursor;
use quadra_prim::Color;
use quadra_reactive::{Signal, effect, signal};
use quadra_tree::{NodeId, Paint, State};
use quadra_view::{AnyView, Div, IntoProp, MountCx, NodeRef, Prop, View, by_text, column, div, row, text};

use super::area::{Hsv, WIDE, area, hex, hue_bar};
use super::color_input;
use crate::popup::popover;
use crate::{size, token};

/// A field that opens a popover to choose a colour; see [`color_picker`].
pub struct ColorPicker {
    value: Signal<String>,
    presets: Vec<(String, Color)>,
    fallback: Option<Paint>,
    placeholder: String,
    label: Option<Prop<String>>,
    width: f32,
    test_id: Option<&'static str>,
}

/// A field, 120 px wide, that shows the colour in `value` (`#rrggbb`, or empty for the
/// default) and opens a popover to choose another.
pub fn color_picker(value: Signal<String>) -> ColorPicker {
    ColorPicker {
        value,
        presets: Vec::new(),
        fallback: None,
        placeholder: "Default".to_string(),
        label: None,
        width: 120.0,
        test_id: None,
    }
}

impl ColorPicker {
    /// The colours offered as swatches, each with its name for a reader.
    pub fn presets<'a>(mut self, colors: impl IntoIterator<Item = (&'a str, Color)>) -> Self {
        self.presets = colors.into_iter().map(|(name, color)| (name.to_string(), color)).collect();
        self
    }

    /// The colour that stands while the value is empty: the field's swatch shows it, and the
    /// swatches start with a "Default" one of it that empties the value.
    pub fn fallback(mut self, paint: impl Into<Paint>) -> Self {
        self.fallback = Some(paint.into());
        self
    }

    /// What the field says while the value is empty: "Default" unless told.
    pub fn placeholder(mut self, text: &str) -> Self {
        self.placeholder = text.to_string();
        self
    }

    /// What a reader calls it, a constant or words that change; the colour is then its value.
    pub fn label<M>(mut self, name: impl IntoProp<String, M>) -> Self {
        self.label = Some(name.into_prop());
        self
    }

    pub fn width(mut self, px: f32) -> Self {
        self.width = px;
        self
    }

    pub fn test_id(mut self, id: &'static str) -> Self {
        self.test_id = Some(id);
        self
    }
}

/// A swatch of 20 px that writes `write` to `value` and closes the popover; ringed while the
/// value is it.
fn swatch(name: String, paint: Paint, write: String, value: Signal<String>, open: Signal<bool>) -> Div {
    let chosen = write.clone();
    let face = div().size(20.0, 20.0).shrink(0.0).radius(token::RADIUS_SM).border(1.0);
    let face = face.border_color(token::LINE_STRONG).bg(paint).cursor(Cursor::Pointer).label(name);
    let face = face.selected(move || value.with(|it| it.trim().eq_ignore_ascii_case(&chosen)));
    let face =
        face.when(State::SELECTED, |s| s.border_color(token::BORDER_FOCUSED).outline(token::BORDER_FOCUSED));
    let face = face.focus_visible(|s| s.outline(token::BORDER_FOCUSED));
    press(face)
        .on_click(move || {
            value.set(write.clone());
            open.set(false);
        })
        .build()
}

/// What the popover holds. The hue, saturation and brightness it moves are its own while it
/// is open, so a drag through grey or black keeps its hue; they follow `value` only when it
/// changes to a colour they do not make.
fn popover_body(
    value: Signal<String>,
    open: Signal<bool>,
    presets: &[(String, Color)],
    fallback: Option<Paint>,
) -> Div {
    let white = Hsv { h: 0.0, s: 0.0, v: 1.0 };
    let start = value.with(|it| Color::parse_hex(it.trim())).map_or(white, |color| Hsv::of(color, white));
    let (hsv, hue) = (signal(start), signal(start.h));
    let put = move |to: Hsv| {
        hsv.set(to);
        if hue.get_untracked() != to.h {
            hue.set(to.h);
        }
        let written = to.hex();
        if value.get_untracked() != written {
            value.set(written);
        }
    };
    effect(move || {
        let (h, now) = (hue.get(), hsv.get_untracked());
        if h != now.h {
            put(Hsv { h, ..now });
        }
    });
    effect(move || {
        let Some(color) = value.with(|it| Color::parse_hex(it.trim())) else { return };
        let now = hsv.get_untracked();
        if hex(color) != now.hex() {
            let to = Hsv::of(color, now);
            hsv.set(to);
            hue.set(to.h);
        }
    });
    let plane = area(hsv, put).on_mount(|tree, id| tree.after_layout(move |tree| tree.focus(id)));
    let mut field = color_input(value).width(WIDE).label("Hex");
    if let Some(paint) = fallback {
        field = field.fallback(paint);
    }
    let default = fallback.map(|paint| swatch("Default".to_string(), paint, String::new(), value, open));
    let given =
        presets.iter().map(|(name, color)| swatch(name.clone(), (*color).into(), hex(*color), value, open));
    let swatches: Vec<Div> = default.into_iter().chain(given).collect();
    let body = column().gap(12.0).padding(12.0).child(plane).child(hue_bar(hue)).child(field);
    match swatches.is_empty() {
        true => body,
        false => body.child(row().width(WIDE).wrap(true).gap(6.0).children(swatches)),
    }
}

impl View for ColorPicker {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (value, open, opener) = (self.value, signal(false), NodeRef::new());
        let (fallback, placeholder) = (self.fallback, self.placeholder);
        let empty = move || value.with(|it| it.trim().is_empty());
        let face = row().align_items(Align::Center).gap(6.0).shrink(0.0).width(self.width);
        let face = face.height(by_text(size::H_MD)).padding_x(8.0).radius(token::RADIUS_MD).border(1.0);
        let face = face.border_color(token::LINE_STRONG).cursor(Cursor::Pointer);
        let face = face.hover(|s| s.bg(token::PANEL_HOVER).border_color(token::LINE_STRONG));
        let face = face.focus_visible(|s| s.border_color(token::BORDER_FOCUSED));
        // Empty, the code of the colour that stands instead, as the window's theme has it.
        let theme = quadra_reactive::use_context::<Signal<std::rc::Rc<quadra_tree::Theme>>>();
        let words = placeholder.clone();
        let said = move || match (empty(), fallback.zip(theme)) {
            (false, _) => value.get(),
            (true, Some((paint, theme))) => hex(theme.with(|it| it.resolve(paint))),
            (true, None) => words.clone(),
        };
        let face = match self.label {
            Some(name) => face.label(name).value(move || value.get()),
            None => face.label(said.clone()),
        };
        let face = match self.test_id {
            Some(id) => face.test_id(id),
            None => face,
        };
        let face = disclose::head(face.node_ref(&opener), open);
        let typed = move || value.with(|it| Color::parse_hex(it.trim()));
        let shown = move || typed().map(Paint::from).or(fallback).unwrap_or_default();
        let mark = div().size(14.0, 14.0).shrink(0.0).radius(token::RADIUS_SM).border(1.0);
        let mark = mark.border_color(token::LINE_STRONG).bg(shown);
        let tone = move || if empty() { token::TEXT_3 } else { token::TEXT };
        let words = div().min_width(0.0).text_color(tone).child(text(said));
        let face = face.child(mark).child(words);
        let presets = self.presets;
        let body = popover(open, &opener, move || popover_body(value, open, &presets, fallback));
        div().shrink(0.0).child(face).child(AnyView::new(body)).mount(cx)
    }
}
