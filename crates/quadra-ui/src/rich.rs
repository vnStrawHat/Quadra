//! Rich text (specs/05, "Rules"): one paragraph of spans, each saying what it has otherwise
//! than the text around it, some of them links.

use std::cell::Cell;
use std::rc::Rc;

use quadra_platform::{Cursor, Key, Mods};
use quadra_prim::Point;
use quadra_text::Shaping;
use quadra_tree::{
    InlineBox, LinkBoxes, NodeId, Paint, PointerEvent, PointerKind, RichSpan, RichText, Role, Style,
};

use crate::view::{MountCx, View};

/// A part of a rich text.
pub struct Span {
    text: String,
    shaping: Shaping,
    color: Option<Paint>,
    background: Option<Paint>,
    strike: bool,
    link: Option<Rc<str>>,
    inline: Option<Box<dyn InlineBox>>,
}

/// A span that is as the text around it, until it says otherwise.
pub fn span(text: impl Into<String>) -> Span {
    let (text, shaping) = (text.into(), Shaping::default());
    Span { text, shaping, color: None, background: None, strike: false, link: None, inline: None }
}

/// A span that is no text: something drawn in the line, on its baseline, as wide as it
/// says (a formula). It is in the word around it, and takes the colour of a span.
pub fn inline(what: impl InlineBox + 'static) -> Span {
    Span { inline: Some(Box::new(what)), ..span("\u{FFFC}") }
}

impl Span {
    pub fn bold(self) -> Self {
        self.weight(700)
    }

    pub fn weight(mut self, weight: u16) -> Self {
        self.shaping.weight = Some(weight);
        self
    }

    pub fn italic(mut self) -> Self {
        self.shaping.italic = Some(true);
        self
    }

    /// The font size in pixels.
    pub fn size(mut self, px: f32) -> Self {
        self.shaping.size = Some(px);
        self
    }

    pub fn family(mut self, family: &str) -> Self {
        self.shaping.family = Some(Rc::from(family));
        self
    }

    /// A colour, or a role of the theme.
    pub fn color(mut self, color: impl Into<Paint>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// A fill behind the span, as high as its line: a colour, or a role of the theme.
    pub fn background(mut self, color: impl Into<Paint>) -> Self {
        self.background = Some(color.into());
        self
    }

    /// A line through the span, in its colour.
    pub fn strike(mut self) -> Self {
        self.strike = true;
        self
    }

    /// Makes the span a link: underlined, and a click on it gives `target` to what the
    /// text's `on_link` was given.
    pub fn link(mut self, target: &str) -> Self {
        self.link = Some(Rc::from(target));
        self
    }
}

/// What is given the target of a link that is clicked.
type OnLink = Rc<dyn Fn(&str)>;
/// What is told the link the pointer is over: the point of the window under its left end,
/// and its target; `None` when it leaves the links.
type OnHover = Rc<dyn Fn(Option<(Point, Rc<str>)>)>;

pub struct RichTextView {
    spans: Vec<Span>,
    on_link: Option<OnLink>,
    on_hover: Option<OnHover>,
    ctrl: bool,
}

/// A paragraph of spans, wrapped as one text.
pub fn rich_text(spans: impl IntoIterator<Item = Span>) -> RichTextView {
    RichTextView { spans: spans.into_iter().collect(), on_link: None, on_hover: None, ctrl: false }
}

impl RichTextView {
    /// Runs `f` with the target of a link when it is clicked.
    pub fn on_link(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.on_link = Some(Rc::new(f));
        self
    }

    /// Links are links only while Ctrl is held: for content the user of the app did not
    /// write, where a plain click must not open anything.
    pub fn ctrl_links(mut self) -> Self {
        self.ctrl = true;
        self
    }

