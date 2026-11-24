//! One choice of a few (specs/design, "Components"): the radio group and the segmented
//! control. Each takes the signal that holds which option is chosen, and writes it.

use quadra_layout::{Align, Direction, Justify, Overflow};
use quadra_platform::Cursor;
use quadra_prim::Corners;
use quadra_reactive::Signal;
use quadra_tree::{NodeId, Paint, State};
use quadra_view::{
    AnyView, Div, IntoProp, MountCx, NodeRef, Prop, View, by_text, column, div, row, show, text,
};

use crate::button::Size;
use crate::{size, token};

pub struct Choice {
    options: Vec<String>,
    hints: Vec<String>,
    chosen: Signal<usize>,
    segmented: bool,
    horizontal: bool,
    size: Size,
    disabled: Prop<bool>,
    label: Option<Prop<String>>,
}

/// Options one under the other, each with a ring: `chosen` holds which one has the dot.
pub fn radio_group<S: Into<String>>(options: impl IntoIterator<Item = S>, chosen: Signal<usize>) -> Choice {
    Choice {
        options: options.into_iter().map(Into::into).collect(),
        hints: Vec::new(),
        chosen,
        segmented: false,
        horizontal: false,
        size: Size::Md,
        disabled: Prop::Value(false),
        label: None,
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

    /// The height of a segmented control with its border: the md control height (28) with
    /// 14 px labels unless told, 22 with 12 px labels at the two smaller sizes, or 32.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn disabled<M>(mut self, disabled: impl IntoProp<bool, M>) -> Self {
        self.disabled = disabled.into_prop();
        self
    }

    /// A muted line under each option of a radio group that says more of it, in their
    /// order ("" for none): under its label, and to a reader its description, not its name.
    pub fn hints<S: Into<String>>(mut self, hints: impl IntoIterator<Item = S>) -> Self {
        self.hints = hints.into_iter().map(Into::into).collect();
        self
    }

    /// What a reader calls the group, a constant or words that change.
    pub fn label<M>(mut self, name: impl IntoProp<String, M>) -> Self {
        self.label = Some(name.into_prop());
        self
    }

    /// An option of a radio group: a ring of 12 px, of the accent with a dot of 6 in it when
    /// chosen.
    fn radio(label: String, hint: Option<&String>, on: impl Fn() -> bool + Copy + 'static) -> Div {
        let dot = || div().size(6.0, 6.0).radius(3.0).bg(token::ACCENT); // design: a radio's dot is a circle
        let edge = move || if on() { token::ACCENT } else { token::LINE_STRONG };
        let ring = div()
            .size(12.0, 12.0)
            .shrink(0.0)
            .radius(6.0)
            .border(1.0)
            .border_color(move || Paint::from(edge()));
        let ring = ring.bg(token::CANVAS).align_items(Align::Center).justify(Justify::Center);
        let line = row().align_items(Align::Center).gap(6.0).child(ring.child(show(on, dot)));
        let Some(hint) = hint.filter(|hint| !hint.is_empty()) else { return line.child(text(label)) };
        // Under the label, past the ring and its gap; the option is named by its label alone.
        let told = NodeRef::new();
        // design: under the words, past the ring of 12 and its gap of 6
        let under = div().padding_left(18.0).text_size(size::TEXT_SM.size).text_color(token::TEXT_3);
        let under = under.node_ref(&told).child(text(hint.clone()));
        column().gap(2.0).label(label.clone()).described_by(&told).child(line.child(text(label))).child(under)
    }

    /// A cell of a segmented control: tinted, with an accent label, when chosen.
    fn cell(&self, label: String) -> Div {
        let small = matches!(self.size, Size::Xs | Size::Sm);
        let cell = row().align_items(Align::Center).justify(Justify::Center).shrink(0.0);
        let cell = if crate::layout::fills() { cell.grow(1.0) } else { cell };
        // Inside the group's border: the group is the control's height.
        let cell = cell.height(by_text(self.size.height() - 2.0)).padding_x(if small { 8.0 } else { 12.0 });
        let cell = match small {
            true => cell.text_size(size::TEXT_SM.size).line_height(size::TEXT_SM.line_height()),
            false => cell,
        };
        let cell = cell.hover(|s| s.bg(token::PANEL_HOVER));
        let chosen = |s: quadra_view::Refine| s.bg(token::ACCENT_TINT).text_color(token::ACCENT_TEXT);
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
            let option = match self.segmented {
                // The frame does not cut what it holds: the cells at its ends are rounded as
                // its inside is, so a chosen one's fill stays in it.
                true => self.cell(label).radii(ends(i, count)),
                false => Choice::radio(label, self.hints.get(i), on),
            };
            parts.push(AnyView::new(quadra_headless::radio(option.cursor(Cursor::Pointer), i, chosen)));
        }
        let group = chooser(quadra_headless::radio_group(div(), count, chosen));
        let group = match self.label {
            Some(name) => group.label(name),
            None => group,
        };
        let group =
            group.disabled(self.disabled).when(State::DISABLED, |s| s.text_color(token::TEXT_DISABLED));
        let group = match (self.segmented, self.horizontal) {
            (true, _) => {
                let outlined = group.radius(token::RADIUS_MD).border(1.0).shrink(0.0);
                let outlined = match crate::layout::fills() {
                    true => outlined.width(quadra_layout::Length::Fill),
                    false => outlined,
                };
                outlined.border_color(token::LINE_STRONG).overflow_y(Overflow::Hidden)
            }
            (false, true) => group.gap(16.0),
            (false, false) => group.direction(Direction::Column).gap(8.0),
        };
        group.children(parts)
    }
}

/// The corners of the cell at `i` of `count` in a frame of `RADIUS_MD` with a border of 1.
fn ends(i: usize, count: usize) -> Corners {
    let r = size::RADIUS_MD - 1.0;
    let (first, last) = (if i == 0 { r } else { 0.0 }, if i + 1 == count { r } else { 0.0 });
    Corners { top_left: first, bottom_left: first, top_right: last, bottom_right: last }
}

/// The box around a few options of which one is chosen (a radio group, tabs), with the focus
/// as a line around it.
pub(crate) fn chooser(group: Div) -> Div {
    group.focus_visible(|s| s.outline(token::BORDER_FOCUSED))
}

impl View for Choice {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        self.build().mount(cx)
    }
}
