//! The view of a document (specs/13, "Markdown"; its look is behaviour/15, "Type"): the
//! blocks in a column, each a child of its own that stays while its source does.

mod code;
mod hint;
mod picture;
mod table;

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use quadra_kit::{fonts, icon, icons, size, token};
use quadra_layout::{Align as Across, Justify, Length};
use quadra_reactive::{memo, signal};
use quadra_text::find_urls;
use quadra_tree::{NodeId, Role};
use quadra_ui::{
    Div, IntoProp, MountCx, Prop, Span as Run, View, column, div, each, rich_text, row, scroll_x, span, text,
};

use crate::block::{Kind, parse};
use crate::inline::parse_inline;
use crate::mermaid::diagram;
use crate::outline::plain;
use crate::tex::formula;
use code::code_box;
use table::Table;

/// The size of the text of a document, and of the code in it.
const BODY: f32 = 16.0;
const CODE: f32 = 14.0;
/// How deep items and quotes are shown nested: a document that is not ours does not say
/// how wide the page is.
const DEEPEST: usize = 12;

type OnLink = Rc<dyn Fn(&str)>;

pub struct Markdown {
    source: Prop<String>,
    on_link: Option<OnLink>,
    base: Option<PathBuf>,
}

/// A document, rendered. The source is a property: when it changes, the blocks whose source
/// did not change keep their nodes.
pub fn markdown<M>(source: impl IntoProp<String, M>) -> Markdown {
    Markdown { source: source.into_prop(), on_link: None, base: None }
}

impl Markdown {
    /// Runs `f` with the target of a link when it is clicked with Ctrl held.
    pub fn on_link(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.on_link = Some(Rc::new(f));
        self
    }

    /// The folder the document is in: where its images are looked for when their paths
    /// do not start at a root.
    pub fn base(mut self, folder: impl Into<PathBuf>) -> Self {
        self.base = Some(folder.into());
        self
    }
}

/// A block as the column knows it: by what it says, and which of those that say the same
/// it is. How far it is from the one before may change under it.
#[derive(Clone, PartialEq)]
struct Entry {
    key: u64,
    gap: f32,
    kind: Rc<Kind>,
}

/// The blocks of a source. Blocks are 16 px apart, the items of one list 4.
fn entries(source: &str) -> Vec<Entry> {
    let blocks = parse(source);
    let alone = blocks.len() == 1;
    let (mut seen, mut out, mut listed_before) = (HashMap::new(), Vec::<Entry>::new(), false);
    // The front matter says things about the document: it shows when there is nothing else.
    for block in blocks.into_iter().filter(|block| alone || !matches!(block.kind, Kind::Front { .. })) {
        let listed = matches!(block.kind, Kind::Item { .. });
        let gap: f32 = match (out.is_empty(), listed && listed_before) {
            (true, _) => 0.0,
            (false, true) => 4.0,
            (false, false) => 16.0,
        };
        let mut hasher = DefaultHasher::new();
        block.kind.hash(&mut hasher);
        let nth = seen.entry(hasher.finish()).or_insert(0u64);
        *nth += 1;
        out.push(Entry { key: hasher.finish().wrapping_add(*nth), gap, kind: Rc::new(block.kind) });
        listed_before = listed;
    }
    out
}

/// A text of a document: its links are links while Ctrl is held, and say where they go when
/// the pointer rests on them.
fn rich(spans: Vec<Run>, on_link: &Option<OnLink>) -> hint::Hinted {
    let words = rich_text(spans).ctrl_links();
    hint::Hinted(match on_link.clone() {
        Some(open) => words.on_link(move |target| open(target)),
        None => words,
    })
}

/// `text` in the style `styled` gives, cut where its URLs are: each is a link that keeps
/// the colour around it.
fn cut(text: &str, styled: impl Fn(&str) -> Run, out: &mut Vec<Run>) {
    let mut from = 0;
    for url in find_urls(text) {
        out.extend((url.range.start > from).then(|| styled(&text[from..url.range.start])));
        out.push(styled(&text[url.range.clone()]).link(&url.url));
        from = url.range.end;
    }
    out.extend((from < text.len()).then(|| styled(&text[from..])));
}

/// The spans of a block's text.
fn spans(source: &str) -> Vec<Run> {
    let mut out = Vec::new();
    for part in parse_inline(source) {
        // An image among words is its words in brackets: a paragraph of images alone shows
        // them (`picture`).
        let said = if part.image.is_some() { format!("[{}]", part.text) } else { part.text.clone() };
        let styled = |text: &str| {
            let run = span(text);
            let run = if part.bold { run.bold() } else { run };
            let run = if part.italic { run.italic() } else { run };
            let run = if part.strike { run.strike().color(token::TEXT_3) } else { run };
            let code = part.code || part.math;
            if code { run.family(fonts::MONO).color(token::SYNTAX_STRING).size(BODY * 0.875) } else { run }
        };
        match (&part.link, part.code) {
            _ if said.is_empty() => {}
            // A formula is drawn in the line; one with a fault is its source, in the colour
            // of what is wrong.
            _ if part.math => out.push(formula(&said).span().size(BODY)),
            (Some(link), _) => out.push(styled(&said).link(link).color(token::ACCENT_TEXT)),
            (None, true) => cut(&said, styled, &mut out),
            (None, false) => out.push(styled(&said)),
        }
    }
    out
}

