//! A table of a document: a grid with lines, the head on the tone of the chrome, every
//! other row on the panel's.

use quadra_design::{fonts, size, token};
use quadra_layout::{Justify, Track, TrackSize};
use quadra_text::{Paragraph, TextStyle};
use quadra_tree::{NodeId, Role};
use quadra_view::{MountCx, View, div, row};

use super::{BODY, OnLink, rich, spans};
use crate::block::Align;
use crate::outline::plain;

/// How many columns a table has at most: a document that is not ours does not say how wide
/// the page is.
const WIDEST: usize = 64;
/// The padding of a cell, and a little for runs that are bold or code.
const PADDING: f32 = 24.0;

pub struct Table {
    pub align: Vec<Align>,
    pub rows: Vec<Vec<String>>,
    pub on_link: Option<OnLink>,
}

impl Table {
    /// What each column asks for: its longest cell on one line. The columns share the width
    /// of the table by it, and none is narrower than its longest word.
    fn wants(&self, columns: usize, cx: &mut MountCx) -> Vec<f32> {
        let (fonts, mut wants, scale) = (cx.tree.fonts(), vec![0.0f32; columns], cx.tree.text_scale());
        let text = &mut fonts.borrow_mut().text;
        for (r, cells) in self.rows.iter().enumerate() {
            let weight = if r == 0 { size::WEIGHT_STRONG } else { size::WEIGHT_NORMAL };
            let style =
                TextStyle { family: fonts::UI.into(), size: BODY * scale, weight, ..TextStyle::default() };
            for (want, cell) in wants.iter_mut().zip(cells) {
                let wide = Paragraph::new(text, plain(cell), style.clone()).wrap(text, f32::INFINITY).width;
                *want = want.max(wide);
            }
        }
        wants
    }
}

impl View for Table {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let columns = self.rows.iter().map(Vec::len).max().unwrap_or(0).clamp(1, WIDEST);
        let share = |want: f32| Track::minmax(TrackSize::Auto, TrackSize::Fr(want + PADDING));
        let tracks: Vec<Track> = self.wants(columns, cx).into_iter().map(share).collect();
        let mut grid = div().grid(tracks).grow(1.0).border(1.0).radius(token::RADIUS_MD).role(Role::Table);
        grid = grid.border_color(token::LINE_STRONG);
        for (r, cells) in self.rows.iter().enumerate() {
            for c in 0..columns {
                let cell =
                    row().padding_x(10.0).padding_y(4.0).role(Role::Cell).justify(match self.align.get(c) {
                        Some(Align::Center) => Justify::Center,
                        Some(Align::Right) => Justify::End,
                        _ => Justify::Start,
                    });
                let cell = match r {
                    0 => cell.bg(token::CHROME).font_weight(size::WEIGHT_STRONG),
                    _ if r % 2 == 0 => cell.bg(token::PANEL).border_top(1.0).border_color(token::LINE_STRONG),
                    _ => cell.border_top(1.0).border_color(token::LINE_STRONG),
                };
                grid = grid.child(match cells.get(c).map(|cell| spans(cell)) {
                    Some(runs) if !runs.is_empty() => cell.child(rich(runs, &self.on_link)),
                    _ => cell,
                });
            }
        }
        grid.mount(cx)
    }
}
