//! Rich text (specs/05, "Rules"): one paragraph of spans, each saying what it has otherwise
//! than the text around it, some of them links.

use std::cell::Cell;
use std::rc::Rc;

use quadra_platform::{Cursor, Mods};
use quadra_text::Shaping;
use quadra_tree::{
    InlineBox, LinkBoxes, NodeId, Paint, PointerEvent, PointerKind, RichSpan, RichText, Style,
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

pub struct RichTextView {
    spans: Vec<Span>,
    on_link: Option<OnLink>,
    ctrl: bool,
}

/// A paragraph of spans, wrapped as one text.
pub fn rich_text(spans: impl IntoIterator<Item = Span>) -> RichTextView {
    RichTextView { spans: spans.into_iter().collect(), on_link: None, ctrl: false }
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
}

/// What makes the links of a mounted rich text links: a hand over them, and their click.
fn link(cx: &mut MountCx, id: NodeId, boxes: LinkBoxes, on_link: Option<OnLink>, ctrl: bool) {
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

impl View for RichTextView {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (mut text, mut spans, mut boxes) = (String::new(), Vec::new(), Vec::new());
        for Span { text: words, shaping, color, background, strike, link, inline } in self.spans {
            text.push_str(&words);
            boxes.extend(inline.map(|inline| (spans.len(), inline)));
            spans.push(RichSpan { end: text.len(), shaping, color, background, strike, link });
        }
        let links = spans.iter().any(|span| span.link.is_some());
        let rich = RichText::new(text, spans);
        let rich = boxes.into_iter().fold(rich, |rich, (span, inline)| rich.with_box(span, inline));
        let boxes = rich.links();
        let id = cx.tree.insert_widget(Style::default(), Box::new(rich));
        if links {
            link(cx, id, boxes, self.on_link, self.ctrl);
        }
        id
    }
}
