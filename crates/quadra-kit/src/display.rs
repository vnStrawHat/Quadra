//! The small things that show and do nothing (specs/behaviour/09, "Data" and "Layout"):
//! headings and labels, badges and tags, keys, an avatar, a card, dividers.

use quadra_layout::{Align, Display, Justify};
use quadra_prim::Color;
use quadra_tree::{Role, Side, Token};
use quadra_ui::{Div, TextView, column, div, row, text};

use crate::{fonts, size, token};

/// A heading of a level from 1: the size alone ranks it, 23, 18 and 16 px, then that of text,
/// on lines as tall as text of the UI has them (1.618 of the size, in whole pixels).
pub fn heading(words: impl Into<String>, level: u8) -> Div {
    let px = match level {
        1 => size::TEXT_2XL.size,
        2 => size::TEXT_XL.size,
        3 => size::TEXT_LG.size,
        _ => size::TEXT_MD.size,
    };
    let line = div().role(Role::Heading).text_size(px).line_height(size::LINE);
    line.child(text(words.into()))
}

/// Muted secondary text, 12 px.
pub fn label(words: impl Into<String>) -> TextView {
    text(words.into()).size(size::TEXT_SM.size).color(token::TEXT_2)
}

/// What a badge is about: its colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Neutral,
    Primary,
    Success,
    Warning,
    Danger,
    Info,
    /// A kind among others: one of the hues (`token::HUE_GREEN`), with its words beside it.
    Hue(Token),
}

impl Tone {
    /// The role of the theme that is the tone's colour.
    pub(crate) fn role(self) -> Token {
        match self {
            Tone::Neutral => token::TEXT_2,
            Tone::Primary => token::ACCENT_TEXT,
            Tone::Success => token::OK,
            Tone::Warning => token::WARN,
            Tone::Danger => token::ERROR,
            Tone::Info => token::INFO,
            Tone::Hue(hue) => hue,
        }
    }
}

/// A chip of 16 px: 10 px mono text in a faint tint of its tone.
pub fn badge(words: impl Into<String>, tone: Tone) -> Div {
    let tone = tone.role();
    let chip = chip(16.0).border_color(tone.alpha(0.2)).bg(tone.alpha(0.1)).text_color(tone);
    // The row around keeps the chip as wide as its text where it would be stretched.
    row().child(chip.child(text(words.into())))
}

/// A chip of 18 px on the raised plane: a name among others.
pub fn tag(words: impl Into<String>) -> Div {
    let chip = chip(18.0).border_color(token::LINE_STRONG).bg(token::RAISED).text_color(token::TEXT);
    row().child(chip.child(text(words.into())))
}

fn chip(height: f32) -> Div {
    let chip = row().align_items(Align::Center).shrink(0.0).height(height).padding_x(4.0).border(1.0);
    let chip = chip.radius(size::RADIUS_MD).font_family(fonts::MONO);
    chip.text_size(size::TEXT_XS.size).line_height(size::TEXT_XS.line_height())
}

/// A binding as the kit shows shortcuts: "ctrl-shift-p" is "Ctrl-Shift-P", the modifiers in
/// the order Ctrl, Win, Alt, Shift; the strokes of a chord keep the space between them.
pub fn key_label(keys: &str) -> String {
    let stroke = |stroke: &str| {
        let lower = stroke.to_lowercase().replace("super", "win").replace("cmd", "win");
        let mut parts: Vec<&str> = lower.split('-').collect();
        let key = parts.pop().unwrap_or_default();
        let held = ["ctrl", "win", "alt", "shift"].into_iter().filter(|held| parts.contains(held));
        let mut out: Vec<String> = held.map(capital).collect();
        out.push(match key {
            "minus" => "-".to_string(),
            "plus" => "+".to_string(),
            "pageup" => "PageUp".to_string(),
            "pagedown" => "PageDown".to_string(),
            key => capital(key),
        });
        out.join("-")
    };
    keys.split(' ').map(stroke).collect::<Vec<_>>().join(" ")
}

