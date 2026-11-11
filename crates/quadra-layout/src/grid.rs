//! Grid style (specs/behaviour/05, "Style"): track definitions and placement, a subset of CSS
//! grid. Not supported: subgrid, masonry, baseline alignment.

use std::collections::HashMap;

use crate::style::{Align, Justify};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum TrackSize {
    #[default]
    Auto,
    Px(f32),
    /// A percentage of the container's size on that axis.
    Pct(f32),
    /// A share of the space left over.
    Fr(f32),
    MinContent,
    MaxContent,
}

impl TrackSize {
    /// The size when it does not depend on content: pixels, or a percentage of a known size.
    pub(crate) fn fixed(self, inner: Option<f32>) -> Option<f32> {
        match self {
            TrackSize::Px(v) => Some(v),
            TrackSize::Pct(p) => inner.map(|size| p / 100.0 * size),
            _ => None,
        }
    }

    pub(crate) fn is_fixed_unit(self) -> bool {
        matches!(self, TrackSize::Px(_) | TrackSize::Pct(_))
    }
}

/// `minmax(min, max)`. A plain size uses the same value for both.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Track {
    pub min: TrackSize,
    pub max: TrackSize,
}

impl Track {
    /// Sized by its content.
    pub const AUTO: Track = Track { min: TrackSize::Auto, max: TrackSize::Auto };

    pub const fn px(v: f32) -> Track {
        Track { min: TrackSize::Px(v), max: TrackSize::Px(v) }
    }

    pub const fn pct(v: f32) -> Track {
        Track { min: TrackSize::Pct(v), max: TrackSize::Pct(v) }
    }

    /// A share of the leftover space, with a minimum of zero.
    pub const fn fr(v: f32) -> Track {
        Track { min: TrackSize::Auto, max: TrackSize::Fr(v) }
    }

    pub const fn min_content() -> Track {
        Track { min: TrackSize::MinContent, max: TrackSize::MinContent }
    }

    pub const fn max_content() -> Track {
        Track { min: TrackSize::MaxContent, max: TrackSize::MaxContent }
    }

    pub const fn minmax(min: TrackSize, max: TrackSize) -> Track {
        Track { min, max }
    }

    /// `n` tracks of the same size.
    pub fn repeat(n: usize, track: Track) -> Vec<Track> {
        vec![track; n]
    }
}

/// The grid part of a style: the container's tracks, and an item's placement.
#[derive(Clone, Debug, PartialEq)]
pub struct GridStyle {
    pub columns: Vec<Track>,
    pub rows: Vec<Track>,
    /// The size of rows and columns added for items placed beyond the defined tracks.
    pub auto_rows: Track,
    pub auto_cols: Track,
    /// Auto-placement fills columns first.
    pub column_flow: bool,
    /// Auto-placement goes back to fill holes.
    pub dense: bool,
    pub justify_items: Align,
    pub justify_content: Justify,
    pub align_content: Justify,
    /// Named areas, one string per row: `["header header", "nav main"]`; `.` is an empty cell.
    pub areas: Vec<String>,
    /// Above zero: as many equal columns of at least this width as fit (responsive cards).
    pub fill_min: f32,

    /// An item's first column line, counted from 1; 0 places it automatically.
    pub col_start: usize,
    pub col_span: usize,
    pub row_start: usize,
    pub row_span: usize,
    /// The named area an item takes.
    pub area: String,
    /// `None`: the container's `justify_items`.
    pub justify_self: Option<Align>,
}

impl Default for GridStyle {
    fn default() -> GridStyle {
        GridStyle {
            columns: Vec::new(),
            rows: Vec::new(),
            auto_rows: Track::AUTO,
            auto_cols: Track::AUTO,
            column_flow: false,
            dense: false,
            justify_items: Align::Stretch,
            justify_content: Justify::Start,
            align_content: Justify::Start,
            areas: Vec::new(),
            fill_min: 0.0,
            col_start: 0,
            col_span: 1,
            row_start: 0,
            row_span: 1,
            area: String::new(),
            justify_self: None,
        }
    }
}

impl GridStyle {
    /// Whether what a grid container reads from its item is equal.
    pub(crate) fn same_as_item(a: Option<&GridStyle>, b: Option<&GridStyle>) -> bool {
        let item = |g: Option<&GridStyle>| {
            g.map(|g| (g.col_start, g.col_span, g.row_start, g.row_span, g.area.clone(), g.justify_self))
        };
        let none = Some((0, 1, 0, 1, String::new(), None));
        item(a).or_else(|| none.clone()) == item(b).or(none)
    }
}

/// A named area as rows and columns counted from zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Area {
    pub row: usize,
    pub col: usize,
    pub rows: usize,
    pub cols: usize,
}

/// Turns area rows into named rectangles.
pub(crate) fn parse_areas(rows: &[String]) -> HashMap<&str, Area> {
    let mut out: HashMap<&str, Area> = HashMap::new();
    for (r, line) in rows.iter().enumerate() {
        for (c, name) in line.split_whitespace().enumerate().filter(|(_, name)| *name != ".") {
            out.entry(name)
                .and_modify(|a| {
                    if a.row == r {
                        a.cols = c - a.col + 1;
                    }
                    a.rows = r - a.row + 1;
                })
                .or_insert(Area { row: r, col: c, rows: 1, cols: 1 });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn areas_become_rectangles() {
        let rows = ["header header".to_string(), "nav main".to_string(), "nav .".to_string()];
        let areas = parse_areas(&rows);
        assert_eq!(areas["header"], Area { row: 0, col: 0, rows: 1, cols: 2 });
        assert_eq!(areas["nav"], Area { row: 1, col: 0, rows: 2, cols: 1 });
        assert_eq!(areas["main"], Area { row: 1, col: 1, rows: 1, cols: 1 });
        assert!(!areas.contains_key("."));
    }

    #[test]
    fn only_item_values_reach_the_container() {
        let a = GridStyle { columns: Track::repeat(3, Track::fr(1.0)), ..GridStyle::default() };
        assert!(GridStyle::same_as_item(Some(&a), None));
        let b = GridStyle { col_span: 2, ..GridStyle::default() };
        assert!(!GridStyle::same_as_item(Some(&b), None));
    }
}
