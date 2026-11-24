//! Charts: the sparkline, the line chart, the bar chart and the graph.

use quadra::{Align, Div, Point, Size, div, row, signal, text};
use quadra_design::{Status, Tone, plain_badge, status_mark, token};
use quadra_viz::{
    Bar, Graph, Hit, Link, LinkAxis, LinkStyle, Node, Series, bar_chart, graph, line_chart, series_color,
    sparkline,
};

use crate::common::{demo, framed, hint, mono, page, stack};

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
        row().align_items(Align::Center).gap(token::SPACE_XL).child(name).child(trend).child(mono(last))
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
    let one = one.time(START, 1000).limit("limit", 250.0).unit("m").height(180.0);
    let many = line_chart(pods).time(START, 1000).unit("m").height(220.0);
    let twelve = (0..12)
        .map(|i| Series::new(format!("node-{i}"), wave(120, i as f64 * 0.7, 40.0 + i as f64 * 15.0, 12.0)));
    let twelve = line_chart(twelve).time(START, 1000).height(220.0).legend();
    let bare = line_chart([Series::new("used", wave(120, 0.4, 120.0, 40.0))]).time(START, 1000);
    let bare = bare.limit("limit", 250.0).unit("m").bare().height(70.0);
    let bare = bare.legend_note(|| "since 09:12, when this view was opened".to_string());
    let health = [
        Series::new("CPU", wave(120, 0.2, 55.0, 15.0)),
        Series::new("Memory", wave(120, 1.1, 70.0, 8.0)).dashed(),
    ];
    let health = line_chart(health).time(START, 1000).unit("%").top(100.0).height(180.0).legend();
    let about = "Up to 8 series on a time axis, a y axis rounded up to 1, 2, 2.5 or 5 times a power of \
                 ten, and a crosshair with the values under the mouse.";
    page(
        "Line chart",
        about,
        [
            demo(
                "One series (with area) and its limit",
                framed(div().padding(token::SPACE_LG).child(one)),
                "line_chart([Series::new(\"cpu\", values)]).time(start_ms, 1000).limit(\"limit\", 250.0)",
            ),
            demo(
                "Bare, beside its numbers",
                framed(div().padding(token::SPACE_LG).width(520.0).child(bare)),
                "line_chart(series).limit(\"limit\", 250.0).bare().legend_note(|| since)",
            ),
            demo(
                "Of what can be allocated",
                framed(div().padding(token::SPACE_LG).child(health)),
                "Series::new(\"Memory\", values).dashed(), .unit(\"%\").top(100.0): a sign on the scale",
            ),
            demo(
                "Five series",
                framed(div().padding(token::SPACE_LG).child(many)),
                "the colours of series are the theme's",
            ),
            demo(
                "Twelve series, with a legend",
                framed(div().padding(token::SPACE_LG).child(twelve)),
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
                framed(div().padding(token::SPACE_LG).child(bar_chart(bars.clone()).unit("m"))),
                "bar_chart(vec![Bar::new(\"api\", 120.0)]).unit(\"m\")",
            ),
            demo(
                "Upright",
                framed(div().padding(token::SPACE_LG).child(bar_chart(bars).unit("m").upright(180.0))),
                "bar_chart(bars).upright(180.0)",
            ),
        ],
    )
}

