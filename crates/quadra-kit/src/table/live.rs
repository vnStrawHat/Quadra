//! A table of live data (specs/13, "Components"): the base's (`quadra_base::live`), and the
//! tints of the cells that changed, which fade.

use std::time::Instant;

use quadra_base::table::{FLASH, Live, Sheet};
use quadra_layout::Length;
use quadra_reactive::{Signal, memo};
use quadra_ui::{Region, div, dynamic};

use crate::token;

/// The tints of the row at a place: a box over each cell that changed, under what the cell
/// says, as faint as its change is old.
pub(crate) fn tints(live: Live, state: &Sheet, place: Signal<usize>) -> Region {
    let (rows, widths, Live { keys, flashes, tick }) = (state.clone(), state.widths, live);
    let changed = memo(move || {
        let key = keys.with(|keys| keys.get(rows.data(place.get())).cloned());
        key.and_then(|key| flashes.with(|all| all.get(&key).cloned())).unwrap_or_default()
    });
    let whole = || div().absolute().left(0.0).top(0.0).right(0.0).bottom(0.0);
    let marks = dynamic(move || {
        let mark = move |(c, since): (usize, Instant)| {
            let age = move || tick.get().saturating_duration_since(since).as_secs_f32();
            let mark = div().absolute().top(0.0).bottom(0.0).bg(token::ACCENT.alpha(0.3));
            let mark = mark.left(move || Length::Px(widths.with(|all| all[..c].iter().sum())));
            let mark = mark.width(move || Length::Px(widths.with(|all| all[c])));
            mark.opacity(move || (1.0 - age() / FLASH.as_secs_f32()).clamp(0.0, 1.0))
        };
        whole().children(changed.get().into_iter().map(mark))
    });
    marks.container(whole())
}
