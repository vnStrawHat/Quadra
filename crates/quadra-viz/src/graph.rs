//! The graph (specs/13, "Charts"): nodes that are views at points of its own plane, links
//! between them as curves with heads, frames around groups; panned by a drag, zoomed by the
//! wheel with Ctrl, fitted to its box; and the drawing as SVG.

mod links;
mod parts;
mod pointer;
mod svg;

pub use self::links::LinkAxis;
pub use self::parts::{Link, LinkStyle, Node};
pub use self::pointer::{GraphView, fitted};

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use quadra_design::{size, token};
use quadra_layout::{Align, Length, Overflow};
use quadra_platform::{Key, Mods};
use quadra_prim::{Point, Rect};
use quadra_reactive::{Signal, effect, signal};
use quadra_tree::{NodeId, PointerKind, Role, Theme};
use quadra_view::{Div, MountCx, View, column, div, text};

use self::links::{LinksView, route};

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
    /// `None`: 360 px, or as tall as its drawing when still.
    height: Option<Length>,
    fit: bool,
    still: bool,
    /// How its links run: along which axis, and whether straight.
    lines: (LinkAxis, bool),
}

/// A graph of `nodes` and the `links` between them, as wide as its box and 360 px tall.
pub fn graph(nodes: impl IntoIterator<Item = Node>, links: impl IntoIterator<Item = Link>) -> Graph {
    let (nodes, links) = (nodes.into_iter().collect(), links.into_iter().collect());
    let style: Styles = Rc::new(|_| LinkStyle::default());
    let (groups, view, click, hover) = (Vec::new(), None, None, None);
    let lines = (LinkAxis::Auto, false);
    let (height, fit, still) = (None, false, false);
    Graph { nodes, links, groups, style, view, click, hover, height, fit, still, lines }
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
        self.height = Some(Length::Px(px));
        self
    }

    /// As tall as the box it is in, following it: a view under a toolbar.
    pub fn fill(mut self) -> Self {
        self.height = Some(Length::Fill);
        self
    }

    /// All of it in view when its box is first laid out.
    pub fn fit(mut self) -> Self {
        self.fit = true;
        self
    }

    /// A diagram that does not move: no pan, no zoom, every wheel to the page; clicks and
    /// hover as before.
    pub fn still(mut self) -> Self {
        self.still = true;
        self
    }

    /// The axis its links run along: side to side for a map in columns.
    pub fn axis(mut self, axis: LinkAxis) -> Self {
        self.lines.0 = axis;
        self
    }

    /// Its links straight from stop to stop, not curves: columns near each other.
    pub fn straight(mut self) -> Self {
        self.lines.1 = true;
        self
    }

    /// The drawing as SVG, at its own size, on the theme's canvas.
    pub fn to_svg(&self, theme: &Theme) -> String {
        svg::write(self, theme, Some(quadra_design::token::CANVAS.into()))
    }

    /// The drawing as SVG on `ground`, or on nothing.
    pub fn to_svg_on(&self, theme: &Theme, ground: Option<quadra_tree::Paint>) -> String {
        svg::write(self, theme, ground)
    }

    /// The nodes' boxes, and each link's place and line (none for an id no node has).
    fn plane(&self) -> (Vec<Rect>, Vec<(usize, Vec<Point>)>) {
        let rects: Vec<Rect> =
            self.nodes.iter().map(|n| Rect::new(n.at.x, n.at.y, n.size.width, n.size.height)).collect();
        let of: HashMap<&str, usize> = self.nodes.iter().enumerate().map(|(i, n)| (&*n.id, i)).collect();
        let line = |(k, l): (usize, &Link)| {
            Some((k, route(rects[*of.get(&*l.from)?], rects[*of.get(&*l.to)?], &l.through, self.lines)))
        };
        let routes = self.links.iter().enumerate().filter_map(line).collect();
        (rects, routes)
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
        let points = self.links.iter().flat_map(|l| &l.through).map(|p| Rect::new(p.x, p.y, 0.0, 0.0));
        rects.into_iter().chain(frames).chain(points).reduce(|a, b| a.union(b)).unwrap_or_default()
    }
}

/// `content` at the size of `r` in the plane, scaled about the middle of the box `placed`
/// gives `r` and drawn again at the zoom, its letters sharp.
fn at_zoom(view: Signal<GraphView>, r: Rect, content: Div) -> Div {
    let zoom = move || view.get().zoom;
    let face = column().align_items(Align::Stretch).absolute().width(r.width).height(r.height);
    let face = face.left(move || r.width * (zoom() - 1.0) / 2.0).top(move || r.height * (zoom() - 1.0) / 2.0);
    face.scale(zoom).sharp_scale().child(content)
}

/// The box of `r` as the view shows it, bound to the view.
fn placed(face: Div, view: Signal<GraphView>, r: Rect) -> Div {
    let at = move |side: fn(Rect) -> f32| move || side(view.get().rect(r));
    let face = face.absolute().left(at(|r| r.x)).top(at(|r| r.y));
    face.width(at(|r| r.width)).height(at(|r| r.height))
}

impl View for Graph {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (rects, routes) = self.plane();
        let (frames, bounds, routes) = (self.frames(&rects), self.bounds(), Rc::new(routes));
        // Still, its drawing at zoom 1 with 16 px around it, as tall as that.
        let drawing = GraphView { pan: Point::new(16.0 - bounds.x, 16.0 - bounds.y), zoom: 1.0 };
        let view =
            self.view.unwrap_or_else(|| signal(if self.still { drawing } else { GraphView::default() }));
        let height = self.height.unwrap_or(Length::Px(if self.still { bounds.height + 32.0 } else { 360.0 }));
        // A frame's title in the plane as its nodes are: its band never under the title.
        let frames = frames.into_iter().map(|(title, r)| {
            let words = div().padding_x(8.0).padding_y(4.0).text_size(size::TEXT_SM.size).child(text(title));
            let frame = div().border(1.0).border_color(token::LINE_STRONG).radius(6.0);
            placed(frame, view, r).child(at_zoom(view, r, words.text_color(token::TEXT_3)))
        });
        let drawn = LinksView { view, routes: routes.clone(), style: self.style.clone() };
        let (click, hover) = (self.click.clone(), self.hover.clone());
        // What the pointer is over and whether it drags, shared by the nodes and the box.
        let (over, moved) = (Rc::new(RefCell::new(None)), Rc::new(Cell::new(false)));
        let shared = (over.clone(), moved.clone());
        let nodes = self.nodes.into_iter().zip(rects.iter().copied()).map(|(node, r)| {
            let face_of = at_zoom(view, r, (node.face)());
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
        let all = div().width(Length::Fill).height(height).shrink(0.0).role(Role::Group).child(world);
        let all = pointer::pointer(all, view, routes, (self.click, self.hover), shared, (bounds, self.still));
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
