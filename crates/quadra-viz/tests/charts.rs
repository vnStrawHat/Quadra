//! The charts (specs/13, "Charts"): the sparkline as instances, the line chart and the bar
//! chart as what they say and where they put it.

use std::rc::Rc;

use quadra_design::dark;
use quadra_reactive::signal;
use quadra_render::{Instance, Kind};
use quadra_testing::Harness;
use quadra_tree::{NodeId, PointerInput, Role};
use quadra_view::div;
use quadra_viz::{Bar, Series, bar_chart, line_chart, sparkline};

/// Noon of a day, in milliseconds since 1970.
const NOON: i64 = 1_793_966_400_000;

fn harness() -> Harness {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h
}

/// What the window draws, of one kind.
fn drawn(h: &Harness, kind: Kind) -> Vec<Instance> {
    let scene = h.tree.scene();
    let all = scene.roots().iter().flat_map(|&root| scene.layer(root).instances());
    all.filter(|instance| instance.kind == kind as u32).copied().collect()
}

#[test]
fn a_sparkline_is_a_line_a_pixel_column_and_new_values_only_paint_it() {
    let values = signal(vec![10.0, 80.0, 30.0, 95.0, 20.0]);
    let mut h = harness();
    h.scale = 1.5;
    h.mount(move || div().padding(7.0).child(sparkline(values).size(100.0, 60.0).label("CPU")));
    h.frame();
    // 100 px at 150 %: 150 columns, each a quad of the area and a piece of the line.
    let (lines, areas) = (drawn(&h, Kind::Line), drawn(&h, Kind::Quad));
    assert_eq!((lines.len(), areas.len()), (150, 150));
    // A column is one device pixel wide, on the pixel grid, beside the one before it...
    let left = (7.0f32 * 1.5).round();
    for (x, line) in lines.iter().enumerate() {
        assert_eq!((line.bounds[0], line.bounds[2]), (left + x as f32, 1.0));
    }
    // ...and where the line is in it is not rounded to pixels: the shader works out how
    // much of each pixel it covers from the height and the fall it is told, which is what
    // keeps a slope from being steps.
    assert!(lines.iter().filter(|line| line.bounds[1].fract() != 0.0).count() > 100);
    assert!(lines.iter().all(|line| line.params[1] == 2.0 && line.params[0] >= 1.5 * 1.5));
    assert!(lines.iter().any(|line| line.params[2] > 1.0), "a steep column says how far it falls");
    h.get_by_role(Role::Image, "CPU");

    // New values: the plot is painted again over its own instances, and nothing is laid out.
    values.set(vec![50.0, 50.0, 90.0]);
    let (stats, _) = h.frame();
    assert_eq!((stats.nodes_painted, stats.nodes_laid_out, stats.layers_patched), (1, 0, 1));
    assert_eq!(drawn(&h, Kind::Line).len(), 150);
    // The same values again are no work at all.
    values.set(vec![50.0, 50.0, 90.0]);
    assert!(h.frame_if_needed().is_none());
}

/// Where the text of this name is in the window.
fn text_at(h: &mut Harness, name: &str) -> quadra_prim::Rect {
    let node = h.get_by_text(name).node();
    h.tree.window_rect(node)
}

