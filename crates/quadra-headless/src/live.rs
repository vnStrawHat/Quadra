//! A table of live data (specs/17, "The components"): rows known by a key each. The selection
//! and the row the keys are on stay with their rows when the data changes, and a cell that
//! says something else than it did under the same key is noted, with when: the look tints it
//! for `FLASH` as it fades.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use quadra_reactive::{Memo, Signal, effect, memo, on_cleanup, signal, untrack};
use quadra_tree::Handle;

use crate::table::Sheet;

/// How long the tint of a changed cell takes to fade.
pub const FLASH: Duration = Duration::from_millis(800);

/// What a row is known by.
pub type Key = Rc<dyn Fn(usize) -> String>;

/// The columns of a row that changed, and when, by the row's key.
pub type Flashes = HashMap<Rc<str>, Vec<(usize, Instant)>>;

/// What a table of live data keeps.
#[derive(Clone, Copy)]
pub struct Live {
    /// The key of every row, as the data numbers them.
    pub keys: Memo<Rc<Vec<Rc<str>>>>,
    /// The columns of a row that changed, and when, by the row's key.
    pub flashes: Signal<Rc<Flashes>>,
    /// The time of the frame, while a tint fades.
    pub tick: Signal<Instant>,
}

/// What the user chose, by key.
struct Held {
    /// The rows and their order when it was noted: other ones are other data.
    keys: Rc<Vec<Rc<str>>>,
    order: Rc<Vec<usize>>,
    chosen: Vec<Rc<str>>,
    cursor: Option<Rc<str>>,
    anchor: Option<Rc<str>>,
}

/// Makes `sheet` a table of live data, whose rows `key` says what they are known by. A change
/// of the data reads the key of every row; it reads the cells of every row too (10 000 rows
/// of 7 columns are a few milliseconds), unless the source says in `changes` which rows it
/// changed: then the cells of those are read, and of all only when rows come or go.
pub fn watch(sheet: &mut Sheet, key: Key, changes: Option<Signal<Vec<usize>>>, handle: Handle) {
    let state = &*sheet;
    let count = state.rows.clone();
    let keys = memo(move || Rc::new((0..count()).map(|row| Rc::from(key(row))).collect::<Vec<Rc<str>>>()));
    let live = Live { keys, flashes: signal(Rc::default()), tick: signal(handle.now()) };
    follow(state.clone(), keys);
    flash(state.clone(), live, changes, handle);
    sheet.live = Some(live);
}

/// Keeps the selection, the row the keys are on and the one a range starts from with their
/// keys: other data puts them where those keys are now, and what has gone is not selected.
fn follow(state: Sheet, keys: Memo<Rc<Vec<Rc<str>>>>) {
    let held: RefCell<Option<Held>> = RefCell::new(None);
    effect(move || {
        let (keys, order) = (keys.get(), state.order.get());
        let (chosen, cursor, anchor) = (state.chosen.get(), state.cursor.get(), state.anchor.get());
        let mut held = held.borrow_mut();
        let other = |held: &&mut Held| !Rc::ptr_eq(&held.keys, &keys) || !Rc::ptr_eq(&held.order, &order);
        let found = match held.as_mut().filter(other) {
            // Other rows, or another order of them: what was chosen is found again by its key.
            Some(held) => {
                let rows: HashMap<&str, usize> =
                    keys.iter().enumerate().map(|(row, key)| (&**key, row)).collect();
                let place = |key: &Rc<str>| {
                    let row = *rows.get(&**key)?;
                    if order.is_empty() { Some(row) } else { order.iter().position(|&at| at == row) }
                };
                let mut chosen: Vec<usize> =
                    held.chosen.iter().filter_map(|key| rows.get(&**key).copied()).collect();
                chosen.sort_unstable();
                let (cursor, anchor) =
                    (held.cursor.as_ref().and_then(place), held.anchor.as_ref().and_then(place));
                (held.keys, held.order) = (keys.clone(), order.clone());
                Some((chosen, cursor, anchor))
            }
            // The same rows in the same order: what the user chose is noted by its key.
            None => {
                let key_at = |place: usize| keys.get(order.get(place).copied().unwrap_or(place)).cloned();
                *held = Some(Held {
                    chosen: chosen.iter().filter_map(|&row| keys.get(row).cloned()).collect(),
                    cursor: cursor.and_then(key_at),
                    anchor: anchor.and_then(key_at),
                    order: order.clone(),
                    keys,
                });
                None
            }
        };
        drop(held);
        // Each set runs this again, which notes what is chosen now.
        if let Some((chosen, cursor, anchor)) = found {
            state.chosen.set(chosen);
            state.cursor.set(cursor);
            state.anchor.set(anchor);
        }
    });
}

