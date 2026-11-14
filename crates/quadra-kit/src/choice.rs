//! One choice of a few (specs/behaviour/15, "Components"): the radio group and the segmented
//! control. Each takes the signal that holds which option is chosen, and writes it.

use quadra_layout::{Align, Direction, Justify, Overflow};
use quadra_platform::Cursor;
use quadra_reactive::Signal;
use quadra_tree::{NodeId, Paint, Role, State};
use quadra_ui::{AnyView, Div, IntoProp, MountCx, Prop, View, div, row, show, text};

use crate::action::{CHOICE, MoveNext, MovePrevious};
use crate::button::Size;
use crate::{size, token};

pub struct Choice {
    options: Vec<String>,
    chosen: Signal<usize>,
    segmented: bool,
    horizontal: bool,
    size: Size,
    disabled: Prop<bool>,
}

/// Options one under the other, each with a ring: `chosen` holds which one has the dot.
pub fn radio_group<S: Into<String>>(options: impl IntoIterator<Item = S>, chosen: Signal<usize>) -> Choice {
    Choice {
        options: options.into_iter().map(Into::into).collect(),
        chosen,
        segmented: false,
        horizontal: false,
        size: Size::Sm,
        disabled: Prop::Value(false),
    }
}

/// Options side by side as the cells of one outlined control: the chosen cell is tinted.
pub fn segmented<S: Into<String>>(options: impl IntoIterator<Item = S>, chosen: Signal<usize>) -> Choice {
    Choice { segmented: true, horizontal: true, ..radio_group(options, chosen) }
}

impl Choice {
    /// The options of a radio group side by side.
    pub fn horizontal(mut self) -> Self {
        self.horizontal = true;
        self
    }

    /// The height of the cells of a segmented control: 22 px with 12 px labels at the two
    /// smaller sizes, 28 or 32 with 14 px labels.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn disabled<M>(mut self, disabled: impl IntoProp<bool, M>) -> Self {
        self.disabled = disabled.into_prop();
        self
    }

    /// An option of a radio group: a ring of 12 px, of the accent with a dot of 6 in it when
    /// chosen.
    fn radio(label: String, on: impl Fn() -> bool + Copy + 'static) -> Div {
        let dot = || div().size(6.0, 6.0).radius(3.0).bg(token::ACCENT);
        let edge = move || if on() { token::ACCENT } else { token::LINE_STRONG };
        let ring = div()
            .size(12.0, 12.0)
            .shrink(0.0)
            .radius(6.0)
            .border(1.0)
            .border_color(move || Paint::from(edge()));
        let ring = ring.bg(token::CANVAS).align_items(Align::Center).justify(Justify::Center);
        row().align_items(Align::Center).gap(6.0).child(ring.child(show(on, dot))).child(text(label))
    }

    /// A cell of a segmented control: tinted, with an accent label, when chosen.
    fn cell(&self, label: String) -> Div {
        let small = matches!(self.size, Size::Xs | Size::Sm);
        let cell = row().align_items(Align::Center).justify(Justify::Center).shrink(0.0);
        let cell = cell.height(self.size.height()).padding_x(if small { 8.0 } else { 12.0 });
        let cell = match small {
            true => cell.text_size(size::TEXT_SM.size).line_height(size::TEXT_SM.line_height()),
            false => cell,
        };
        let cell = cell.hover(|s| s.bg(token::PANEL_HOVER));
        let chosen = |s: quadra_ui::Refine| s.bg(token::ACCENT_TINT).text_color(token::ACCENT_TEXT);
        cell.when(State::CHECKED, chosen).child(text(label))
    }

    fn build(self) -> Div {
        let (chosen, count) = (self.chosen, self.options.len());
        let mut parts: Vec<AnyView> = Vec::new();
        for (i, label) in self.options.iter().cloned().enumerate() {
            let on = move || chosen.get() == i;
            if self.segmented && i > 0 {
                // A hairline between two cells, as tall as they are.
                parts.push(AnyView::new(div().width(1.0).bg(token::LINE_STRONG)));
            }
            let option = if self.segmented { self.cell(label) } else { Choice::radio(label, on) };
            let option = option.role(Role::Radio).checked(on).cursor(Cursor::Pointer);
            parts.push(AnyView::new(option.on_click(move |_| chosen.set(i))));
        }
        let group = chooser(Role::Group, count, chosen);
        let group =
            group.disabled(self.disabled).when(State::DISABLED, |s| s.text_color(token::TEXT_DISABLED));
        let group = match (self.segmented, self.horizontal) {
            (true, _) => {
                let outlined = group.radius(size::RADIUS_LG).border(1.0).shrink(0.0);
                outlined.border_color(token::LINE_STRONG).overflow_y(Overflow::Hidden)
            }
            (false, true) => group.gap(16.0),
            (false, false) => group.direction(Direction::Column).gap(8.0),
        };
        group.children(parts)
    }
}

/// The box around a few options of which one is chosen: it is what Tab reaches, and the
/// arrows move the choice, round the ends.
pub(crate) fn chooser(role: Role, count: usize, chosen: Signal<usize>) -> Div {
    let step = move |by: usize| chosen.update(|at| *at = (*at + by) % count.max(1));
    let group = div().role(role).focusable().key_context(CHOICE);
    let group = group.on_action::<MoveNext>(move |_| step(1));
    let group = group.on_action::<MovePrevious>(move |_| step(count.saturating_sub(1)));
    group.focus_visible(|s| s.outline(token::BORDER_FOCUSED))
}

impl View for Choice {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        self.build().mount(cx)
    }
}