#[test]
fn a_line_chart_says_its_scale_and_the_values_under_the_pointer() {
    let second = signal(vec![10.0, 20.0, 30.0, 40.0]);
    let mut h = harness();
    h.mount(move || {
        let series = [Series::new("a", vec![0.0, 100.0, 180.0, 150.0]), Series::new("b", second)];
        // 452 px wide and 126 tall: the plot is 400 by 100, at (44, 6).
        div().width(452.0).child(line_chart(series).time(NOON, 60_000).unit("m").height(126.0))
    });
    h.frame();
    // The top is 180 rounded up, and a sample a minute.
    for said in ["200", "150", "100", "50", "0", "12:00", "12:01", "12:03"] {
        assert!(h.has_text(said), "{said} is not said:\n{}", h.semantics());
    }
    let plot = h.get_by_role(Role::Image, "a: 0 to 180m, last 150m; b: 10 to 40m, last 40m").node();
    assert_eq!(h.tree.window_rect(plot), quadra_prim::Rect::new(44.0, 6.0, 400.0, 100.0));
    assert!(!h.has_text("a 100m"));

    // The pointer near the second sample of four: its values, at the right of the hairline.
    h.pointer(PointerInput::Move, 44.0 + 140.0, 50.0);
    h.frame();
    assert!(h.has_text("a 100m") && h.has_text("b 20m"), "{}", h.semantics());
    let hair = 44.0 + 400.0 / 3.0;
    assert!(text_at(&mut h, "a 100m").x > hair + 12.0);
    // Still nearest to that sample: nothing to do.
    h.pointer(PointerInput::Move, 44.0 + 150.0, 52.0);
    assert!(h.frame_if_needed().is_none());
    // Near the last one, in the right half: the box is at the left of the hairline.
    h.pointer(PointerInput::Move, 44.0 + 390.0, 50.0);
    h.frame();
    assert!(h.has_text("b 40m") && !h.has_text("a 100m"));
    assert!(text_at(&mut h, "a 150m").right() < 44.0 + 400.0 - 12.0);
    h.pointer(PointerInput::Leave, 0.0, 0.0);
    h.frame();
    assert!(!h.has_text("a 150m"));

    // A series that changes moves the scale of all.
    second.set(vec![10.0, 900.0, 30.0, 40.0]);
    h.frame();
    assert!(h.has_text("1k") && h.has_text("750") && !h.has_text("200"));
}

/// The bar of the row with this label: the label's box, the row, its track, the bar.
fn bar_of(h: &mut Harness, label: &str) -> NodeId {
    let text = h.get_by_text(label).node();
    let row = h.tree.parent(h.tree.parent(text).unwrap()).unwrap();
    h.tree.children(h.tree.children(row)[1])[0]
}

#[test]
fn a_bar_is_as_long_as_its_share_of_the_rounded_maximum() {
    let bars = signal(vec![Bar::new("api", 150.0), Bar::new("web", 50.0)]);
    let mut h = harness();
    // 624 px wide: the track is 400.
    h.mount(move || div().width(624.0).child(bar_chart(bars).unit("m")));
    h.frame();
    let (api, web) = (bar_of(&mut h, "api"), bar_of(&mut h, "web"));
    // Under a top of 200.
    assert_eq!((h.tree.window_rect(api).width, h.tree.window_rect(web).width), (300.0, 100.0));
    assert_eq!(h.tree.window_rect(api).x, 160.0);
    assert!(h.has_text("150m") && h.has_text("50m"));

    // Other values: a row keeps its nodes by its label, and a bar is 2 px at least.
    bars.set(vec![Bar::new("api", 100.0), Bar::new("web", 50.0), Bar::new("db", 0.4)]);
    h.frame();
    assert_eq!(bar_of(&mut h, "api"), api);
    assert_eq!((h.tree.window_rect(api).width, h.tree.window_rect(web).width), (400.0, 200.0));
    let db = bar_of(&mut h, "db");
    assert_eq!(h.tree.window_rect(db).width, 2.0);
    assert!(h.has_text("0.4m") && !h.has_text("150m"));

    // A constant chart is the same rows, with nothing bound.
    let mut h = harness();
    h.mount(|| div().width(624.0).child(bar_chart(vec![Bar::new("api", 25.0), Bar::new("web", 50.0)])));
    h.frame();
    let api = bar_of(&mut h, "api");
    assert_eq!(h.tree.window_rect(api).width, 200.0);
}

#[test]
fn upright_bars_stand_on_one_line_and_a_legend_names_the_series() {
    let mut h = harness();
    let bars = vec![Bar::new("api", 150.0), Bar::new("web", 50.0)];
    h.mount(move || div().width(400.0).child(bar_chart(bars).upright(200.0)));
    h.frame();
    // The label's box, its column, the track, the bar.
    let bar = |h: &mut Harness, label: &str| {
        let text = h.get_by_text(label).node();
        let column = h.tree.parent(h.tree.parent(text).unwrap()).unwrap();
        h.tree.window_rect(h.tree.children(h.tree.children(column)[1])[0])
    };
    let (api, web) = (bar(&mut h, "api"), bar(&mut h, "web"));
    assert!((api.height - 3.0 * web.height).abs() < 0.01 && api.bottom() == web.bottom(), "{api:?} {web:?}");
    assert!(api.x < web.x && h.has_text("150"));

    let mut h = harness();
    h.mount(|| line_chart((0..9).map(|i| Series::new(format!("s{i}"), vec![1.0, 2.0]))).legend());
    h.frame();
    assert!(h.has_text("s0") && h.has_text("s8"));
    // The ninth colour is the first, fainter.
    let ninth = quadra_viz::series_color(8);
    assert_eq!(ninth, quadra_design::token::SERIES[0].alpha(0.6));
}

