//! The graph (specs/13, "Charts"): nodes that are views at points of its own plane, links
//! between them as curves with heads, frames around groups; panned by a drag, zoomed by the
//! wheel with Ctrl, fitted to its box; and the drawing as SVG.

mod links;
mod pointer;
mod svg;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use quadra_design::{size, token};
use quadra_layout::{Align, Length, Overflow};
use quadra_platform::{Key, Mods};
use quadra_prim::{Point, Rect, Size};
use quadra_reactive::{Signal, effect, signal};
use quadra_tree::{NodeId, Paint, PointerKind, Role, Theme};
use quadra_view::{Div, MountCx, View, column, div, text};

use self::links::LinksView;

/// A view at a point of the graph's plane, as large as its size, named by its label.
pub struct Node {
    id: Rc<str>,
    at: Point,
    size: Size,
    label: String,
    face: Rc<dyn Fn() -> Div>,
}

impl Node {
    pub fn new(
        id: impl Into<Rc<str>>,
        at: Point,
        size: Size,
        label: impl Into<String>,
        face: impl Fn() -> Div + 'static,
    ) -> Node {
        Node { id: id.into(), at, size, label: label.into(), face: Rc::new(face) }
    }
}

/// A link from the node `from` to the node `to`, by their ids.
pub struct Link {
    from: Rc<str>,
    to: Rc<str>,
}

impl Link {
    pub fn new(from: impl Into<Rc<str>>, to: impl Into<Rc<str>>) -> Link {
        Link { from: from.into(), to: to.into() }
    }
}

/// How a link is drawn: its colour, its width, its dashes (the length of a dash and of a
/// gap; 0 is solid), and a head at its end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinkStyle {
    pub color: Paint,
    pub width: f32,
    pub dash: f32,
    pub arrow: bool,
}

impl Default for LinkStyle {
    fn default() -> Self {
        LinkStyle { color: token::TEXT_3.into(), width: 1.5, dash: 0.0, arrow: true }
    }
}

/// Where the graph's plane shows in its box: a point `p` of it is at `p * zoom + pan`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphView {
    pub pan: Point,
    pub zoom: f32,
}

impl Default for GraphView {
    fn default() -> Self {
        GraphView { pan: Point::ZERO, zoom: 1.0 }
    }
}

impl GraphView {
    pub fn point(self, p: Point) -> Point {
        Point::new(p.x * self.zoom + self.pan.x, p.y * self.zoom + self.pan.y)
    }

    pub fn rect(self, r: Rect) -> Rect {
        let at = self.point(r.origin());
        Rect::new(at.x, at.y, r.width * self.zoom, r.height * self.zoom)
    }
}

/// The view that shows all of `bounds` in a box of `size`: 16 px of margin, its middle in
/// the box's, never larger than it is.
pub fn fitted(bounds: Rect, size: Size) -> GraphView {
    let zoom = ((size.width - 32.0) / bounds.width).min((size.height - 32.0) / bounds.height);
    let zoom = zoom.clamp(0.25, 1.0);
    let middle = bounds.center();
    let pan = Point::new(size.width / 2.0 - middle.x * zoom, size.height / 2.0 - middle.y * zoom);
    GraphView { pan, zoom }
}

/// What the pointer is on: a node, by its id, or a link, by its place.
#[derive(Clone, Debug, PartialEq)]
pub enum Hit {
    Node(Rc<str>),
    Link(usize),
}

type Styles = Rc<dyn Fn(usize) -> LinkStyle>;

pub struct Graph {
    nodes: Vec<Node>,
    links: Vec<Link>,
    groups: Vec<(String, Vec<Rc<str>>)>,
    style: Styles,
    view: Option<Signal<GraphView>>,
    click: Option<Rc<dyn Fn(Hit)>>,
    hover: Option<Rc<dyn Fn(Option<Hit>)>>,
    height: f32,
    fit: bool,
}