/// A topology: an ingress, its services, their workloads; the link a policy denies dashed in
/// the error colour, the chain the pointer is on in the accent.
fn topology(lit: quadra::Signal<Option<Hit>>) -> Graph {
    let node = |id: &'static str, kind: &'static str, status: Status, (x, y): (f32, f32)| {
        let face = move || {
            let line =
                row().align_items(Align::Center).gap(token::SPACE_SM).height(40.0).padding_x(token::SPACE_LG);
            let line =
                line.bg(token::PANEL).radius(token::RADIUS_MD).border(1.0).border_color(token::LINE_STRONG);
            line.child(status_mark(status)).child(plain_badge(kind, Tone::Primary)).child(text(id))
        };
        // The export draws what the face shows: the box, the kind, the name.
        let svg = move |r: quadra::Rect| {
            let (x, y, mid) = (r.x, r.y, r.y + r.height / 2.0);
            let frame = format!(
                r##"<rect x="{x}" y="{y}" width="{}" height="{}" rx="6" fill="#21252b" stroke="#464b57"/>"##,
                r.width, r.height
            );
            let kind = format!(
                r##"<text x="{}" y="{mid}" font-size="10" fill="#74ade8" dominant-baseline="middle">{kind}</text>"##,
                x + 24.0
            );
            let name = format!(
                r##"<text x="{}" y="{mid}" font-size="14" fill="#dce0e5" dominant-baseline="middle">{id}</text>"##,
                x + 80.0
            );
            frame + &kind + &name
        };
        Node::new(id, Point::new(x, y), Size::new(190.0, 40.0), id, face).svg(svg)
    };
    let nodes = [
        node("shop", "Ingress", Status::Running, (0.0, 80.0)),
        node("api", "Service", Status::Running, (260.0, 20.0)),
        node("web", "Service", Status::Running, (260.0, 140.0)),
        node("api-7c9f", "Pod", Status::Running, (520.0, 0.0)),
        node("api-x2kq", "Pod", Status::Failed, (520.0, 60.0)),
        node("web-5d8b", "Pod", Status::Pending, (520.0, 160.0)),
    ];
    let ends = [
        ("shop", "api"),
        ("shop", "web"),
        ("api", "api-7c9f"),
        ("api", "api-x2kq"),
        ("web", "web-5d8b"),
        ("shop", "web-5d8b"),
    ];
    // The last skips the column of the services: it passes under web.
    let under =
        |k: usize| if k == 5 { vec![Point::new(260.0, 215.0), Point::new(450.0, 215.0)] } else { Vec::new() };
    let links = (0..ends.len()).map(|k| Link::new(ends[k].0, ends[k].1).through(under(k)));
    let style = move |k: usize| {
        let on = match lit.get() {
            Some(Hit::Link(at)) => at == k,
            Some(Hit::Node(id)) => ends[k].0 == &*id || ends[k].1 == &*id,
            None => false,
        };
        match (k, on) {
            (_, true) => LinkStyle { color: token::ACCENT.into(), width: 2.0, ..LinkStyle::default() },
            (3, false) => LinkStyle { color: token::ERROR.into(), dash: 4.0, ..LinkStyle::default() },
            _ => LinkStyle::default(),
        }
    };
    let g = graph(nodes, links).link_style(style).group("payments", ["api", "api-7c9f", "api-x2kq"]);
    g.height(320.0).fit()
}

/// A storage map: claims, their volumes, the classes: columns 40 px apart, links side to side.
fn storage() -> Graph {
    let box_of = |id: &'static str, (x, y): (f32, f32)| {
        let face = move || {
            let line =
                row().align_items(Align::Center).height(28.0).padding_x(token::SPACE_MD).bg(token::PANEL);
            line.radius(token::RADIUS_SM).border(1.0).border_color(token::LINE_STRONG).child(mono(id))
        };
        Node::new(id, Point::new(x, y), Size::new(120.0, 28.0), id, face)
    };
    let nodes = [
        box_of("data-pg-0", (0.0, 0.0)),
        box_of("data-pg-1", (0.0, 40.0)),
        box_of("cache", (0.0, 120.0)),
        box_of("pv-3f1c", (160.0, 0.0)),
        box_of("pv-8e77", (160.0, 40.0)),
        box_of("pv-c2ab", (160.0, 120.0)),
        box_of("ssd", (320.0, 20.0)),
        box_of("standard", (320.0, 120.0)),
    ];
    let ends = [
        ("data-pg-0", "pv-3f1c"),
        ("data-pg-1", "pv-8e77"),
        ("cache", "pv-c2ab"),
        ("pv-3f1c", "ssd"),
        ("pv-8e77", "ssd"),
        ("pv-c2ab", "standard"),
    ];
    let links = ends.map(|(a, b)| Link::new(a, b));
    graph(nodes, links).axis(LinkAxis::Across).straight().height(200.0).fit()
}

pub fn graph_page() -> Div {
    let lit = signal(None);
    let svg = topology(lit).to_svg(&quadra_design::dark().theme()).len();
    let shown = topology(lit).on_hover(move |hit| lit.set(hit));
    let told = move || match lit.get() {
        Some(Hit::Node(id)) => format!("On {id}"),
        Some(Hit::Link(k)) => format!("On link {k}"),
        None => {
            format!("Drag to pan, Ctrl and the wheel to zoom, a double click fits. The SVG: {svg} bytes.")
        }
    };
    page(
        "Graph",
        "Nodes that are views at points, links between them, frames around groups; pan, zoom, fit and SVG.",
        [
            demo(
                "A topology",
                stack().child(framed(shown)).child(hint(told)),
                "graph(nodes, links).link_style(|i| ..).group(\"payments\", ids).fit().to_svg(&theme)",
            ),
            demo(
                "Columns near each other",
                framed(storage()),
                "graph(nodes, links).axis(LinkAxis::Across).straight()",
            ),
        ],
    )
}