#[test]
fn a_limit_is_a_dashed_line_the_scale_reaches_and_live_times_follow_the_samples() {
    let (values, first, limit) = (signal(vec![100.0, 120.0, 110.0]), signal(NOON), signal(2000.0));
    let mut h = harness();
    h.mount(move || {
        let chart = line_chart([Series::new("cpu", values)]).time(first, 60_000).limit("limit", limit);
        // The plot is 400 by 100, at (44, 6).
        div().width(452.0).child(chart.unit("m").height(126.0))
    });
    h.frame();
    // The top reaches the limit, far over the samples; a reader hears it after the series.
    assert!(h.has_text("2k") && h.has_text("1k"), "{}", h.semantics());
    h.get_by_role(Role::Image, "cpu: 100 to 120m, last 110m; limit 2km");
    // The line is dashed, at the limit's height: the top of the plot under a top of 2000.
    let dashed =
        drawn(&h, Kind::Quad).into_iter().filter(|q| q.flags & quadra_render::FLAG_DASHED != 0).count();
    assert_eq!(dashed, 1);
    limit.set(1000.0);
    h.frame();
    assert!(h.has_text("1k") && !h.has_text("2k"));

    // Live: the oldest sample goes, a new one comes, and the times move with them.
    assert!(h.has_text("12:00") && h.has_text("12:02"));
    values.set(vec![120.0, 110.0, 130.0]);
    first.set(NOON + 60_000);
    h.frame();
    assert!(h.has_text("12:01") && h.has_text("12:03") && !h.has_text("12:00"), "{}", h.semantics());
    // The box under the pointer says the time of the sample there, and the limit.
    h.pointer(PointerInput::Move, 44.0 + 395.0, 50.0);
    h.frame();
    assert!(h.has_text("12:03") && h.has_text("cpu 130m") && h.has_text("limit 1km"), "{}", h.semantics());
}

#[test]
fn a_bare_chart_fills_its_box_and_its_legend_names_the_limit_beside_a_note() {
    let mut h = harness();
    h.mount(|| {
        let used = Series::new("used", vec![20.0, 35.0, 30.0]);
        let chart = line_chart([used]).time(NOON, 15_000).offset(7 * 3600).limit("limit", 80.0).top(100.0);
        let chart = chart.bare().height(70.0).legend_note(|| "since 19:00".to_string());
        div().width(300.0).child(chart)
    });
    h.frame();
    // The plot is all of the box: no values at its left, no times under it.
    let plot = h.query(|n| n.semantics.role == Role::Image)[0];
    assert_eq!(h.tree.window_rect(plot), quadra_prim::Rect::new(0.0, 0.0, 300.0, 70.0));
    assert!(!h.has_text("100") && !h.has_text("19:00"));
    // The legend: the series, then the limit, then the note at the end of the row.
    assert!(h.has_text("used") && h.has_text("limit") && h.has_text("since 19:00"));
    assert!(text_at(&mut h, "limit").x > text_at(&mut h, "used").x);
    assert!((text_at(&mut h, "since 19:00").right() - 300.0).abs() < 0.5);
    // The time under the pointer is local: seven hours ahead of noon UTC.
    h.pointer(PointerInput::Move, 10.0, 30.0);
    h.frame();
    assert!(h.has_text("19:00"), "{}", h.semantics());
}

#[test]
fn a_fixed_top_is_the_top_of_the_scale() {
    let mut h = harness();
    h.mount(|| div().width(452.0).child(line_chart([Series::new("cpu", vec![10.0, 30.0])]).top(100.0)));
    h.frame();
    assert!(h.has_text("100") && h.has_text("75") && !h.has_text("40"));
}
