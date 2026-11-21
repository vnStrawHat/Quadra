//! Quadra's charts (specs/13, "Charts"): the plot, a line drawn a device pixel column at a
//! time with the coverage of each pixel worked out by the shader, and what is built on it:
//! the sparkline, the line chart, and the bar chart, which is views alone; and the graph of
//! nodes and links.

mod bar_chart;
mod geometry;
mod graph;
mod line_chart;
mod plot;

use quadra_tree::Paint;

pub use bar_chart::{Bar, BarChart, bar_chart};
pub use geometry::{Span, hh_mm, line_spans, max_of, nice_max, short, value_at};
pub use graph::{Graph, GraphView, Hit, Link, LinkAxis, LinkStyle, Node, fitted, graph};
pub use line_chart::{LineChart, Series, line_chart};
pub use plot::{Plot, Sparkline, sparkline};

/// The colour of the series at `index`: the theme has eight; the next eight are those at
/// 60 %, the eight after at 35 %, and so round again.
pub fn series_color(index: usize) -> Paint {
    let colors = &quadra_design::token::SERIES;
    let round = [1.0, 0.6, 0.35][index / colors.len() % 3];
    colors[index % colors.len()].alpha(round)
}
