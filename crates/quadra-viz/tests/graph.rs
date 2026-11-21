//! The graph (specs/13, "Charts"): nodes found where they show at any zoom, links told of by
//! a click, a drag that pans, the wheel with Ctrl that zooms about the pointer, a fit, links
//! repainted when their style changes, and the drawing as SVG.

use std::rc::Rc;

use quadra_design::{dark, token};
use quadra_platform::{Key, Mods};
use quadra_prim::{Point, Rect, Size};
use quadra_reactive::{Signal, signal};
use quadra_render::{Instance, Kind};
use quadra_testing::Harness;
use quadra_tree::{PointerInput, Role};
use quadra_view::{div, text};
use quadra_viz::{GraphView, Hit, Link, LinkAxis, LinkStyle, Node, graph};

struct Page {
    h: Harness,
    view: Signal<GraphView>,
    hits: Signal<Vec<Hit>>,
    lit: Signal<bool>,
}

fn node(id: &str, x: f32, y: f32) -> Node {
    let words = id.to_string();
    Node::new(id, Point::new(x, y), Size::new(100.0, 40.0), id, move || div().child(text(words.clone())))
}

fn page(fit: bool) -> Page {
    let (view, hits, lit) = (signal(GraphView::default()), signal(Vec::new()), signal(false));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        let nodes = [node("ingress", 0.0, 0.0), node("svc", 200.0, 0.0), node("pod", 200.0, 200.0)];
        let links = [Link::new("ingress", "svc"), Link::new("svc", "pod"), Link::new("svc", "gone")];
        let style = move |_| match lit.get() {
            true => LinkStyle { color: token::ACCENT.into(), ..LinkStyle::default() },
            false => LinkStyle::default(),
        };
        let g = graph(nodes, links).view(view).link_style(style).height(400.0);
        let g = g.on_click(move |hit| hits.update(|all| all.push(hit)));
        div().width(800.0).child(if fit { g.fit() } else { g })
    });
    h.frame();
    Page { h, view, hits, lit }
}

impl Page {
    fn settle(&mut self) {
        self.h.frame();
        while self.h.frame_if_needed().is_some() {}
    }

    fn rect(&mut self, name: &str) -> Rect {
        let node = self.h.get_by_role(Role::Button, name).node();
        self.h.tree.window_rect(node)
    }
}

/// The strokes drawn, in every layer.
fn lines(h: &Harness) -> Vec<Instance> {
    let layers = h.renderer.steps().iter().filter_map(|step| match step {
        quadra_render::Step::Layer { id, .. } => Some(h.tree.scene().layer(*id)),
        quadra_render::Step::Run { .. } => None,
    });
    let all = layers.flat_map(|layer| layer.instances().iter().copied());
    all.filter(|instance| instance.kind == Kind::Line as u32).collect()
}

#[test]
fn a_node_is_found_where_it_shows_at_any_zoom() {
    let mut p = page(false);
    assert_eq!(p.rect("svc"), Rect::new(200.0, 0.0, 100.0, 40.0));
    for (zoom, pan) in [(2.0, Point::new(10.0, 20.0)), (0.5, Point::new(100.0, 50.0))] {
        p.view.set(GraphView { pan, zoom });
        p.settle();
        let at = p.rect("svc");
        assert_eq!(at, Rect::new(200.0 * zoom + pan.x, pan.y, 100.0 * zoom, 40.0 * zoom));
        p.h.get_by_role(Role::Button, "svc").click();
        p.h.click(at.center().x, at.center().y);
        // Just past its edge, on nothing: no node is told of.
        p.h.click(at.right() + 3.0, at.y - 3.0);
    }
    let svc = Hit::Node("svc".into());
    assert_eq!(p.hits.get_untracked(), vec![svc.clone(), svc.clone(), svc.clone(), svc]);
}

#[test]
fn a_drag_pans_the_wheel_with_ctrl_zooms_about_the_pointer_and_a_link_is_clicked() {
    let mut p = page(false);
    // The link from svc down to pod runs at x = 250 from y = 40 to 200.
    p.h.click(250.0, 120.0);
    assert_eq!(p.hits.get_untracked(), vec![Hit::Link(1)]);
    p.h.pointer(PointerInput::Down, 500.0, 300.0);
    p.h.pointer(PointerInput::Move, 520.0, 330.0);
    p.h.pointer(PointerInput::Up, 520.0, 330.0);
    assert_eq!(p.view.get_untracked().pan, Point::new(20.0, 30.0));
    assert_eq!(p.hits.get_untracked().len(), 1, "a drag clicks nothing");
    // What is under the pointer stays there.
    let under = Point::new(270.0, 50.0);
    let before = p.view.get_untracked();
    let plane = Point::new((under.x - before.pan.x) / before.zoom, (under.y - before.pan.y) / before.zoom);
    p.h.tree.wheel(Point::new(0.0, 4.0), false, under, Mods::CTRL);
    let after = p.view.get_untracked();
    assert!(after.zoom > 1.0);
    let back = after.point(plane);
    assert!((back.x - under.x).abs() < 0.01 && (back.y - under.y).abs() < 0.01, "{back:?}");
    // Without Ctrl the wheel is not the graph's.
    p.h.wheel(270.0, 50.0, 4.0);
    assert_eq!(p.view.get_untracked(), after);
}

