//! The number input (specs/design, "Number input"): a field that holds a number, with
//! a pair of chevrons at its end, inside it, or as a stepper: one frame with a minus, the
//! value and a plus. The up and down arrows step it.

use quadra_headless::number::{Number, number};
use quadra_layout::{Align, Direction, Justify};
use quadra_platform::Cursor;
use quadra_reactive::Signal;
use quadra_tree::NodeId;
use quadra_view::{Div, IntoProp, MountCx, Prop, View, by_text, column, div, row, text_input};

use crate::icon::icon;
use crate::input::input;
use crate::{icons, size, token};

pub struct NumberInput {
    value: Signal<f64>,
    ends: (f64, f64),
    step: f64,
    decimals: usize,
    stepper: bool,
    label: Option<Prop<String>>,
    test_id: Option<&'static str>,
    width: Option<f32>,
}

/// A field that edits `value`: whole numbers in steps of 1 unless told otherwise.
pub fn number_input(value: Signal<f64>) -> NumberInput {
    NumberInput {
        value,
        ends: (-1.0e15, 1.0e15),
        step: 1.0,
        decimals: 0,
        stepper: false,
        label: None,
        test_id: None,
        width: None,
    }
}

impl NumberInput {
    /// How wide it is: its own width, or its column's in a form, unless told (a port: 96).
    pub fn width(mut self, px: f32) -> Self {
        self.width = Some(px);
        self
    }

    /// The least and the most the steps go to.
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.ends = (min, max);
        self
    }

    /// The step of the chevrons and the arrows; the field shows at least its digits.
    pub fn step(mut self, step: f64) -> Self {
        let digits = format!("{step}").split_once('.').map_or(0, |(_, after)| after.len());
        (self.step, self.decimals) = (step, self.decimals.max(digits));
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

    /// What a reader calls the field, a constant or words that change.
    pub fn label<M>(mut self, name: impl IntoProp<String, M>) -> Self {
        self.label = Some(name.into_prop());
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
            let field = match self.label {
                Some(name) => field.label(name),
                None => field,
            };
            let ghost = |around: Div| {
                around.hover(|s| s.bg(token::LINE.alpha(0.4))).active(|s| s.bg(token::LINE.alpha(0.6)))
            };
            let side = || ghost(row().width(32.0).height(by_text(size::H_MD - 2.0)));
            let middle = row().grow(1.0).justify(Justify::Center).cursor(Cursor::Text).child(field);
            let frame = crate::layout::field_width(around, self.width, 128.0)
                .height(by_text(size::H_MD))
                .radius(token::RADIUS_MD)
                .border(1.0);
            let frame = frame.bg(token::INPUT_BG).border_color(token::LINE_STRONG);
            let frame = frame.overflow_x(quadra_layout::Overflow::Hidden).selection_color(token::SELECTION);
            let frame = frame.focus_within(|s| s.border_color(token::BORDER_FOCUSED));
            let frame =
                frame.invalid(wrong).when(quadra_tree::State::INVALID, |s| s.border_color(token::ERROR));
            let frame = frame.child(step(side(), number, false, icons::MINUS, 16.0)).child(middle);
            return frame.child(step(side(), number, true, icons::PLUS, 16.0)).mount(cx);
        }
        let field = input(words).own_width(140.0).invalid(wrong);
        let field = match self.width {
            Some(px) => field.width(px),
            None => field,
        };
        let field = match self.test_id {
            Some(id) => field.test_id(id),
            None => field,
        };
        let field = match self.label {
            Some(name) => field.label(name),
            None => field,
        };
        // Two halves of the field's height at its end, inside its frame: the frame is 28 px,
        // 22 inside its border and the padding above and below, 11 a half.
        let small = || {
            let around = div().size(18.0, 11.0).radius(token::RADIUS_SM).text_color(token::TEXT_2);
            around.direction(Direction::Row).hover(|s| s.bg(token::PANEL_HOVER))
        };
        let up = step(small(), number, true, icons::CHEVRON_UP, 10.0);
        let down = step(small(), number, false, icons::CHEVRON_DOWN, 10.0);
        let field = field.build().padding_y(2.0).padding_right(2.0);
        let field = field.child(column().shrink(0.0).child(up).child(down));
        around.child(field).mount(cx)
    }
}
