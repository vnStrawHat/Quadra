//! The number input (specs/behaviour/15, "Number input"): a field that holds a number, with
//! a pair of chevrons at its end, inside it, or as a stepper: one frame with a minus, the
//! value and a plus. The up and down arrows step it.

use quadra_base::number::{Number, number};
use quadra_layout::{Align, Direction, Justify};
use quadra_platform::Cursor;
use quadra_reactive::Signal;
use quadra_tree::NodeId;
use quadra_ui::{Div, MountCx, View, column, div, row, text_input};

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

/// A button that steps the number up or down: `around` is its box, `body` its icon at `px`.
fn step(around: Div, number: Number, up: bool, body: &'static str, px: f32) -> Div {
    let around = around.align_items(Align::Center).justify(Justify::Center).shrink(0.0);
    number.stepper(around.cursor(Cursor::Pointer), up).child(icon(body).size(px))
}

impl View for NumberInput {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let number = number(self.value, self.ends, self.step, self.decimals);
        let (words, wrong) = (number.words, move || number.wrong());
        let around = number.keys(row().align_items(Align::Center).shrink(0.0));
        if self.stepper {
            let field = text_input(words).fit(); // centred between the minus and the plus
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
            let frame = frame.child(step(side(), number, false, icons::MINUS, 16.0)).child(middle);
            return frame.child(step(side(), number, true, icons::PLUS, 16.0)).mount(cx);
        }
        let field = input(words).width(140.0).invalid(wrong);
        let field = match self.test_id {
            Some(id) => field.test_id(id),
            None => field,
        };
        // Two halves of the field's height at its end, inside its frame: the frame is 32 px,
        // and its padding less above and below makes room for both.
        let small = || {
            let around = div().size(18.0, 13.0).radius(size::RADIUS_SM).text_color(token::TEXT_2);
            around.direction(Direction::Row).hover(|s| s.bg(token::PANEL_HOVER))
        };
        let up = step(small(), number, true, icons::CHEVRON_UP, 10.0);
        let down = step(small(), number, false, icons::CHEVRON_DOWN, 10.0);
        let field = field.build().padding_y(2.0).padding_right(2.0);
        let field = field.child(column().shrink(0.0).child(up).child(down));
        around.child(field).mount(cx)
    }
}
