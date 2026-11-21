//! The log view (specs/13, "Data"): the lines of a log in a list, the newest kept in view,
//! under a bar with a filter.

use std::cell::Cell;
use std::rc::Rc;

use quadra_design::rows::{one_column, selectable, sideways};
use quadra_design::{Size, Variant, button, fonts, input, token, tooltip};
use quadra_layout::{Align, Overflow};
use quadra_platform::Clipboard;
use quadra_reactive::{Signal, effect, memo, signal, use_context};
use quadra_tree::{NodeId, Token};
use quadra_view::{
    Div, MountCx, View, by_text, column, div, dynamic, rich_text, row, span, text, uniform_list,
};

use crate::ansi::{plain, runs};
use crate::log::{Level, Log};

mod look;

use look::{Look, paint, tone};

/// The width of a letter of the mono face, for each px of its size. It grows with the text
/// scale (`by_text`).
const LETTER: f32 = 0.6;
/// A place past the end of any list: the list keeps it at its end, which is following.
const FOLLOWING: f32 = f32::MAX;

pub struct LogView {
    log: Log,
    look: Look,
    bar: bool,
}

/// The lines of a log, with the time of each before it.
pub fn log_view(log: &Log) -> LogView {
    let look = Look {
        timestamps: true,
        time: token::TEXT_3,
        offset: 0,
        size: 12.0,
        row: 18.0,
        levels: false,
        millis: true,
        ground: false,
    };
    LogView { log: log.clone(), look, bar: true }
}

impl LogView {
    /// Whether the time of each line is before it.
    pub fn timestamps(mut self, shown: bool) -> Self {
        self.look.timestamps = shown;
        self
    }

    /// Whether the bar is over the lines: an app with tools of its own filters the `Log`.
    pub fn bar(mut self, shown: bool) -> Self {
        self.bar = shown;
        self
    }

    /// The size of the text and the height of a row: 12 on 18 unless told.
    pub fn text(mut self, size: f32, row: f32) -> Self {
        (self.look.size, self.look.row) = (size, row);
        self
    }

    /// The lines on the canvas, a radius of 6, 6 by 12 px in.
    pub fn ground(mut self) -> Self {
        self.look.ground = true;
        self
    }

    /// The time to the millisecond (`09:12:41.208`, unless told), or to the second.
    pub fn millis(mut self, shown: bool) -> Self {
        self.look.millis = shown;
        self
    }

    pub fn time_color(mut self, color: Token) -> Self {
        self.look.time = color;
        self
    }

    /// The time shown at this offset from UTC, in seconds: the app's local time.
    pub fn offset(mut self, seconds: i64) -> Self {
        self.look.offset = seconds * 1000;
        self
    }

    /// Each line in its level's colour, where its own escape sequences say none.
    pub fn levels(mut self) -> Self {
        self.look.levels = true;
        self
    }
}

/// The row of the line at a place of the listing; what it says moved left by `sx`.
fn line(log: &Log, place: usize, look: Look, sx: Signal<f32>) -> Div {
    let Some((_, at_ms, raw, faint)) = log.line(place) else { return div() };
    let (fade, (said, pens)) = (if faint { 0.4 } else { 1.0 }, runs(&raw));
    let base = if look.levels { tone(log.level(place)) } else { token::TEXT };
    let mut from = 0;
    let spans = pens.into_iter().map(|(end, pen)| {
        let run = span(&said[std::mem::replace(&mut from, end)..end]).color(paint(pen, fade, base));
        if pen.bold { run.bold() } else { run }
    });
    let spans: Vec<_> = spans.collect();
    let letter = by_text(LETTER * look.size)();
    let (pad, time) = look.lead(letter);
    let line = row().grow(1.0).height(by_text(look.row)).align_items(Align::Center).padding_left(pad);
    let line = match (look.timestamps, at_ms) {
        (false, _) => line,
        (true, 0) => line.child(div().width(time).shrink(0.0)),
        (true, at) => {
            let said = text(look.when(at)).color(look.time.alpha(fade));
            line.child(div().width(time).shrink(0.0).child(said))
        }
    };
    if spans.is_empty() {
        return line;
    }
    // A long line is cut at the edge and scrolls sideways; the times stay.
    // As wide as the line at least (a byte is a cell or more of it): it never wraps.
    let wide = said.len() as f32 * letter + 8.0;
    let words = div().absolute().top(0.0).left(move || -sx.get()).width(wide).cut_to_parent();
    let words = words.child(rich_text(spans));
    line.child(div().width(0.0).grow(1.0).height(by_text(look.row)).overflow_x(Overflow::Hidden).child(words))
}

