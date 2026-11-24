//! The parts of the colour picker's popover (specs/13, `color_picker`; specs/design, "Color
//! picker"): the area of saturation and brightness, the bar of the hues, and the sums between
//! a hue, saturation and brightness and the red, green and blue of a colour.

use std::cell::Cell;
use std::rc::Rc;

use quadra_headless::slider::{self, Scale};
use quadra_layout::Length;
use quadra_platform::{Cursor, Key};
use quadra_prim::Color;
use quadra_reactive::{Signal, memo};
use quadra_render::Gradient;
use quadra_tree::{PointerKind, Role, SemanticAction};
use quadra_view::{Div, div};

use crate::token;

/// The width of what the popover holds, and the height of the area.
pub(crate) const WIDE: f32 = 208.0;
const HIGH: f32 = 140.0;

/// A hue in degrees, a saturation and a brightness from 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Hsv {
    pub h: f32,
    pub s: f32,
    pub v: f32,
}

impl Hsv {
    pub fn color(self) -> Color {
        let c = self.v * self.s;
        let hh = self.h.rem_euclid(360.0) / 60.0;
        let x = c * (1.0 - (hh.rem_euclid(2.0) - 1.0).abs());
        let (r, g, b) = match hh as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = self.v - c;
        Color { r: r + m, g: g + m, b: b + m, a: 1.0 }
    }

    /// The hue, saturation and brightness of a colour; where it has no hue (a grey) or no
    /// saturation (black) it keeps those of `was`.
    pub fn of(color: Color, was: Hsv) -> Hsv {
        let Color { r, g, b, .. } = color;
        let (max, min) = (r.max(g).max(b), r.min(g).min(b));
        let d = max - min;
        let s = if max > 0.0 { d / max } else { was.s };
        let h = if d == 0.0 {
            was.h
        } else if max == r {
            60.0 * ((g - b) / d).rem_euclid(6.0)
        } else if max == g {
            60.0 * ((b - r) / d + 2.0)
        } else {
            60.0 * ((r - g) / d + 4.0)
        };
        Hsv { h, s, v: max }
    }

    /// `#rrggbb`, in lower case.
    pub fn hex(self) -> String {
        hex(self.color())
    }
}

/// `#rrggbb` of a colour, in lower case: what the picker writes.
pub(crate) fn hex(color: Color) -> String {
    color.hex()[..7].to_string()
}

/// A ring the knobs are: 2 px of white inside a pixel of black at 40 %, seen on any colour.
fn ring(px: f32) -> Div {
    let ring = div().absolute().size(px, px).radius(px / 2.0).border(2.0).border_color(Color::WHITE);
    ring.outline(Color::BLACK.alpha(0.4))
}

