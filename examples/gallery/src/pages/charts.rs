//! Charts: the sparkline, the line chart and the bar chart.

use quadra::{Align, Div, div, row, text};
use quadra_viz::{Bar, Series, bar_chart, line_chart, series_color, sparkline};

use crate::common::{demo, framed, mono, page, stack};

/// Samples of something that goes up and down around `base`.
fn wave(n: usize, phase: f64, base: f64, amp: f64) -> Vec<f64> {
    let at = |i: usize| (i as f64 / 7.0 + phase).sin() + 0.3 * (i as f64 / 2.3 + phase * 2.0).sin();
    (0..n).map(|i| base + amp * at(i)).collect()
}

/// When the first sample of the charts was taken: noon of a day.
const START: i64 = 1_793_966_400_000;

pub fn sparkline_page() -> Div {
    let rows = ["api", "worker", "web", "postgres"].into_iter().enumerate().map(|(i, name)| {
        let values = wave(60, i as f64, 120.0, 60.0);
        let last = format!("{}m", values[59] as i64);
        let trend =
            sparkline(values).size(160.0, 24.0).color(series_color(i)).label(format!("CPU of {name}"));
        let name = div().width(80.0).child(text(name));
        row().align_items(Align::Center).gap(16.0).child(name).child(trend).child(mono(last))
    });
    let about = "An inline trend: a 1.5 px line over an area at 12 %. Lines are drawn a pixel column at a \
                 time by the shader, so live data never fills the atlas.";
    page(
        "Sparkline",
        about,
        [demo("CPU per app", stack().children(rows), "sparkline(values).size(160.0, 24.0)")],
    )
}

pub fn line_chart_page() -> Div {
    let one = line_chart([Series::new("cpu", wave(120, 0.0, 140.0, 50.0))]);
    let pods = (0..5)
        .map(|i| Series::new(format!("pod-{i}"), wave(120, i as f64 * 1.3, 60.0 + i as f64 * 40.0, 25.0)));
    let one = one.time(START, 1000).unit("m").height(180.0);
    let many = line_chart(pods).time(START, 1000).unit("m").height(220.0);
    let twelve = (0..12)
        .map(|i| Series::new(format!("node-{i}"), wave(120, i as f64 * 0.7, 40.0 + i as f64 * 15.0, 12.0)));
    let twelve = line_chart(twelve).time(START, 1000).height(220.0).legend();
    let about = "Up to 8 series on a time axis, a y axis rounded up to 1, 2, 2.5 or 5 times a power of \
                 ten, and a crosshair with the values under the mouse.";
    page(
        "Line chart",
        about,
        [
            demo(
                "One series (with area)",
                framed(div().padding(12.0).child(one)),
                "line_chart([Series::new(\"cpu\", values)]).time(start_ms, 1000)",
            ),
            demo(
                "Five series",
                framed(div().padding(12.0).child(many)),
                "the colours of series are the theme's",
            ),
            demo(
                "Twelve series, with a legend",
                framed(div().padding(12.0).child(twelve)),
                "line_chart(series).legend(): past eight, the colours again, fainter",
            ),
        ],
    )
}

pub fn bar_chart_page() -> Div {
    let apps = ["api", "worker", "web", "redis", "postgres", "prometheus"];
    let bars: Vec<Bar> =
        apps.into_iter().map(|app| Bar::new(app, 40.0 + (app.len() * 37 % 160) as f64)).collect();
    page(
        "Bar chart",
        "Labeled horizontal bars with their values, scaled to a rounded maximum.",
        [
            demo(
                "CPU by app",
                framed(div().padding(12.0).child(bar_chart(bars.clone()).unit("m"))),
                "bar_chart(vec![Bar::new(\"api\", 120.0)]).unit(\"m\")",
            ),
            demo(
                "Upright",
                framed(div().padding(12.0).child(bar_chart(bars).unit("m").upright(180.0))),
                "bar_chart(bars).upright(180.0)",
            ),
        ],
    )
}
