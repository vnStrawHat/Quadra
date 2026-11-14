//! What the user does to a diff besides reading it: rows selected and copied as a unified
//! diff writes them, F7 to the next run of changes, the wheel sideways.

use std::rc::Rc;

use quadra_base::action::{NextChange, PreviousChange};
use quadra_base::table::Sheet;
use quadra_platform::{Clipboard, Mods};
use quadra_reactive::{Memo, Signal, untrack};
use quadra_tree::Role;
use quadra_ui::Div;

use super::Sides;
use super::rows::Row;
use super::view::as_text;

pub(super) type Listed = Memo<Rc<Vec<Row>>>;

/// The rows as a table of one column: what each says is its text in a unified diff.
pub(super) fn sheet_of(sides: Signal<Rc<Sides>>, listed: Listed) -> Sheet {
    let says = move |row: usize| {
        let at = listed.with(|rows| rows.get(row).copied());
        at.map_or_else(String::new, |at| sides.with_untracked(|sides| as_text(sides, at)))
    };
    crate::rows::one_column(move || listed.with(|rows| rows.len()), says)
}

/// The first row of the next run of changes after `from`, or of the one before it.
fn change(rows: &[Row], from: Option<usize>, back: bool) -> Option<usize> {
    let changed = |i: usize| !matches!(rows[i], Row::Same { .. } | Row::Fold { .. });
    let mut starts = (0..rows.len()).filter(|&i| changed(i) && (i == 0 || !changed(i - 1)));
    match (from, back) {
        (Some(from), true) => starts.rfind(|&i| i < from),
        (None, true) => starts.next_back(),
        (from, false) => starts.find(|&i| from.is_none_or(|from| i > from)),
    }
}

/// The frame of the rows: a table named "Diff" with the keys of a table, Ctrl-C, and F7.
pub(super) fn frame(sheet: &Sheet, frame: Div, listed: Listed, clipboard: Option<Rc<dyn Clipboard>>) -> Div {
    let go = |back: bool| {
        let sheet = sheet.clone();
        move || {
            let to = untrack(|| listed.with(|rows| change(rows, sheet.cursor.get(), back)));
            if let Some(to) = to {
                sheet.pick(to, Mods::NONE);
            }
        }
    };
    let (next, previous) = (go(false), go(true));
    let frame = sheet.frame(frame, clipboard).role(Role::Table).label("Diff".to_string());
    frame.on_action::<NextChange>(move |_| next()).on_action::<PreviousChange>(move |_| previous())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keys_go_to_the_first_row_of_each_run_of_changes() {
        let same = Row::Same { old: 0, new: 0 };
        let (gone, came) = (Row::Removed { old: 1, with: None }, Row::Added { new: 1, with: None });
        let rows = [same, gone, came, same, same, came, same];
        assert_eq!(change(&rows, None, false), Some(1));
        assert_eq!(change(&rows, Some(1), false), Some(5));
        assert_eq!(change(&rows, Some(5), false), None);
        assert_eq!(change(&rows, Some(5), true), Some(1));
        assert_eq!(change(&rows, None, true), Some(5));
    }
}
