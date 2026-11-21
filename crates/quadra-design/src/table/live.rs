//! A table of live data (specs/13, "Components"): the headless crate's (`quadra_headless::live`), and the
//! tints of the cells that changed, which fade.

use std::time::Instant;

use quadra_headless::table::{FLASH, Live, Sheet};
use quadra_reactive::{Memo, Signal, memo};
use quadra_view::{Region, div, show};

use crate::token;

/// The columns of the row at a place that changed, and when.
pub(crate) fn changed(live: Live, state: &Sheet, place: Signal<usize>) -> Memo<Vec<(usize, Instant)>> {
    let (rows, Live { keys, flashes, .. }) = (state.clone(), live);
    memo(move || {
        let key = keys.with(|keys| keys.get(rows.data(place.get())).cloned());
        key.and_then(|key| flashes.with(|all| all.get(&key).cloned())).unwrap_or_default()
    })
}

/// The tint of the cell of column `c`: over all of the cell, under what it says, as faint as
/// its change is old; a box only while it has changed, in a holder that takes no room.
pub(crate) fn tint(live: Live, changed: Memo<Vec<(usize, Instant)>>, c: usize) -> Region {
    let since = memo(move || changed.with(|all| all.iter().find(|(k, _)| *k == c).map(|(_, at)| *at)));
    let tick = live.tick;
    let whole = || div().absolute().left(0.0).top(0.0).right(0.0).bottom(0.0);
    let tint = move || {
        let age = move || match since.get() {
            Some(at) => tick.get().saturating_duration_since(at).as_secs_f32(),
            None => FLASH.as_secs_f32(),
        };
        whole()
            .bg(token::ACCENT.alpha(0.3))
            .opacity(move || (1.0 - age() / FLASH.as_secs_f32()).clamp(0.0, 1.0))
    };
    show(move || since.get().is_some(), tint).container(whole())
}