fn heading(level: u8, source: &str, on_link: &Option<OnLink>) -> Div {
    // 28, 22.4, 19.2, 16, 14 and 13.6 px, semibold; the first two over a line.
    let scale = [1.75, 1.4, 1.2, 1.0, 0.875, 0.85][usize::from(level.clamp(1, 6)) - 1];
    let head = column().role(Role::Heading).label(plain(source)).padding_top(8.0).line_height(1.25);
    let head = head.text_size(BODY * scale).font_weight(size::WEIGHT_STRONG);
    let head = head.text_color(if level == 6 { token::TEXT_2 } else { token::TEXT });
    let head = head.child(rich(spans(source), on_link));
    match level {
        1 | 2 => head
            .padding_bottom(if level == 1 { 8.0 } else { 4.0 })
            .border_bottom(1.0)
            .border_color(token::LINE),
        _ => head,
    }
}

fn item(
    number: Option<u32>,
    depth: usize,
    task: Option<bool>,
    source: &str,
    on_link: &Option<OnLink>,
) -> Div {
    let mark = row().width(22.0).shrink(0.0).height(BODY * 1.5).align_items(Across::Center);
    let mark = match (task, number) {
        (Some(done), _) => {
            // A box that is read, not clicked: outlined, with a check in the accent.
            let tick = row().width(16.0).height(16.0).radius(size::RADIUS_SM).border(1.0);
            let tick =
                tick.border_color(token::LINE_STRONG).align_items(Across::Center).justify(Justify::Center);
            let tick = tick.role(Role::Checkbox).label(plain(source)).checked(done);
            mark.child(if done { tick.child(icon(icons::CHECK).color(token::ACCENT_TEXT)) } else { tick })
        }
        (None, Some(number)) => mark.child(text(format!("{number}.")).color(token::TEXT_3)),
        (None, None) => mark.child(text(if depth == 0 { "•" } else { "◦" }).color(token::TEXT_3)),
    };
    let body = column().width(0.0).grow(1.0).child(rich(spans(source), on_link));
    let body = if task == Some(true) { body.text_color(token::TEXT_3) } else { body };
    row().padding_left(depth.min(DEEPEST) as f32 * 22.0).child(mark).child(body)
}

/// What is drawn from a source, a formula or a diagram, as a line of its own, in the
/// middle; one wider than the page scrolls sideways. One with a fault is its source in a
/// box, and what is wrong under it.
fn figure(drawn: impl View + 'static, fault: Option<String>, source: &str) -> Div {
    if let Some(fault) = fault {
        let note = column().text_size(size::TEXT_SM.size).text_color(token::TEXT_3).child(text(fault));
        return column().gap(6.0).child(code_box("", source, &None)).child(note);
    }
    let line = row().justify(Justify::Center).padding_y(6.0).child(drawn);
    column().child(scroll_x(signal(0.0), line))
}

fn math(source: &str) -> Div {
    let formula = formula(source).display();
    let fault = formula.fault().map(|fault| format!("Math: {fault}"));
    figure(formula, fault, source)
}

/// A diagram: its text is that of the controls of the kit, not of the page.
fn chart(source: &str) -> Div {
    let diagram = diagram(source);
    let fault = diagram.fault().map(|fault| format!("Diagram: {fault}"));
    figure(row().text_size(size::TEXT_MD.size).child(diagram), fault, source)
}

fn block(kind: &Kind, on_link: &Option<OnLink>, base: Option<&Path>) -> Div {
    let words = |source: &str| match spans(source) {
        runs if runs.is_empty() => column(),
        runs => column().child(rich(runs, on_link)),
    };
    match kind {
        Kind::Paragraph { text } => match parse_inline(text) {
            parts if picture::only_images(&parts) => picture::pictures(&parts, base),
            _ => words(text),
        },
        Kind::Heading { level, text } => heading(*level, text, on_link),
        Kind::Item { number, depth, task, text } => item(*number, *depth, *task, text, on_link),
        Kind::Code { lang, text } if lang == "mermaid" => chart(text),
        Kind::Code { lang, text } => code_box(lang, text, on_link),
        Kind::Math { text } => math(text),
        Kind::Front { text } => code_box("yaml", text, on_link),
        Kind::Quote { depth, text } => {
            let bar = words(text).border_left(4.0).border_color(token::LINE_STRONG).padding_left(16.0);
            let nested = (depth.clamp(&1, &DEEPEST) - 1) as f32 * 16.0;
            column().padding_left(nested).child(bar.text_color(token::TEXT_2))
        }
        Kind::Rule => column().child(div().height(1.0).bg(token::LINE_STRONG)),
        Kind::Table { align, rows } => {
            // One wider than the page, its columns at their least, scrolls sideways.
            let table = Table { align: align.clone(), rows: rows.clone(), on_link: on_link.clone() };
            column().child(scroll_x(signal(0.0), row().min_width(Length::Fill).child(table)))
        }
    }
}

impl View for Markdown {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (on_link, base) = (self.on_link, self.base);
        let read: Box<dyn Fn() -> String> = match self.source {
            Prop::Value(source) => Box::new(move || source.clone()),
            Prop::Reactive(read) => read,
        };
        // ponytail: every block is a child, and a change costs the count of blocks; a list
        // of the blocks that show when documents of tens of thousands of lines are read.
        let blocks = memo(move || entries(&read()));
        let shown = move |entry: quadra_reactive::Signal<Entry>| {
            let kind = entry.with_untracked(|entry| entry.kind.clone());
            let block = block(&kind, &on_link, base.as_deref());
            column().padding_top(move || entry.with(|entry| entry.gap)).child(block)
        };
        let page = column().width(Length::Fill).text_color(token::TEXT).text_size(BODY).line_height(1.5);
        each(move || blocks.get(), |entry| entry.key, shown).container(page).mount(cx)
    }
}