fn capital(word: &str) -> String {
    let mut letters = word.chars();
    letters.next().map_or_else(String::new, |first| first.to_uppercase().chain(letters).collect())
}

/// A shortcut as muted text: `kbd("ctrl-s")` shows "Ctrl-S".
pub fn kbd(keys: &str) -> TextView {
    text(key_label(keys)).color(token::TEXT_2)
}

/// A hint: the keys in a small box tinted toward the accent, then what they do, in small
/// mono italics in the colour of what is off.
pub fn key_hint(keys: &str, does: impl Into<String>) -> Div {
    let key = div().padding_x(2.0).radius(size::RADIUS_MD).border(1.0).border_color(token::TEXT.alpha(0.16));
    let key =
        key.bg(token::ACCENT_TEXT.alpha(0.1)).text_size(size::TEXT_XS.size).child(text(key_label(keys)));
    let hint = row().align_items(Align::Center).gap(4.0).font_family(fonts::MONO).italic(true);
    let hint = hint.text_color(token::TEXT_DISABLED).text_size(size::TEXT_SM.size);
    hint.child(key).child(text(does.into()))
}

/// The initials of a name in a disc of `side` px, in a colour that is the name's own.
pub fn avatar(name: &str, side: f32) -> Div {
    let initials: String = name.split_whitespace().filter_map(|word| word.chars().next()).take(2).collect();
    // ponytail: the colour is from the bytes of the name; an image comes with image nodes.
    let hue = name.bytes().fold(7u32, |hash, byte| hash.wrapping_mul(31).wrapping_add(byte as u32)) % 360;
    let disc = row().align_items(Align::Center).justify(Justify::Center).shrink(0.0).size(side, side);
    let disc = disc.radius(side / 2.0).bg(Color::hsla(hue as f32, 0.55, 0.55, 1.0)).text_color(Color::WHITE);
    let disc = disc.text_size(side * 0.4).line_height(1.0).font_weight(size::WEIGHT_STRONG);
    disc.role(Role::Image).label(name.to_string()).child(text(initials.to_uppercase()))
}

/// A card: a bordered box on the panel's plane, for what belongs together. Give it children;
/// [`card_head`] is its title.
pub fn card() -> Div {
    let card = column().gap(8.0).padding(12.0).bg(token::PANEL).radius(size::RADIUS_MD);
    card.border(1.0).border_color(token::LINE_STRONG)
}

/// The head of a card: a title of 16 px over a muted line that says more, if there is one.
pub fn card_head(title: impl Into<String>, more: impl Into<String>) -> Div {
    let (title, more) = (text(title.into()).size(size::TEXT_LG.size), more.into());
    let head = column().gap(2.0).child(title);
    if more.is_empty() { head } else { head.child(label(more)) }
}

/// The mark of what is chosen (specs/behaviour/15, principle 2): a bar of 2 px in the accent
/// along one edge of the box it is put in, over what is there and taking no room, shown
/// while `on` says. Along the left it stops 4 px short of the ends.
pub fn chosen_mark(side: Side, on: impl Fn() -> bool + 'static) -> Div {
    let shown = move || if on() { Display::Flex } else { Display::None };
    let bar = div().absolute().bg(token::ACCENT).display(shown);
    match side {
        Side::Left => bar.left(0.0).top(4.0).bottom(4.0).width(2.0).radius(1.0),
        Side::Right => bar.right(0.0).top(4.0).bottom(4.0).width(2.0).radius(1.0),
        Side::Above => bar.left(0.0).right(0.0).top(0.0).height(2.0),
        Side::Below => bar.left(0.0).right(0.0).bottom(0.0).height(2.0),
    }
}

/// A line of one pixel across a column, and one down a row.
pub fn divider() -> Div {
    div().height(1.0).shrink(0.0).bg(token::LINE)
}

pub fn vdivider() -> Div {
    div().width(1.0).shrink(0.0).bg(token::LINE)
}
