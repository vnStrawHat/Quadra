//! Rich text (specs/05, "Rules"): one paragraph of spans, each saying what it has otherwise
//! than the text around it, some of them links.

use std::cell::Cell;
use std::rc::Rc;

use quadra_platform::Cursor;
use quadra_text::Shaping;
use quadra_tree::{LinkBoxes, NodeId, Paint, PointerEvent, PointerKind, RichSpan, RichText, Style};

use crate::view::{MountCx, View};

/// A part of a rich text.
pub struct Span {
    text: String,
    shaping: Shaping,
    color: Option<Paint>,
    link: Option<Rc<str>>,
}

/// A span that is as the text around it, until it says otherwise.
pub fn span(text: impl Into<String>) -> Span {
    Span { text: text.into(), shaping: Shaping::default(), color: None, link: None }
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

    /// Makes the span a link: underlined, and a click on it gives `target` to what the
    /// text's `on_link` was given.
    pub fn link(mut self, target: &str) -> Self {
        self.link = Some(Rc::from(target));
        self
    }
}

/// What is given the target of a link that is clicked.
type OnLink = Rc<dyn Fn(&str)>;

pub struct RichTextView {
    spans: Vec<Span>,
    on_link: Option<OnLink>,
}

/// A paragraph of spans, wrapped as one text.
pub fn rich_text(spans: impl IntoIterator<Item = Span>) -> RichTextView {
    RichTextView { spans: spans.into_iter().collect(), on_link: None }
}

impl RichTextView {
    /// Runs `f` with the target of a link when it is clicked.
    pub fn on_link(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.on_link = Some(Rc::new(f));
        self
    }
}

/// What makes the links of a mounted rich text links: a hand over them, and their click.
fn link(cx: &mut MountCx, id: NodeId, boxes: LinkBoxes, on_link: Option<OnLink>) {
    let under = move |e: &PointerEvent| {
        let boxes = boxes.borrow();
        boxes.iter().find(|(part, _)| part.contains(e.local)).map(|(_, target)| target.clone())
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

impl View for RichTextView {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (mut text, mut spans) = (String::new(), Vec::new());
        for Span { text: words, shaping, color, link } in self.spans {
            text.push_str(&words);
            spans.push(RichSpan { end: text.len(), shaping, color, link });
        }
        let links = spans.iter().any(|span| span.link.is_some());
        let rich = RichText::new(text, spans);
        let boxes = rich.links();
        let id = cx.tree.insert_widget(Style::default(), Box::new(rich));
        if links {
            link(cx, id, boxes, self.on_link);
        }
        id
    }
}
