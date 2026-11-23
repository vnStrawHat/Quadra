//! Rename tab (specs/19, "Rail"): in the rail at its width a tab is renamed in a dialog, its
//! name in a field; Enter or Rename keeps it (an empty one gives the program's title back),
//! Cancel, Escape or the cross leave it as it was.

use quadra::{View, effect, signal, untrack};
use quadra_design::{Variant, input};

use crate::model::Model;

pub(crate) fn dialog(model: &Model) -> impl View + use<> {
    let (renaming, tabs) = (model.renaming(), model.tabs());
    let (open, value) = (signal(false), signal(String::new()));
    let tab = move || {
        let id = renaming.get_untracked()?;
        tabs.with_untracked(|all| all.iter().find(|it| it.id == id).copied())
    };
    effect(move || {
        if renaming.get().is_some()
            && let Some(tab) = untrack(tab)
        {
            value.set(untrack(|| tab.name.get().unwrap_or_else(|| tab.title())));
            open.set(true);
        }
    });
    // Closed however it was: no tab is being renamed.
    effect(move || {
        if !open.get() {
            untrack(|| renaming.set(None));
        }
    });
    let keep = move || {
        if let Some(tab) = tab() {
            let name = value.get_untracked();
            let name = name.trim();
            tab.name.set((!name.is_empty()).then(|| name.to_string()));
        }
    };
    let body = move || {
        input(value).width(320.0).test_id("rename-tab").on_submit(move |_: &str| {
            keep();
            open.set(false);
        })
    };
    quadra_design::dialog(open, "Rename tab", body)
        .action("Cancel", Variant::Secondary, || {})
        .action("Rename", Variant::Primary, keep)
        .shortcut("Enter")
}
