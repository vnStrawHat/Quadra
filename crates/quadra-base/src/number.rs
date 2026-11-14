//! The number input (specs/17, "The components"): a field that holds a number. A number
//! typed, with a point or a comma, is the value; what is no number changes nothing and is not
//! accepted; a value written from elsewhere is what the field says, unless it says it
//! already. The up and down arrows and two buttons step it, within its ends. The look gives
//! the field, its box and the buttons.

use quadra_reactive::{Signal, effect, signal};
use quadra_tree::Role;
use quadra_ui::Div;

use crate::action::{Decrease, Increase, NUMBER};

/// What a text says as a number: a comma is a point too.
pub fn parse(text: &str) -> Option<f64> {
    text.trim().replace(',', ".").parse().ok().filter(|v: &f64| v.is_finite())
}

/// A number and the text of its field, which follow each other.
#[derive(Clone, Copy)]
pub struct Number {
    pub value: Signal<f64>,
    /// What the field says: give it to the look's field.
    pub words: Signal<String>,
    ends: (f64, f64),
    step: f64,
}

/// The number of `value` stepped by `step` between `ends`, written with `digits` after the
/// point.
pub fn number(value: Signal<f64>, ends: (f64, f64), step: f64, digits: usize) -> Number {
    let show = move |v: f64| format!("{v:.digits$}");
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
    Number { value, words, ends, step }
}

impl Number {
    /// A step up (1) or down (-1), within the ends.
    pub fn bump(self, way: f64) {
        let ((min, max), by) = (self.ends, self.step);
        self.value.update(|v| *v = (*v + way * by).clamp(min, max));
    }

    /// Whether what the field says is no number: a reactive read, for `.invalid(..)`.
    pub fn wrong(self) -> bool {
        parse(&self.words.get()).is_none()
    }

    /// The keys of the number on the box around its field: the up and down arrows step it.
    pub fn keys(self, around: Div) -> Div {
        let around = around.key_context(NUMBER).on_action::<Increase>(move |_| self.bump(1.0));
        around.on_action::<Decrease>(move |_| self.bump(-1.0))
    }

    /// A button on its face that steps the number up, or down: "Increment" or "Decrement" to a
    /// reader.
    pub fn stepper(self, face: Div, up: bool) -> Div {
        let name = if up { "Increment" } else { "Decrement" };
        let face = face.role(Role::Button).label(name.to_string());
        face.on_click(move |_| self.bump(if up { 1.0 } else { -1.0 }))
    }
}
