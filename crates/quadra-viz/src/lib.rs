//! Quadra's charts (specs/13, "Charts"): the plot, a line drawn a device pixel column at a
//! time with the coverage of each pixel worked out by the shader, and what is built on it:
//! the sparkline, the line chart, and the bar chart, which is views alone.

mod bar_chart;
mod geometry;
mod line_chart;
mod plot;

use quadra_tree::Token;

pub use bar_chart::{Bar, BarChart, bar_chart};
pub use geometry::{Span, hh_mm, line_spans, max_of, nice_max, short, value_at};
pub use line_chart::{LineChart, Series, line_chart};
pub use plot::{Plot, Sparkline, sparkline};

/// The colour of the series at `index`: the theme has eight, and a ninth is the first again.
pub fn series_color(index: usize) -> Token {
    quadra_kit::token::SERIES[index % quadra_kit::token::SERIES.len()]
}