/// The area of saturation (left to right) and brightness (top to bottom) at the hue of
/// `hsv`: a press or a drag in it gives `put` the colour under the pointer, the arrows step
/// it by 1 %.
pub(crate) fn area(hsv: Signal<Hsv>, put: impl Fn(Hsv) + Copy + 'static) -> Div {
    let clear = |color: Color| color.alpha(0.0);
    let hue = move || Hsv { h: hsv.get().h, s: 1.0, v: 1.0 }.color();
    let face = div().size(WIDE, HIGH).shrink(0.0).radius(token::RADIUS_MD).cursor(Cursor::Pointer);
    // A box's gradient stands in place of its fill: the two ramps are boxes over the hue.
    let face = face.bg(hue);
    let over = || div().absolute().left(0.0).top(0.0).right(0.0).bottom(0.0).radius(token::RADIUS_MD);
    let light = over().gradient(Gradient::linear(0.0, [Color::WHITE, clear(Color::WHITE)]));
    let dark = over().gradient(Gradient::linear(90.0, [clear(Color::BLACK), Color::BLACK]));
    let knob = ring(12.0).left(move || Length::Px(hsv.get().s * WIDE - 6.0));
    let knob = knob.top(move || Length::Px((1.0 - hsv.get().v) * HIGH - 6.0));
    let at = move |x: f32, y: f32| {
        let (s, v) = ((x / WIDE).clamp(0.0, 1.0), 1.0 - (y / HIGH).clamp(0.0, 1.0));
        put(Hsv { s, v, ..hsv.get_untracked() })
    };
    let held: Rc<Cell<bool>> = Rc::default();
    let (press, drag) = (held.clone(), held.clone());
    let face = face.on_pointer(PointerKind::Down, move |e| {
        press.set(true);
        e.capture_pointer();
        at(e.local.x, e.local.y);
    });
    let face = face.on_pointer(PointerKind::Move, move |e| {
        if drag.get() {
            at(e.local.x, e.local.y);
        }
    });
    let face = face.on_pointer(PointerKind::Up, move |_| held.set(false));
    let face = face.focusable().focus_visible(|s| s.outline(token::BORDER_FOCUSED));
    let step = move |ds: f32, dv: f32| {
        let now = hsv.get_untracked();
        put(Hsv { s: (now.s + ds).clamp(0.0, 1.0), v: (now.v + dv).clamp(0.0, 1.0), ..now });
    };
    let face = face.on_key_down(move |e| {
        let (ds, dv) = match e.key {
            Key::Left => (-0.01, 0.0),
            Key::Right => (0.01, 0.0),
            Key::Up => (0.0, 0.01),
            Key::Down => (0.0, -0.01),
            _ => return,
        };
        e.stop();
        step(ds, dv);
    });
    // A reader's steps are the brightness's.
    let face = face.on_semantic(move |action| match action {
        SemanticAction::Increment | SemanticAction::Decrement => {
            step(0.0, if *action == SemanticAction::Increment { 0.01 } else { -0.01 });
            true
        }
        _ => false,
    });
    let says = move || {
        let Hsv { s, v, .. } = hsv.get();
        format!("saturation {:.0} %, brightness {:.0} %", s * 100.0, v * 100.0)
    };
    let face = face.role(Role::Slider).label("Saturation and brightness".to_string()).value(says);
    face.child(light).child(dark).child(knob)
}

/// The bar of the hues, round from red: a slider of `hue` from 0 to 360.
pub(crate) fn hue_bar(hue: Signal<f32>) -> Div {
    let scale = Scale::new((0.0, 360.0), 1.0, 7.0, WIDE - 14.0);
    let hues = (0..=12).map(|i| Hsv { h: i as f32 * 30.0, s: 1.0, v: 1.0 }.color());
    let rail = div().absolute().left(0.0).top(1.0).size(WIDE, 12.0).radius(token::RADIUS_MD);
    let rail = rail.gradient(Gradient::linear(0.0, hues));
    let shown = move || Hsv { h: hue.get(), s: 1.0, v: 1.0 }.color();
    let knob = ring(14.0).top(0.0).bg(shown);
    let knob = knob.left(move || Length::Px(scale.track * scale.part(hue.get())));
    let face = div().size(WIDE, 14.0).shrink(0.0).cursor(Cursor::Pointer).label("Hue".to_string());
    // design: the knob is a ring, half its 14 px
    let face = face.radius(7.0).focus_visible(|s| s.outline(token::BORDER_FOCUSED));
    slider::slider(face, hue, scale, memo(|| false)).child(rail).child(knob)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_colour_comes_back_from_its_hue_saturation_and_brightness() {
        let was = Hsv { h: 10.0, s: 0.5, v: 0.5 };
        for hex in ["#e06c75", "#61afef", "#98c379", "#000000", "#ffffff", "#808080", "#ff00ff"] {
            let color = Color::parse_hex(hex).unwrap();
            assert_eq!(Hsv::of(color, was).hex(), hex);
        }
        // Grey keeps the hue it had, black the saturation too.
        assert_eq!(Hsv::of(Color::parse_hex("#808080").unwrap(), was).h, 10.0);
        assert_eq!(Hsv::of(Color::BLACK, was), Hsv { h: 10.0, s: 0.5, v: 0.0 });
        assert_eq!(Hsv { h: 120.0, s: 1.0, v: 1.0 }.hex(), "#00ff00");
    }
}