/// A graph of `nodes` and the `links` between them, as wide as its box and 360 px tall.
pub fn graph(nodes: impl IntoIterator<Item = Node>, links: impl IntoIterator<Item = Link>) -> Graph {
    let (nodes, links) = (nodes.into_iter().collect(), links.into_iter().collect());
    let style: Styles = Rc::new(|_| LinkStyle::default());
    let (groups, view, click, hover) = (Vec::new(), None, None, None);
    Graph { nodes, links, groups, style, view, click, hover, height: 360.0, fit: false }
}

impl Graph {
    /// How each link is drawn, by its place: read again when what it reads changes.
    pub fn link_style(mut self, style: impl Fn(usize) -> LinkStyle + 'static) -> Self {
        self.style = Rc::new(style);
        self
    }

    /// A frame with a title around the nodes of these ids (a namespace).
    pub fn group<S: Into<Rc<str>>>(
        mut self,
        title: impl Into<String>,
        ids: impl IntoIterator<Item = S>,
    ) -> Self {
        self.groups.push((title.into(), ids.into_iter().map(Into::into).collect()));
        self
    }

    /// Where the plane shows, read and written: by a drag, the wheel, a fit, or the app.
    pub fn view(mut self, view: Signal<GraphView>) -> Self {
        self.view = Some(view);
        self
    }

    pub fn on_click(mut self, f: impl Fn(Hit) + 'static) -> Self {
        self.click = Some(Rc::new(f));
        self
    }

    /// What the pointer comes over, and `None` when it is over nothing.
    pub fn on_hover(mut self, f: impl Fn(Option<Hit>) + 'static) -> Self {
        self.hover = Some(Rc::new(f));
        self
    }

    pub fn height(mut self, px: f32) -> Self {
        self.height = px;
        self
    }

    /// All of it in view when its box is first laid out.
    pub fn fit(mut self) -> Self {
        self.fit = true;
        self
    }

    /// The drawing as SVG, at its own size.
    pub fn to_svg(&self, theme: &Theme) -> String {
        svg::write(self, theme)
    }

    /// The nodes' boxes, and each link's place and its two nodes: a link to an id no node
    /// has is left out.
    fn plane(&self) -> (Vec<Rect>, Vec<(usize, usize, usize)>) {
        let rects = self.nodes.iter().map(|n| Rect::new(n.at.x, n.at.y, n.size.width, n.size.height));
        let of: HashMap<&str, usize> = self.nodes.iter().enumerate().map(|(i, n)| (&*n.id, i)).collect();
        let ends = self.links.iter().enumerate();
        let ends = ends.filter_map(|(k, l)| Some((k, *of.get(&*l.from)?, *of.get(&*l.to)?))).collect();
        (rects.collect(), ends)
    }

    /// Each group's title and frame: 12 px out from its nodes, 24 above them for its title.
    fn frames(&self, rects: &[Rect]) -> Vec<(String, Rect)> {
        let of: HashMap<&str, Rect> = self.nodes.iter().zip(rects).map(|(n, r)| (&*n.id, *r)).collect();
        let frame = |(title, ids): &(String, Vec<Rc<str>>)| {
            let r = ids.iter().filter_map(|id| of.get(&**id)).copied().reduce(|a, b| a.union(b))?;
            Some((title.clone(), Rect::new(r.x - 12.0, r.y - 24.0, r.width + 24.0, r.height + 36.0)))
        };
        self.groups.iter().filter_map(frame).collect()
    }

    /// What it covers: its nodes and its frames.
    pub fn bounds(&self) -> Rect {
        let (rects, _) = self.plane();
        let frames = self.frames(&rects).into_iter().map(|(_, r)| r);
        rects.into_iter().chain(frames).reduce(|a, b| a.union(b)).unwrap_or_default()
    }
}

/// The box of `r` as the view shows it, bound to the view.
fn placed(face: Div, view: Signal<GraphView>, r: Rect) -> Div {
    let at = move |side: fn(Rect) -> f32| move || side(view.get().rect(r));
    let face = face.absolute().left(at(|r| r.x)).top(at(|r| r.y));
    face.width(at(|r| r.width)).height(at(|r| r.height))
}

