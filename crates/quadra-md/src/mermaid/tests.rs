//! Diagrams: what a source is read as, the faults it tells, and where a layout puts things.

use quadra_prim::{Point, Rect};
use quadra_text::{FakeBackend, TextStyle, TextSystem};

use super::draw::{Drawing, Head, Ink, Measure, Op, decode};
use super::flow::{Dir, Flow, Shape, Stroke, place};
use super::{Chart, read};

fn flow(source: &str) -> Flow {
    match read(source) {
        Ok(Chart::Flow(flow)) => flow,
        Err(fault) => panic!("{fault}"),
    }
}

fn fault(source: &str) -> String {
    read(source).err().expect("a fault")
}

fn node(f: &Flow, id: &str) -> usize {
    f.nodes.iter().position(|node| node.id == id).expect(id)
}

#[test]
fn a_node_has_the_shape_of_what_is_about_its_text() {
    let f = flow(
        "graph TD\na[Rect]\nb(Round)\nc([Stadium])\nd((Circle))\ne{Diamond}\nf{{Hex}}\ng[[Sub]]\nh[(DB)]\ni>Flag]\nj[\"Quoted [x]\"]\nk",
    );
    let shapes: Vec<Shape> = f.nodes.iter().map(|node| node.shape).collect();
    use Shape::*;
    assert_eq!(
        shapes,
        [Rect, Round, Stadium, Circle, Diamond, Hexagon, Subroutine, Cylinder, Flag, Rect, Rect]
    );
    assert_eq!(
        (f.nodes[0].label.as_str(), f.nodes[9].label.as_str(), f.nodes[10].label.as_str()),
        ("Rect", "Quoted [x]", "k")
    );
    // The slanted boxes are boxes, without their slashes.
    let slanted = flow("graph TD\na[/In/]\nb[\\Out\\]\nc[/é/]");
    assert_eq!(slanted.nodes.iter().map(|node| node.label.as_str()).collect::<Vec<_>>(), ["In", "Out", "é"]);
    // A label is read once: what a name gives is not read again.
    assert_eq!(decode(" \"a<br/>b &amp;lt; c\" "), "a\nb &lt; c");
}

#[test]
fn a_link_has_a_stroke_heads_a_label_and_a_length() {
    let f = flow(
        "flowchart LR\nA --> B\nB --- C\nC -.-> D\nD ==> E\nE --o F\nF --x G\nG <--> H\nH --->|long| I\nI -- text --> J\nJ -. dots .-> K",
    );
    assert_eq!(f.dir(), Dir::Right);
    let e = &f.edges;
    assert_eq!(e.len(), 10);
    assert_eq!((e[0].head, e[0].stroke, e[1].head), (Head::Arrow, Stroke::Solid, Head::None));
    assert_eq!((e[2].stroke, e[2].head, e[3].stroke), (Stroke::Dotted, Head::Arrow, Stroke::Thick));
    assert_eq!((e[4].head, e[5].head), (Head::Circle, Head::Cross));
    assert_eq!((e[6].head, e[6].tail), (Head::Arrow, Head::Arrow));
    assert_eq!((e[7].label.as_str(), e[7].span), ("long", 2));
    assert_eq!((e[8].label.as_str(), e[8].head, e[8].span), ("text", Head::Arrow, 1));
    assert_eq!((e[9].label.as_str(), e[9].stroke), ("dots", Stroke::Dotted));
}

#[test]
fn chains_groups_and_lines_that_style() {
    let f = flow(
        "graph TD\n%% a comment\nA --> B --> C; C --> D\na1 & a2 --> b1 & b2\nclassDef x fill:#f00\nstyle A fill:#fff\nA:::x --> E",
    );
    assert_eq!(f.edges.len(), 3 + 4 + 1);
    assert_eq!((f.edges[1].from, f.edges[1].to), (node(&f, "B"), node(&f, "C")));
    let (a2, b2) = (node(&f, "a2"), node(&f, "b2"));
    assert_eq!(f.edges.iter().filter(|e| e.from == a2 && e.to == b2).count(), 1);
}

#[test]
fn subgraphs_hold_their_nodes_and_each_other() {
    let f = flow(
        "graph TD\nsubgraph one [First group]\nA --> B\nsubgraph inner\nC\nend\nend\nsubgraph Two words\nD\nend\nB --> D",
    );
    let titles: Vec<&str> = f.subs.iter().map(|sub| sub.title.as_str()).collect();
    assert_eq!(titles, ["First group", "inner", "Two words"]);
    assert_eq!(f.subs[1].parent, Some(0));
    let sub = |id: &str| f.nodes[node(&f, id)].sub;
    assert_eq!((sub("A"), sub("C"), sub("D")), (Some(0), Some(1), Some(2)));
}

