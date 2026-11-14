//! The number input (specs/behaviour/15, "Number input"): a field that holds a number, with
//! a pair of chevrons beside it, or as a stepper: one frame with a minus, the value and a
//! plus. The up and down arrows step it.

use quadra_layout::{Align, Direction, Justify};
use quadra_platform::Cursor;
use quadra_reactive::{Signal, effect, signal};
use quadra_tree::{NodeId, Role};
use quadra_ui::{Div, MountCx, View, column, div, row, text_input};

use crate::action::{Decrease, Increase, NUMBER};
use crate::icon::icon;
use crate::input::input;
use crate::{icons, size, token};

pub struct NumberInput {
    value: Signal<f64>,
    ends: (f64, f64),
    step: f64,
    decimals: usize,
    stepper: bool,
    test_id: Option<&'static str>,
}

/// A field that edits `value`: whole numbers in steps of 1 unless told otherwise.
pub fn number_input(value: Signal<f64>) -> NumberInput {
    NumberInput { value, ends: (-1.0e15, 1.0e15), step: 1.0, decimals: 0, stepper: false, test_id: None }
}

impl NumberInput {
    /// The least and the most the steps go to.
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.ends = (min, max);
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// How many digits show after the point.
    pub fn decimals(mut self, digits: usize) -> Self {
        self.decimals = digits;
        self
    }

    /// One framed control of 128 by 32 px: a minus, the value in the middle, a plus.
    pub fn stepper(mut self) -> Self {
        self.stepper = true;
        self
    }

    pub fn test_id(mut self, id: &'static str) -> Self {
        self.test_id = Some(id);
        self
    }
}

/// What a text says as a number: a comma is a point too.
fn parse(text: &str) -> Option<f64> {
    text.trim().replace(',', ".").parse().ok().filter(|v: &f64| v.is_finite())
}

/// A button that steps: `around` is its box, `body` its icon at `px`.
fn step(around: Div, name: &'static str, body: &'static str, px: f32, press: impl Fn() + 'static) -> Div {
    let around = around.align_items(Align::Center).justify(Justify::Center).shrink(0.0);
    let around = around.cursor(Cursor::Pointer).role(Role::Button).label(name.to_string());
    around.on_click(move |_| press()).child(icon(body).size(px))
}

impl View for NumberInput {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (value, (min, max), by, digits) = (self.value, self.ends, self.step, self.decimals);
        let show = move |v: f64| format!("{v:.digits$}");
        // The text and the number follow each other: typing a number writes it, and a
        // number written from elsewhere is what the text says, unless it says it already.
        let words = signal(show(value.get_untracked()));
        effect(move || {
            let v = value.get();
            if parse(&words.get_untracked()) != Some(v) {
                words.set(show(v));
            }
        });
        effect(move || {
            if let Some(v) = parse(&words.get()).filter(|v| *v != value.get_untracked()) {
                value.set(v);
            }
        });
        let bump = move |way: f64| value.update(|v| *v = (*v + way * by).clamp(min, max));
        let wrong = move || parse(&words.get()).is_none();
        let around = row().align_items(Align::Center).shrink(0.0).key_context(NUMBER);
        let around =
            around.on_action::<Increase>(move |_| bump(1.0)).on_action::<Decrease>(move |_| bump(-1.0));
        if self.stepper {
            let field = text_input(words);
            let field = match self.test_id {
                Some(id) => field.test_id(id),
                None => field,
            };
            let ghost = |around: Div| {
                around.hover(|s| s.bg(token::LINE.alpha(0.4))).active(|s| s.bg(token::LINE.alpha(0.6)))
            };
            let side = || ghost(row().width(32.0).height(30.0));
            let middle = row().grow(1.0).justify(Justify::Center).cursor(Cursor::Text).child(field);
            let frame = around.size(128.0, 32.0).radius(size::RADIUS_LG).border(1.0);
            let frame = frame.bg(token::INPUT_BG).border_color(token::LINE_STRONG);
            let frame = frame.overflow_x(quadra_layout::Overflow::Hidden).selection_color(token::SELECTION);
            let frame = frame.focus_within(|s| s.border_color(token::BORDER_FOCUSED));
            let frame =
                frame.invalid(wrong).when(quadra_tree::State::INVALID, |s| s.border_color(token::ERROR));
            let frame = frame.child(step(side(), "Decrement", icons::MINUS, 16.0, move || bump(-1.0)));
            let frame = frame.child(middle);
            return frame.child(step(side(), "Increment", icons::PLUS, 16.0, move || bump(1.0))).mount(cx);
        }
        let field = input(words).width(140.0).invalid(wrong);
        let field = match self.test_id {
            Some(id) => field.test_id(id),
            None => field,
        };
        let small = || {
            let around = div().size(16.0, 16.0).radius(size::RADIUS_SM).text_color(token::TEXT_2);
            around.direction(Direction::Row).hover(|s| s.bg(token::PANEL_HOVER))
        };
        let up = step(small(), "Increment", icons::CHEVRON_UP, 12.0, move || bump(1.0));
        let down = step(small(), "Decrement", icons::CHEVRON_DOWN, 12.0, move || bump(-1.0));
        around.gap(4.0).width(160.0).child(field).child(column().child(up).child(down)).mount(cx)
    }
}
