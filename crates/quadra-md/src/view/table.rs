//! A table of a document: a grid with lines, the head on the tone of the chrome, every
//! other row on the panel's.

use quadra_design::{fonts, size, token};
use quadra_layout::{Justify, Length, Track, TrackSize};
use quadra_prim::Corners;
use quadra_reactive::signal;
use quadra_text::{Paragraph, TextStyle};
use quadra_tree::{NodeId, Role};
use quadra_view::{MountCx, View, div, row, scroll_x};

use super::{BODY, OnLink, rich, spans};
use crate::block::Align;
use crate::outline::plain;

/// How many columns a table has at most: a document that is not ours does not say how wide
/// the page is.
const WIDEST: usize = 64;
/// The padding of a cell, and a little for runs that are bold or code.
const PADDING: f32 = 24.0;
/// A column is at least as wide as its longest word up to this: a longer word (an address)
/// is broken inside its cell rather than the table scrolled.
const LONG_WORD: f32 = 200.0;

pub struct Table {
    pub align: Vec<Align>,
    pub rows: Vec<Vec<String>>,
    pub on_link: Option<OnLink>,
}

impl Table {
    /// What each column asks for: its longest cell on one line, and its longest word. The
    /// columns share the width of the table by the first, and none is narrower than the second.
    fn wants(&self, columns: usize, cx: &mut MountCx) -> (Vec<f32>, Vec<f32>) {
        let (fonts, scale) = (cx.tree.fonts(), cx.tree.text_scale());
        let (mut wants, mut least) = (vec![0.0f32; columns], vec![0.0f32; columns]);
        let text = &mut fonts.borrow_mut().text;
        for (r, cells) in self.rows.iter().enumerate() {
            let weight = if r == 0 { size::WEIGHT_STRONG } else { size::WEIGHT_NORMAL };
            let style =
                TextStyle { family: fonts::UI.into(), size: BODY * scale, weight, ..TextStyle::default() };
            for ((want, word), cell) in wants.iter_mut().zip(least.iter_mut()).zip(cells) {
                let words = plain(cell);
                let mut line =
                    |it: &str| Paragraph::new(text, it, style.clone()).wrap(text, f32::INFINITY).width;
                *want = want.max(line(&words));
                // The longest words by their letters, of which the widest drawn.
                let mut long: Vec<&str> = words.split_whitespace().collect();
                long.sort_by_key(|it| std::cmp::Reverse(it.chars().count()));
                *word = long.into_iter().take(3).map(&mut line).fold(*word, f32::max);
            }
        }
        (wants, least)
    }
}

impl View for Table {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let columns = self.rows.iter().map(Vec::len).max().unwrap_or(0).clamp(1, WIDEST);
        let (wants, least) = self.wants(columns, cx);
        let least: Vec<f32> = least.into_iter().map(|word| word.min(LONG_WORD) + PADDING).collect();
        let share =
            |(want, least): (f32, &f32)| Track::minmax(TrackSize::Px(*least), TrackSize::Fr(want + PADDING));
        let tracks: Vec<Track> = wants.into_iter().zip(&least).map(share).collect();
        // As wide as the page, and wider (scrolled sideways) only when its longest words do not
        // fit beside each other: prose in a cell wraps.
        let narrowest = least.iter().sum::<f32>() + 2.0;
        let mut grid =
            div().grid(tracks).width(Length::Fill).border(1.0).radius(token::RADIUS_MD).role(Role::Table);
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
                // The cells at the frame's corners are rounded inside it: their fill does not
                // cover its line.
                let cell = cell.radii(corner(r, c, self.rows.len(), columns));
                grid = grid.child(match cells.get(c).map(|cell| spans(cell)) {
                    Some(runs) if !runs.is_empty() => cell.child(rich(runs, &self.on_link)),
                    _ => cell,
                });
            }
        }
        let page = row().width(Length::Fill).min_width(Length::Px(narrowest)).child(grid);
        scroll_x(signal(0.0), page).mount(cx)
    }
}

/// The corners of the cell at row `r` and column `c` that are those of the table's frame,
/// rounded by the frame's radius less its line.
fn corner(r: usize, c: usize, rows: usize, columns: usize) -> Corners {
    let radius = size::RADIUS_MD - 1.0;
    let at = |yes: bool| if yes { radius } else { 0.0 };
    let (top, bottom, left, right) = (r == 0, r + 1 == rows, c == 0, c + 1 == columns);
    Corners {
        top_left: at(top && left),
        top_right: at(top && right),
        bottom_right: at(bottom && right),
        bottom_left: at(bottom && left),
    }
}