#[test]
fn faults_say_their_line_and_other_kinds_say_they_are_not_drawn() {
    assert!(fault("graph TD\nA -->").starts_with("line 2"));
    assert!(fault("%% hi\n\ngraph TD\nA --> B\nB -- never closed").starts_with("line 5"));
    assert!(fault("pie title Pets\n\"Dogs\" : 3").contains("`pie` is not supported"));
    assert_eq!(
        (fault(""), fault("---\ntitle: x\n---\n"), fault(";")),
        ("empty diagram".into(), "empty diagram".into(), "no diagram kind".into())
    );
    // What a diagram may hold has an end, and the fault says which.
    let many: String = (0..1001).map(|i| format!("n{i}\n")).collect();
    assert_eq!(fault(&format!("graph TD\n{many}")), "line 1002: more than 1000 nodes");
    let group = |name: &str| (0..30).map(|i| format!("{name}{i}")).collect::<Vec<_>>().join(" & ");
    assert_eq!(fault(&format!("graph TD\n{} --> {}", group("a"), group("b"))), "line 2: more than 500 links");
    assert_eq!(fault(&format!("graph TD\n{}", "a --> b\n".repeat(600))), "line 502: more than 500 links");
    // A link may ask for many ranks; all of them together, not for more than can be laid out.
    assert_eq!(
        fault(&format!("graph TD\n{}", "a ------------------------------------> b\n".repeat(400))),
        "the diagram is too large to lay out"
    );
}

fn measure(ts: &mut TextSystem) -> Measure<'_> {
    Measure { ts, base: TextStyle { size: 14.0, ..TextStyle::default() } }
}

fn laid(f: &Flow) -> Drawing {
    place::layout(f, &mut measure(&mut TextSystem::new(Box::new(FakeBackend::new()))))
}

/// The boxes of the nodes of a flowchart laid out: each is one shape, in the order of the
/// nodes.
fn node_boxes(d: &Drawing) -> Vec<Rect> {
    let bounds = |points: &Vec<Point>| {
        let (xs, ys) = (points.iter().map(|p| p.x), points.iter().map(|p| p.y));
        let (left, top) = (xs.clone().fold(f32::MAX, f32::min), ys.clone().fold(f32::MAX, f32::min));
        Rect::new(left, top, xs.fold(f32::MIN, f32::max) - left, ys.fold(f32::MIN, f32::max) - top)
    };
    let shape = |op: &Op| match op {
        Op::Box { rect, fill: Ink::Panel, .. } => Some(*rect),
        Op::Fill { points, .. } => Some(bounds(points)),
        _ => None,
    };
    d.ops.iter().filter_map(shape).collect()
}

const SAMPLE: &str = "graph TD
A[Start] --> B{Is it?}
B -->|Yes| C[OK]
C --> D[Rethink]
D --> B
B ---->|No| E[End]
subgraph work [Work]
C
D
end
F((Hub)) --> A & C & E";

#[test]
fn no_two_nodes_overlap_and_links_go_the_way_of_the_chart() {
    let f = flow(SAMPLE);
    let d = laid(&f);
    let boxes = node_boxes(&d);
    assert_eq!(boxes.len(), f.nodes.len());
    for (i, a) in boxes.iter().enumerate() {
        assert!(boxes[i + 1..].iter().all(|b| !a.intersects(*b)), "node {i} is over another");
    }
    // Every link goes down but the one that closes the cycle, D --> B.
    for e in f.edges.iter().filter(|e| e.from != node(&f, "D")) {
        assert!(boxes[e.to].y > boxes[e.from].bottom(), "{} -> {}", f.nodes[e.from].id, f.nodes[e.to].id);
    }
    // The frame of the subgraph is about its nodes, and no other node is in it.
    let frame = d.ops.iter().find_map(|op| match op {
        Op::Box { rect, fill: Ink::Frame, .. } => Some(*rect),
        _ => None,
    });
    let (frame, inside) = (frame.expect("a frame"), [node(&f, "C"), node(&f, "D")]);
    for (v, b) in boxes.iter().enumerate() {
        assert_eq!(frame.intersects(*b), inside.contains(&v), "node {}", f.nodes[v].id);
    }
    // The drawing starts a margin from its corner and holds everything.
    let all = boxes.iter().fold(frame, |all, b| all.union(*b));
    assert!(all.x >= 8.0 && all.y >= 8.0 && all.right() <= d.width - 8.0 && all.bottom() <= d.height - 8.0);
}

#[test]
fn links_end_on_the_outlines_of_nodes() {
    let f = flow("graph TD\nA[One] --> B[Two]\nA --> C((Three))\nC --> D{Four}");
    let d = laid(&f);
    let boxes = node_boxes(&d);
    let tips: Vec<Point> =
        d.ops.iter().filter_map(|op| if let Op::Head { tip, .. } = op { Some(*tip) } else { None }).collect();
    assert_eq!(tips.len(), 3);
    for (tip, e) in tips.iter().zip(&f.edges) {
        let b = boxes[e.to];
        // On the outline at the top: within a pixel and a half of the box, not deep in it.
        assert!(tip.x >= b.x - 1.5 && tip.x <= b.right() + 1.5, "{tip:?} {b:?}");
        assert!(tip.y >= b.y - 1.5 && tip.y <= b.y + 6.0, "{tip:?} {b:?}");
    }
}

#[test]
fn a_chart_goes_the_way_its_header_says() {
    let b = node_boxes(&laid(&flow("graph LR\nA --> B --> C")));
    assert!(b[1].x > b[0].right() && b[2].x > b[1].right());
    assert!((b[0].y - b[1].y).abs() < 1.0, "a straight row");
    let b = node_boxes(&laid(&flow("graph RL\nA --> B")));
    assert!(b[1].right() < b[0].x);
    let b = node_boxes(&laid(&flow("graph BT\nA --> B")));
    assert!(b[1].bottom() < b[0].y);
    // A node linked to itself, and a chart of nothing, are laid out too.
    assert_eq!(node_boxes(&laid(&flow("graph TD\nA --> A"))).len(), 1);
    assert_eq!(laid(&flow("graph TD")).ops.len(), 0);
}
