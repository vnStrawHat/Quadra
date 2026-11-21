//! The log view (specs/13, "Data"): the lines of a log in a list, the newest kept in view,
//! under a bar with a filter.

use std::cell::Cell;
use std::rc::Rc;

use quadra_design::rows::{one_column, selectable, sideways};
use quadra_design::{MenuItem, context_menu, fonts, token};
use quadra_layout::Align;
use quadra_platform::Clipboard;
use quadra_reactive::{Signal, effect, memo, signal, use_context};
use quadra_tree::{NodeId, Paint};
use quadra_view::{
    AnyView, MountCx, View, by_text, column, div, dynamic, row, text, uniform_list, virtual_list,
};

use crate::log::{Level, Log};

mod bar;
mod look;
mod options;
mod row;

use bar::{Filter, bar};
use look::Look;
use row::{Colors, as_text, line};

/// The width of a letter of the mono face, for each px of its size. It grows with the text
/// scale (`by_text`).
const LETTER: f32 = 0.6;
/// A place past the end of any list: the list keeps it at its end, which is following.
const FOLLOWING: f32 = f32::MAX;

type Menu = Rc<dyn Fn(usize) -> Vec<MenuItem>>;
type Note = Signal<Option<(usize, String)>>;

pub struct LogView {
    log: Log,
    look: Look,
    bar: bool,
    /// What the app's switches say, when they are reactive.
    stamps: Option<Box<dyn Fn() -> bool>>,
    wraps: Option<Box<dyn Fn() -> bool>>,
    /// Rows may be of different heights.
    wrapping: bool,
    follow: Option<Signal<bool>>,
    header: Option<Rc<dyn Fn() -> String>>,
    empty: Option<String>,
    menu: Option<Menu>,
    notes: Option<Note>,
    colors: Colors,
}

/// The lines of a log, with the time of each before it.
pub fn log_view(log: &Log) -> LogView {
    let look = Look {
        timestamps: signal(true),
        time: token::TEXT_3,
        offset: 0,
        size: 12.0,
        row: 18.0,
        levels: false,
        millis: true,
        ground: false,
        wrap: signal(false),
    };
    let colors: Colors = Rc::new(|source| token::HUES[source as usize % token::HUES.len()].into());
    LogView {
        log: log.clone(),
        look,
        bar: true,
        stamps: None,
        wraps: None,
        wrapping: false,
        follow: None,
        header: None,
        empty: None,
        menu: None,
        notes: None,
        colors,
    }
}

impl View for LogView {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let LogView {
            log, look, stamps, wraps, wrapping, follow, header, empty, menu, notes, colors, ..
        } = self;
        if let Some(stamps) = stamps {
            effect(move || look.timestamps.set(stamps()));
        }
        if let Some(wraps) = wraps {
            effect(move || look.wrap.set(wraps()));
        }
        let filter = Filter {
            query: signal(String::new()),
            pattern: signal(false),
            dim: signal(false),
            graver: signal(false),
        };
        let (changed, wrong) = log.signals();
        let (filtered, leveled) = (log.clone(), log.clone());
        // Whether the pattern is one is the log's to say: the bar reads it there. Without
        // the bar, the app filters.
        if self.bar {
            effect(move || {
                filtered.set_filter(&filter.query.get(), filter.pattern.get(), filter.dim.get());
            });
            effect(move || leveled.set_level(if filter.graver.get() { Level::Warn } else { Level::None }));
        }

        // Where the list is; past its end, it follows. The app's switch moves it there, or
        // to the end it is at.
        let (scroll, end, follow) =
            (signal(FOLLOWING), signal(0.0f32), follow.unwrap_or_else(|| signal(true)));
        effect(move || {
            let (on, at) = (follow.get(), scroll.get_untracked());
            if on && at != FOLLOWING {
                scroll.set(FOLLOWING);
            } else if !on && at == FOLLOWING {
                scroll.set(end.get_untracked());
            }
        });
        // Scrolled down to the end, or with nothing to scroll: it follows again.
        let before = Cell::new(FOLLOWING);
        effect(move || {
            let (at, end) = (scroll.get(), end.get());
            if at < FOLLOWING && (end == 0.0 || (at > before.get() && at >= end - 1.0)) {
                scroll.set(FOLLOWING);
            }
            before.set(at);
            follow.set(scroll.get_untracked() == FOLLOWING);
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
        effect(move || {
            if look.wrap.get() {
                sx.set(0.0);
            }
        });
        let reach = move |width: f32| match look.wrap.get_untracked() {
            true => 0.0,
            false => widest.widest() as f32 * letter() + 16.0 - (width - look.before(&widest, letter())),
        };
        let (rows, counted, picks) = (log.clone(), log.clone(), sheet.clone());
        let build = move |place: Signal<usize>| {
            let (rows, colors, menu, open) = (rows.clone(), colors.clone(), menu.clone(), signal(false));
            let noted = memo(move || {
                let mine = |n: &Option<(usize, String)>| {
                    n.as_ref().filter(|n| n.0 == place.get()).map(|n| n.1.clone())
                };
                notes.and_then(|notes| notes.with(mine))
            });
            // Built again for another place, and when other lines are listed: lines that
            // come change neither.
            let built = dynamic(move || {
                changed.get();
                line(&rows, place.get(), look, sx, noted.get(), &colors)
            });
            let face = selectable(&picks, div().min_height(by_text(look.row)), place);
            let face = face.bg(move || if open.get() { token::ACCENT.alpha(0.12) } else { Paint::default() });
            let view = built.container(sideways(face, sx, reach.clone()));
            match menu {
                Some(items) => {
                    let items = move || items(place.get_untracked());
                    AnyView::new(context_menu(view, items).while_open(open))
                }
                None => AnyView::new(view),
            }
        };
        let count = move || counted.listed();
        let list = match wrapping {
            false => uniform_list(count, look.row, scroll, build),
            true => virtual_list(count, look.row, scroll, build),
        };
        let list = list.end(end).container(div().grow(1.0));

        // A row of words in `text_3` that is no line: the header, and why none shows.
        let pad = if look.ground { 0.0 } else { 8.0 };
        let words =
            move || row().height(by_text(look.row)).shrink(0.0).align_items(Align::Center).padding_left(pad);
        let words = move || words().text_color(token::TEXT_3);
        let (all, shown) = (log.clone(), log.clone());
        let why = memo(move || match (shown.listed(), all.is_empty()) {
            (0, true) => empty.clone(),
            (0, false) => Some("No line matches what is set in the bar above".to_string()),
            _ => None,
        });
        let none = dynamic(move || match why.get() {
            Some(said) => words().child(text(said)),
            None => div(),
        });

        let lines = column().grow(1.0).font_family(fonts::MONO).text_size(look.size);
        let lines = lines.line_height(look.row / look.size);
        let lines = match look.ground {
            true => lines.bg(token::CANVAS).radius(6.0).padding_x(12.0).padding_y(6.0),
            false => lines,
        };
        let lines = match header {
            Some(said) => lines.child(words().child(text(move || said()))),
            None => lines,
        };
        let lines = sheet.frame(lines, use_context::<Rc<dyn Clipboard>>()).label("Log".to_string());
        let all = column().grow(1.0);
        let all = if self.bar { all.child(bar(&log, filter, wrong, follow)) } else { all };
        all.child(lines.child(none).child(list)).mount(cx)
    }
}