/// Notes the cells that say something else than they did under the same key, and lets the
/// frames go by while their tints fade. An age is not noted: it changes all the time.
fn flash(state: Sheet, live: Live, changes: Option<Signal<Vec<usize>>>, handle: Handle) {
    let before: RefCell<Option<HashMap<Rc<str>, Vec<String>>>> = RefCell::new(None);
    let last: RefCell<Option<Rc<Vec<Rc<str>>>>> = RefCell::new(None);
    let (fading, alive): (Rc<Cell<bool>>, Rc<Cell<bool>>) = (Rc::default(), Rc::new(Cell::new(true)));
    let ended = alive.clone();
    on_cleanup(move || ended.set(false));
    effect(move || {
        let said = |row: usize| {
            let cell = |(c, column): (usize, &crate::table::Column)| match column.restless {
                true => String::new(),
                false => (state.cell)(row, c),
            };
            state.columns.iter().enumerate().map(cell).collect::<Vec<String>>()
        };
        let (keys, told) = (live.keys.get(), changes.map(|rows| rows.get()));
        // Told which rows changed, the cells are read without being watched; and only those
        // rows are, while the same rows are there.
        let same = last.replace(Some(keys.clone())).is_some_and(|was| Rc::ptr_eq(&was, &keys));
        let rows: Vec<usize> = match (told.as_ref(), same) {
            (Some(rows), true) => rows.iter().copied().filter(|&row| row < keys.len()).collect(),
            _ => (0..keys.len()).collect(),
        };
        let read = |row: usize| if told.is_some() { untrack(|| said(row)) } else { said(row) };
        let now: HashMap<Rc<str>, Vec<String>> =
            rows.iter().map(|&row| (keys[row].clone(), read(row))).collect();
        let Some(mut was) = before.replace(None) else {
            before.replace(Some(now)); // the first rows: nothing has changed yet
            return;
        };
        let (mut flashes, at) = (live.flashes.with_untracked(|all| (**all).clone()), handle.now());
        for (key, cells) in &now {
            let old = was.get(key).map_or(&[][..], Vec::as_slice);
            for (c, _) in old.iter().zip(cells).enumerate().filter(|(_, (old, new))| old != new) {
                let row = flashes.entry(key.clone()).or_default();
                row.retain(|(column, _)| *column != c);
                row.push((c, at));
            }
        }
        let partial = told.is_some() && same;
        before.replace(Some(match partial {
            true => {
                was.extend(now);
                was
            }
            false => now,
        }));
        live.flashes.set(Rc::new(flashes));
        if live.flashes.with_untracked(|all| all.is_empty()) || fading.replace(true) {
            return;
        }
        // Each frame says the time to the tints that show, and forgets those that have faded.
        let (fading, alive, clock) = (fading.clone(), alive.clone(), handle.clone());
        handle.each_frame(move || {
            if !alive.get() {
                return false;
            }
            let now = clock.now();
            let mut left = live.flashes.with_untracked(|all| (**all).clone());
            left.values_mut().for_each(|row| row.retain(|(_, since)| now.duration_since(*since) < FLASH));
            left.retain(|_, row| !row.is_empty());
            fading.set(!left.is_empty());
            live.tick.set(now);
            live.flashes.set(Rc::new(left));
            fading.get()
        });
    });
}
