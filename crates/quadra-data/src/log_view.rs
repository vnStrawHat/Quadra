//! The log view (specs/13, "Data"): the lines of a log in a list, the newest kept in view,
//! under a bar with a filter.

use std::cell::Cell;
use std::rc::Rc;

use quadra_kit::rows::{one_column, selectable, sideways};
use quadra_kit::{Size, Variant, button, fonts, input, token, tooltip};
use quadra_layout::{Align, Overflow};
use quadra_platform::Clipboard;
use quadra_prim::Color;
use quadra_reactive::{Signal, effect, memo, signal, use_context};
use quadra_tree::{NodeId, Paint};
use quadra_ui::{Div, MountCx, View, column, div, dynamic, rich_text, row, span, text, uniform_list};

use crate::ansi::{Ink, Pen, plain, runs};
use crate::log::{Log, clock};

/// The height of a row, and what a time takes before its line: twelve letters of the mono
/// face at 12 px, 7.2 px each, and 20 px of gaps.
const ROW: f32 = 18.0;
const GUTTER: f32 = 12.0 * 7.2 + 20.0;
/// A place past the end of any list: the list keeps it at its end, which is following.
const FOLLOWING: f32 = f32::MAX;

pub struct LogView {
    log: Log,
    timestamps: bool,
}

/// The lines of a log, with the time of each before it.
pub fn log_view(log: &Log) -> LogView {
    LogView { log: log.clone(), timestamps: true }
}

impl LogView {
    /// Whether the time of each line is before it.
    pub fn timestamps(mut self, shown: bool) -> Self {
        self.timestamps = shown;
        self
    }
}

/// The colour a pen writes in; `fade` is how much of it a faint line keeps.
fn paint(pen: Pen, fade: f32) -> Paint {
    let fade = if pen.faint { fade * 0.6 } else { fade };
    match pen.ink {
        Ink::Text => token::TEXT.alpha(fade),
        Ink::Palette(n) => token::ANSI[n as usize % 16].alpha(fade),
        Ink::Rgb(r, g, b) => Paint::Color(Color::rgb(u32::from_be_bytes([0, r, g, b])).alpha(fade)),
    }
}

/// The row of the line at a place of the listing; what it says moved left by `sx`.
fn line(log: &Log, place: usize, timestamps: bool, sx: Signal<f32>) -> Div {
    let Some((_, at_ms, raw, faint)) = log.line(place) else { return div() };
    let (fade, (said, pens)) = (if faint { 0.4 } else { 1.0 }, runs(&raw));
    let mut from = 0;
    let spans = pens.into_iter().map(|(end, pen)| {
        let run = span(&said[std::mem::replace(&mut from, end)..end]).color(paint(pen, fade));
        if pen.bold { run.bold() } else { run }
    });
    let spans: Vec<_> = spans.collect();
    let line = row().height(ROW).align_items(Align::Center).padding_left(8.0);
    let line = match (timestamps, at_ms) {
        (false, _) => line,
        (true, 0) => line.child(div().width(GUTTER - 8.0).shrink(0.0)),
        (true, at) => {
            let time = text(clock(at)).color(token::TEXT_3.alpha(fade));
            line.child(div().width(GUTTER - 8.0).shrink(0.0).child(time))
        }
    };
    if spans.is_empty() {
        return line;
    }
    // A long line is cut at the edge and scrolls sideways; the times stay.
    // As wide as the line at least (a byte is a cell or more of it): it never wraps.
    let wide = said.len() as f32 * 7.2 + 8.0;
    let words = div().absolute().top(0.0).left(move || -sx.get()).width(wide).child(rich_text(spans));
    line.child(div().width(0.0).grow(1.0).height(ROW).overflow_x(Overflow::Hidden).child(words))
}

/// A line as copying gives it: its time, when they show, and what it says.
fn as_text(log: &Log, place: usize, timestamps: bool) -> String {
    let Some((_, at_ms, raw, _)) = log.line(place) else { return String::new() };
    match (timestamps, at_ms) {
        (true, at) if at != 0 => format!("{} {}", clock(at), plain(&raw)),
        _ => plain(&raw).into_owned(),
    }
}

/// A small button of the bar that is on or off, with what it does when the pointer rests.
fn toggle(
    label: &str,
    tip: &str,
    on: impl Fn() -> bool + 'static,
    click: impl FnMut() + 'static,
) -> impl View {
    tooltip(tip, button(label.to_string()).variant(Variant::Ghost).size(Size::Sm).toggled(on).on_click(click))
}

