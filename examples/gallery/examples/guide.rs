//! The code the site's guide quotes (`site/`, `tools/website.py`): each part between `// #name`
//! and `// #end` is shown on a page, and all of it is compiled by the gate. Its window is the
//! first app of "Getting started":
//!
//!     tools/cargo.sh run -p quadra-gallery --example guide

#![allow(dead_code)]

// #hello
use quadra::{WindowOptions, signal, text};
use quadra_design::{Variant, button, button_row, card, card_head, field, form, input, root, token};

fn main() -> quadra::Result<()> {
    let view = || {
        let (name, greeted) = (signal(String::from("Quadra")), signal(0));
        let hello = text(move || format!("Hello, {}! Greeted {} times.", name.get(), greeted.get()));
        let greet = button("Greet").variant(Variant::Primary).on_click(move || greeted.update(|n| *n += 1));
        let body = form().child(field("Name", input(name))).child(hello).child(button_row().child(greet));
        let card =
            card().width(320.0).child(card_head("Hello", "A signal, a field and a button")).child(body);
        root().padding(token::SPACE_XXL).child(card)
    };
    let app = quadra_design::install_system(quadra::app()?)?;
    app.window(WindowOptions::new("Hello").size(420.0, 300.0), view)?.run()
}
// #end

use quadra::{Div, Signal, View, batch, each, effect, memo, row, show};
use quadra_design::{checkbox, columns, dialog, hstack, section, sections, select, spacer, vstack};

// #counter
fn counter() -> Div {
    let count = signal(0);
    let add = button("Add one").variant(Variant::Primary).on_click(move || count.update(|n| *n += 1));
    // The closure is bound to the text: a click measures and paints this one node again.
    hstack().child(add).child(text(move || format!("Clicked {} times", count.get())))
}
// #end

fn once() -> Div {
    // #once
    let greeting = row().gap(token::SPACE_MD).child(text("Hello")).child(text("Quadra").weight(600));
    // #end
    greeting
}

fn signals() -> Div {
    // #signals
    let count = signal(0);
    let bound = text(move || format!("{} items", count.get())); // updates when count is written
    let fixed = text(format!("{} items", count.get())); // read once, now: never updates

    count.set(3);
    count.update(|n| *n += 1);

    // A value computed from signals: computed again when they change, told on when it differs.
    let many = memo(move || count.get() > 10);
    // Something outside the tree, done when what it reads changes.
    effect(move || println!("many: {}", many.get()));
    // Several writes, their readers told once.
    batch(|| {
        count.set(0);
        count.update(|n| *n += 20);
    });
    // #end
    row().child(bound).child(fixed)
}

fn states() -> Div {
    // #states
    let item = row().radius(token::RADIUS_MD).padding_x(token::SPACE_MD);
    let item = item.hover(|s| s.bg(token::PANEL_HOVER)).active(|s| s.bg(token::PRESS));
    // #end
    item
}

// #arrangements
fn filters(query: Signal<String>, kind: Signal<usize>) -> Div {
    // A row of controls, 8 apart, centred across; the button at the far end.
    let bar = hstack()
        .child(input(query).placeholder("Filter").width(200.0))
        .child(select(["All", "Running", "Failed"], kind))
        .child(spacer())
        .child(button("Refresh"));
    // Sections 24 apart; under each small heading, lines 8 apart.
    sections()
        .child(bar)
        .child(section("Today").child(text("3 runs, all passed")).child(text("Next at 02:00")))
        .child(section("This week").child(text("21 runs, 1 failed")))
}
// #end

// #dialog
fn rename(open: Signal<bool>, name: Signal<String>, keep: Signal<bool>) -> impl View {
    dialog(open, "Rename host", move || {
        form()
            .child(field("Name", input(name)).help("Shown in the list and on its tab."))
            .child(checkbox(keep).label("Keep the old name as an alias"))
    })
    .action("Cancel", Variant::Secondary, || {})
    .action("Rename", Variant::Primary, move || println!("renamed to {}", name.get_untracked()))
}
// #end

// #changes
fn hosts(all: Signal<Vec<String>>, only_prod: Signal<bool>) -> Div {
    // The count follows the list: this closure runs again, the view does not.
    let count = text(move || format!("{} hosts", all.with(|hosts| hosts.len())));
    // A row is mounted once for each name and keeps its node while the name stays.
    let rows = each(move || all.get(), |name| name.clone(), |name| text(move || name.get()));
    // A part that is there only while the condition holds.
    let note = show(move || only_prod.get(), || text("Showing production only").color(token::TEXT_2));
    vstack().child(count).child(note).child(rows)
}
// #end

fn pair(left: Div, right: Div) -> Div {
    columns(2).child(left).child(right)
}