#[test]
fn a_fit_shows_all_of_it_and_a_style_that_changes_repaints_the_links() {
    let mut p = page(true);
    p.settle();
    // All of it in the box of 800 by 400, no larger than it is, in the middle.
    let v = p.view.get_untracked();
    assert_eq!(v.zoom, 1.0);
    let (first, last) = (p.rect("ingress"), p.rect("pod"));
    assert!(
        ((first.x + last.right()) / 2.0 - 400.0).abs() < 0.01
            && ((first.y + last.bottom()) / 2.0 - 200.0).abs() < 0.01
    );
    let accent = dark().theme().resolve(token::ACCENT.into()).to_array();
    assert!(!lines(&p.h).is_empty() && lines(&p.h).iter().all(|l| l.color != accent));
    p.lit.set(true);
    p.settle();
    assert!(lines(&p.h).iter().all(|l| l.color == accent));
}

#[test]
fn the_drawing_is_svg_with_its_frames_links_and_nodes() {
    let nodes =
        [node("a", 0.0, 0.0), Node::new("b", Point::new(200.0, 0.0), Size::new(100.0, 40.0), "<b & c>", div)];
    let dashed = |_| LinkStyle { dash: 4.0, ..LinkStyle::default() };
    let g = graph(nodes, [Link::new("a", "b")]).link_style(dashed).group("payments \"eu\"", ["a", "b"]);
    let svg = g.to_svg(&dark().theme());
    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\"") && svg.ends_with("</svg>"));
    for part in
        ["&lt;b &amp; c&gt;", "payments &quot;eu&quot;", "stroke-dasharray=\"4 4\"", "<path d=\"M100.0 20.0"]
    {
        assert!(svg.contains(part), "{part} is not in {svg}");
    }
    // A frame and two nodes; a link and its head.
    assert_eq!((svg.matches("<rect").count(), svg.matches("<path").count()), (3, 2));
}

#[test]
fn the_pointer_from_a_link_onto_a_node_is_on_the_node_and_a_drag_from_a_node_clicks_nothing() {
    let (told, hits, view) =
        (signal(Vec::<Option<Hit>>::new()), signal(Vec::new()), signal(GraphView::default()));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        let nodes = [node("svc", 200.0, 0.0), node("pod", 200.0, 200.0)];
        let g = graph(nodes, [Link::new("svc", "pod")]).view(view).height(400.0);
        let g = g.on_hover(move |hit| told.update(|all| all.push(hit)));
        div().width(800.0).child(g.on_click(move |hit| hits.update(|all| all.push(hit))))
    });
    h.frame();
    // Along the link at x = 250, then onto the node it ends at, then on in the node.
    for y in [120.0, 215.0, 225.0] {
        h.pointer(PointerInput::Move, 250.0, y);
    }
    assert_eq!(told.get_untracked(), vec![Some(Hit::Link(0)), Some(Hit::Node("pod".into()))]);
    // A drag that starts on a node pans, and clicks nothing.
    h.pointer(PointerInput::Down, 250.0, 20.0);
    h.pointer(PointerInput::Move, 280.0, 20.0);
    h.pointer(PointerInput::Up, 280.0, 20.0);
    assert_eq!((view.get_untracked().pan, hits.get_untracked()), (Point::new(30.0, 0.0), Vec::new()));
    // The press focused svc; Tab goes on to the next node, and Enter is its click.
    h.key(Key::Tab, Mods::NONE);
    h.key(Key::Enter, Mods::NONE);
    assert_eq!(hits.get_untracked(), vec![Hit::Node("pod".into())]);
}

#[test]
fn a_link_through_points_passes_them_past_the_column_it_skips() {
    let hits = signal(Vec::new());
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    // Three columns: the link from a to c skips b by a point below it.
    let around = Point::new(250.0, 100.0);
    h.mount(move || {
        let nodes = [node("a", 0.0, 0.0), node("b", 200.0, 0.0), node("c", 400.0, 0.0)];
        let g = graph(nodes, [Link::new("a", "c").through([around])]).height(400.0);
        div().width(800.0).child(g.on_click(move |hit| hits.update(|all| all.push(hit))))
    });
    h.frame();
    // Through the point, and nowhere across b.
    h.click(around.x, around.y);
    h.click(250.0, 20.0);
    assert_eq!(hits.get_untracked(), vec![Hit::Link(0), Hit::Node("b".into())]);
    let svg = graph([node("a", 0.0, 0.0), node("c", 400.0, 0.0)], [Link::new("a", "c").through([around])]);
    let svg = svg.to_svg(&dark().theme());
    assert!(svg.contains("L250.0 100.0") && svg.contains("<path d=\"M100.0 20.0"), "{svg}");
}

#[test]
fn links_run_side_to_side_and_straight_when_told() {
    let nodes = || [node("claim", 0.0, 0.0), node("volume", 140.0, 200.0)];
    // Further apart down than across: by itself the link leaves the bottom.
    let auto = graph(nodes(), [Link::new("claim", "volume")]).to_svg(&dark().theme());
    assert!(auto.contains("<path d=\"M50.0 40.0"), "{auto}");
    // Across, from its side to the other's; straight, one piece.
    let across = graph(nodes(), [Link::new("claim", "volume")]).axis(LinkAxis::Across).straight();
    let across = across.to_svg(&dark().theme());
    assert!(across.contains("<path d=\"M100.0 20.0 L140.0 220.0\""), "{across}");
}