impl View for Graph {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let view = self.view.unwrap_or_else(|| signal(GraphView::default()));
        let (rects, ends) = self.plane();
        let (frames, bounds) = (self.frames(&rects), self.bounds());
        let (rects, ends) = (Rc::new(rects), Rc::new(ends));
        let frames = frames.into_iter().map(|(title, r)| {
            let frame = div().border(1.0).border_color(token::LINE_STRONG).radius(6.0).padding_x(8.0);
            let frame = frame.padding_y(4.0).text_size(size::TEXT_SM.size).text_color(token::TEXT_3);
            placed(frame, view, r).child(text(title))
        });
        let drawn = LinksView { view, rects: rects.clone(), ends: ends.clone(), style: self.style.clone() };
        let (click, hover) = (self.click.clone(), self.hover.clone());
        // What the pointer is over and whether it drags, shared by the nodes and the box.
        let (over, moved) = (Rc::new(RefCell::new(None)), Rc::new(Cell::new(false)));
        let shared = (over.clone(), moved.clone());
        let nodes = self.nodes.into_iter().zip(rects.iter().copied()).map(|(node, r)| {
            // Its view at its own size, scaled about the middle of its box.
            // ponytail: its letters are drawn at their size and scaled, soft past a zoom of 1;
            // text drawn at the zoom when graphs are read zoomed in.
            let zoom = move || view.get().zoom;
            let face =
                column().align_items(Align::Stretch).absolute().width(r.width).height(r.height).scale(zoom);
            let face = face.left(move || r.width * (zoom() - 1.0) / 2.0);
            let face_of = face.top(move || r.height * (zoom() - 1.0) / 2.0).child((node.face)());
            let (id, told, came, left) = (node.id, click.clone(), hover.clone(), hover.clone());
            let (pressed, keyed, on, told_key) = (id.clone(), id.clone(), id, told.clone());
            let (came_over, left_over, dragged) = (over.clone(), over.clone(), moved.clone());
            // Its box cuts what is under the pointer: the view's own box, larger while zoomed out,
            // is not hit past it.
            let face = placed(div(), view, r).overflow_x(Overflow::Hidden).role(Role::Button);
            let face = face.label(node.label).child(face_of).focusable();
            // Enter and Space do what a click does: a node is reached by Tab.
            let face = face.on_key_down(move |e| {
                if matches!(e.key, Key::Enter | Key::Space) && e.mods == Mods::NONE {
                    e.stop();
                    told_key.iter().for_each(|f| f(Hit::Node(keyed.clone())));
                }
            });
            let face = face.on_click(move |e| {
                e.stop();
                if !dragged.get() {
                    told.iter().for_each(|f| f(Hit::Node(pressed.clone())));
                }
            });
            let face = face.on_pointer(PointerKind::Enter, move |_| {
                *came_over.borrow_mut() = Some(Hit::Node(on.clone()));
                came.iter().for_each(|f| f(Some(Hit::Node(on.clone()))))
            });
            face.on_pointer(PointerKind::Leave, move |_| {
                *left_over.borrow_mut() = None;
                left.iter().for_each(|f| f(None))
            })
        });
        let world = div().absolute().left(0.0).top(0.0).right(0.0).bottom(0.0).cut_to_parent();
        let world =
            world.children(frames.collect::<Vec<_>>()).child(drawn).children(nodes.collect::<Vec<_>>());
        let all = div().width(Length::Fill).height(self.height).shrink(0.0).role(Role::Group).child(world);
        let all = pointer::pointer(all, view, (rects, ends), (self.click, self.hover), shared, bounds);
        let fit = self.fit;
        all.on_mount(move |tree, id| {
            let (size, done) = (tree.size_signal(id), Cell::new(!fit));
            effect(move || {
                let size = size.get();
                if size.width > 0.0 && !done.replace(true) {
                    view.set(fitted(bounds, size));
                }
            });
        })
        .mount(cx)
    }
}