impl View for LogView {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (log, timestamps) = (self.log, self.timestamps);
        let (query, pattern, dim) = (signal(String::new()), signal(false), signal(false));
        let (changed, wrong) = log.signals();
        let filtered = log.clone();
        // Whether the pattern is one is the log's to say: the bar reads it there.
        effect(move || {
            filtered.set_filter(&query.get(), pattern.get(), dim.get());
        });

        // Where the list is; past its end, it follows.
        let scroll = signal(FOLLOWING);
        let follows = memo(move || scroll.get() == FOLLOWING);
        let (seen, mounted): (Rc<Cell<Option<Signal<quadra_prim::Size>>>>, _) =
            (Rc::default(), signal(false));
        let (count, view) = (log.clone(), seen.clone());
        let end = move || {
            mounted.get();
            let height = view.get().map_or(0.0, |size| size.get().height);
            (count.listed() as f32 * ROW - height).max(0.0)
        };
        // Scrolled down to the end, or with nothing to scroll: it follows again.
        let (to_end, before) = (end.clone(), Cell::new(FOLLOWING));
        effect(move || {
            let (at, end) = (scroll.get(), to_end());
            if at < FOLLOWING && (end == 0.0 || (at > before.get() && at >= end - 1.0)) {
                scroll.set(FOLLOWING);
            }
            before.set(at);
        });

        // Rows selected and copied as a table's are; other lines at the places, none.
        let (says, counted) = (log.clone(), log.clone());
        let sheet = one_column(move || counted.listed(), move |place| as_text(&says, place, timestamps));
        let chosen = sheet.chosen;
        effect(move || {
            changed.get();
            chosen.set(Vec::new());
        });
        let (sx, widest) = (signal(0.0f32), log.clone());
        let reach = move |width: f32| widest.widest() as f32 * 7.2 + 16.0 - (width - GUTTER);
        let (rows, counted, picks) = (log.clone(), log.clone(), sheet.clone());
        let list = uniform_list(
            move || counted.listed(),
            ROW,
            scroll,
            move |place| {
                let rows = rows.clone();
                // Built again for another place, and when other lines are listed: lines that
                // come change neither.
                let built = dynamic(move || {
                    changed.get();
                    line(&rows, place.get(), timestamps, sx)
                });
                let face = selectable(&picks, div().height(ROW), place);
                built.container(sideways(face, sx, reach.clone()))
            },
        );
        let viewport = div().grow(1.0).on_mount(move |tree, id| {
            seen.set(Some(tree.size_signal(id)));
            mounted.set(true);
        });
        let list = list.container(viewport);

        let (all, shown) = (log.clone(), log.clone());
        let counts = text(move || match (wrong.get(), query.with(String::is_empty) || dim.get()) {
            (true, _) => "Bad pattern".to_string(),
            (_, true) => format!("{} lines", all.len()),
            _ => format!("{} of {}", shown.listed(), all.len()),
        });
        let follow = move || scroll.set(if scroll.get_untracked() == FOLLOWING { end() } else { FOLLOWING });
        let bar = row().align_items(Align::Center).gap(4.0).padding_x(8.0).height(45.0).shrink(0.0);
        let bar = bar.border_bottom(1.0).border_color(token::LINE);
        let bar = bar.child(input(query).placeholder("Filter").width(240.0).invalid(wrong));
        let bar = bar.child(toggle(
            ".*",
            "Regular expression",
            move || pattern.get(),
            move || pattern.update(|on| *on = !*on),
        ));
        let bar = bar.child(toggle(
            "Dim",
            "Dim other lines instead of hiding them",
            move || dim.get(),
            move || dim.update(|on| *on = !*on),
        ));
        let bar =
            bar.child(div().grow(1.0)).child(div().text_size(12.0).text_color(token::TEXT_3).child(counts));
        let bar = bar.child(toggle("Follow", "Keep the newest line in view", move || follows.get(), follow));
        let lines = column().grow(1.0).font_family(fonts::MONO).text_size(12.0).line_height(1.5);
        let lines = sheet.frame(lines, use_context::<Rc<dyn Clipboard>>()).label("Log".to_string());
        column().grow(1.0).child(bar).child(lines.child(list)).mount(cx)
    }
}