    /// Runs `f` when the pointer comes over a link, Ctrl held or not, with the point of the
    /// window under the link's left end and its target, and with `None` when it leaves them:
    /// what a hint of where a link goes follows.
    pub fn on_link_hover(mut self, f: impl Fn(Option<(Point, Rc<str>)>) + 'static) -> Self {
        self.on_hover = Some(Rc::new(f));
        self
    }
}

/// What makes the links of a mounted rich text links: a hand over them, and their click.
fn link(cx: &mut MountCx, id: NodeId, boxes: LinkBoxes, on: (Option<OnLink>, Option<OnHover>), ctrl: bool) {
    let (on_link, on_hover) = on;
    let (all, last) = (boxes.clone(), Rc::new(std::cell::RefCell::new(None::<Rc<str>>)));
    let hover = move |e: &mut PointerEvent| {
        let Some(tell) = &on_hover else { return };
        let found = all
            .borrow()
            .iter()
            .find(|(part, _)| e.kind == PointerKind::Move && part.contains(e.local))
            .cloned();
        let target = found.as_ref().map(|(_, target)| target.clone());
        if *last.borrow() != target {
            *last.borrow_mut() = target;
            let corner = Point::new(e.at.x - e.local.x, e.at.y - e.local.y);
            tell(
                found.map(|(part, target)| (Point::new(corner.x + part.x, corner.y + part.bottom()), target)),
            );
        }
    };
    cx.tree.on_pointer(id, PointerKind::Move, hover.clone());
    cx.tree.on_pointer(id, PointerKind::Leave, hover);
    let under = move |e: &PointerEvent| {
        let boxes = boxes.borrow();
        let held = !ctrl || e.mods.contains(Mods::CTRL);
        boxes.iter().find(|(part, _)| held && part.contains(e.local)).map(|(_, target)| target.clone())
    };
    // The node's cursor is written when the pointer comes over a link or leaves one, not at
    // every move.
    let (hand, handle, over) = (Rc::new(Cell::new(false)), cx.handle.clone(), under.clone());
    let point = move |e: &mut PointerEvent| {
        let on = e.kind == PointerKind::Move && over(e).is_some();
        if hand.replace(on) != on {
            handle.with(move |tree| tree.set_cursor(id, on.then_some(Cursor::Pointer)));
        }
    };
    cx.tree.on_pointer(id, PointerKind::Move, point.clone());
    cx.tree.on_pointer(id, PointerKind::Leave, point);
    cx.tree.on_pointer(id, PointerKind::Click, move |e| {
        if let (Some(target), Some(on_link)) = (under(e), &on_link) {
            on_link(&target);
        }
    });
}

/// What the keyboard and a reader reach the links of a mounted text by: a node for each,
/// in the order of the text, without a size of its own, which takes the focus (Tab), is a
/// link named by its words, and gives its target to `on_link` on Enter or a reader's click.
/// The text outlines the one that has the focus.
fn reachable(
    cx: &mut MountCx,
    text: NodeId,
    links: Vec<(String, Rc<str>)>,
    on_link: Option<OnLink>,
) -> Vec<NodeId> {
    let mut nodes = Vec::new();
    for (k, (words, target)) in links.into_iter().enumerate() {
        let (handle, on) = (cx.handle.clone(), on_link.clone());
        let go = move || {
            if let Some(on) = &on {
                on(&target);
            }
        };
        let enter = go.clone();
        let node = crate::div().absolute().focusable().role(Role::Link).label(words);
        let node = node.on_click(move |_| go()).on_key_down(move |e| {
            if e.key == Key::Enter {
                e.stop();
                enter();
            }
        });
        let node = node.on_focus(move |has| {
            handle.with(move |tree| {
                let outlined = tree.widget_mut::<RichText>(text).map(|w| w.focus_link(has.then_some(k)));
                if outlined == Some(true) {
                    tree.mark_paint(text);
                }
            })
        });
        nodes.push(node.mount(cx));
    }
    nodes
}

impl View for RichTextView {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (mut text, mut spans, mut boxes, mut named) = (String::new(), Vec::new(), Vec::new(), Vec::new());
        for Span { text: words, shaping, color, background, strike, link, inline } in self.spans {
            text.push_str(&words);
            boxes.extend(inline.map(|inline| (spans.len(), inline)));
            named.extend(link.clone().map(|target| (words, target)));
            spans.push(RichSpan { end: text.len(), shaping, color, background, strike, link });
        }
        let rich = RichText::new(text, spans);
        let rich = boxes.into_iter().fold(rich, |rich, (span, inline)| rich.with_box(span, inline));
        let boxes = rich.links();
        let id = cx.tree.insert_widget(Style::default(), Box::new(rich));
        if named.is_empty() {
            return id;
        }
        let reached = reachable(cx, id, named, self.on_link.clone());
        link(cx, id, boxes, (self.on_link, self.on_hover), self.ctrl);
        // The text and the nodes of its links, in a box that is as the text is.
        let around = cx.tree.insert(Style::default());
        cx.tree.set_children(around, &[&[id][..], &reached].concat());
        around
    }
}