/// A line as copying gives it: its time, when they show, and what it says.
fn as_text(log: &Log, place: usize, look: Look) -> String {
    let Some((_, at_ms, raw, _)) = log.line(place) else { return String::new() };
    match (look.timestamps, at_ms) {
        (true, at) if at != 0 => format!("{} {}", look.when(at), plain(&raw)),
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
        let (log, look) = (self.log, self.look);
        let (query, pattern, dim) = (signal(String::new()), signal(false), signal(false));
        let graver = signal(false);
        let (changed, wrong) = log.signals();
        let (filtered, leveled) = (log.clone(), log.clone());
        // Whether the pattern is one is the log's to say: the bar reads it there. Without
        // the bar, the app filters.
        if self.bar {
            effect(move || {
                filtered.set_filter(&query.get(), pattern.get(), dim.get());
            });
            effect(move || leveled.set_level(if graver.get() { Level::Warn } else { Level::None }));
        }

        // Where the list is; past its end, it follows.
        let scroll = signal(FOLLOWING);
        let follows = memo(move || scroll.get() == FOLLOWING);
        let (seen, mounted): (Rc<Cell<Option<Signal<quadra_prim::Size>>>>, _) =
            (Rc::default(), signal(false));
        let (count, view, row_px) = (log.clone(), seen.clone(), by_text(look.row));
        let end = move || {
            mounted.get();
            let height = view.get().map_or(0.0, |size| size.get().height);
            (count.listed() as f32 * row_px() - height).max(0.0)
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
        let sheet = one_column(move || counted.listed(), move |place| as_text(&says, place, look));
        let chosen = sheet.chosen;
        effect(move || {
            changed.get();
            chosen.set(Vec::new());
        });
        let (sx, widest, letter) = (signal(0.0f32), log.clone(), by_text(LETTER * look.size));
        let reach = move |width: f32| {
            let letter = letter();
            let (pad, time) = look.lead(letter);
            widest.widest() as f32 * letter + 16.0 - (width - pad - time)
        };
        let (rows, counted, picks) = (log.clone(), log.clone(), sheet.clone());
        let list = uniform_list(
            move || counted.listed(),
            look.row,
            scroll,
            move |place| {
                let rows = rows.clone();
                // Built again for another place, and when other lines are listed: lines that
                // come change neither.
                let built = dynamic(move || {
                    changed.get();
                    line(&rows, place.get(), look, sx)
                });
                let face = selectable(&picks, div().height(by_text(look.row)), place);
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
        let bar = row().align_items(Align::Center).gap(4.0).padding_x(8.0).height(by_text(45.0)).shrink(0.0);
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
        let bar = bar.child(toggle(
            "Warn",
            "Only warnings and errors",
            move || graver.get(),
            move || graver.update(|on| *on = !*on),
        ));
        let bar = bar.child(toggle("Follow", "Keep the newest line in view", move || follows.get(), follow));
        let lines = column().grow(1.0).font_family(fonts::MONO).text_size(look.size);
        let lines = lines.line_height(look.row / look.size);
        let lines = match look.ground {
            true => lines.bg(token::CANVAS).radius(6.0).padding_x(12.0).padding_y(6.0),
            false => lines,
        };
        let lines = sheet.frame(lines, use_context::<Rc<dyn Clipboard>>()).label("Log".to_string());
        let all = column().grow(1.0);
        let all = if self.bar { all.child(bar) } else { all };
        all.child(lines.child(list)).mount(cx)
    }
}
