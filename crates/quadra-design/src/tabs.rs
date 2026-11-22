//! Tabs (specs/13, "Components"): the names of a few pages in a row, one of them chosen. The
//! caller shows the page of the chosen one. Underlined by default, the chosen tab with the
//! accent's line; `.pill()` tints the chosen tab and draws no line.

use quadra_layout::Align;
use quadra_platform::Cursor;
use quadra_prim::Color;
use quadra_reactive::Signal;
use quadra_tree::{NodeId, Side, State};
use quadra_view::{Div, MountCx, Refine, View, by_text, div, row, text};

use crate::button::Size;
use crate::choice::chooser;
use crate::display::chosen_mark;
use crate::token;

pub struct Tabs {
    labels: Vec<String>,
    chosen: Signal<usize>,
    pill: bool,
}

/// A row of tabs: `chosen` holds which one is, and a click or the arrows write it.
pub fn tabs<S: Into<String>>(labels: impl IntoIterator<Item = S>, chosen: Signal<usize>) -> Tabs {
    Tabs { labels: labels.into_iter().map(Into::into).collect(), chosen, pill: false }
}

impl Tabs {
    /// The chosen tab is tinted, the others are apart, and no line runs under the row.
    pub fn pill(mut self) -> Self {
        self.pill = true;
        self
    }

    fn tab(&self, i: usize, label: String) -> Div {
        let (chosen, pill) = (self.chosen, self.pill);
        let tab =
            row().align_items(Align::Center).shrink(0.0).height(by_text(Size::Md.height())).padding_x(8.0);
        let tab = tab.radius(token::RADIUS_MD).cursor(Cursor::Pointer);
        let tab = tab.text_color(if pill { token::TEXT } else { token::TEXT_2 });
        // Over the line under the row a fill would sit as a box on it: the label alone lights
        // up. A pill is a box already, and fills.
        let tab = match pill {
            true => tab.hover(|s| s.bg(token::PANEL_HOVER)),
            false => tab.hover(|s| s.text_color(token::TEXT)),
        };
        // The chosen one has the accent's label whatever the pointer does.
        let on = move |s: Refine| {
            let s = s.text_color(token::ACCENT_TEXT);
            if pill { s.bg(token::ACCENT_TINT) } else { s.bg(Color::TRANSPARENT) }
        };
        let tab = quadra_headless::tab(tab.when(State::SELECTED, on), i, chosen).child(text(label));
        if pill { tab } else { tab.child(chosen_mark(Side::Below, move || chosen.get() == i)) }
    }
}

impl View for Tabs {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let strip = chooser(quadra_headless::tabs(div(), self.labels.len(), self.chosen));
        let strip = match self.pill {
            true => strip.gap(4.0),
            false => strip.border_bottom(1.0).border_color(token::LINE),
        };
        let all = self.labels.iter().cloned().enumerate().map(|(i, label)| self.tab(i, label));
        strip.children(all.collect::<Vec<_>>()).mount(cx)
    }
}
