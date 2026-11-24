//! The patterns of the design language (specs/design/patterns.md), built with its helpers
//! alone: no view here writes a distance of its own. The spec quotes these functions.

use quadra::{AnyView, Div, Signal, View, div, signal, text};
use quadra_design::{
    SettingsGroup, Tone, Variant, alert, button, button_row, card, card_head, checkbox, columns, confirm,
    dialog, empty_state, field, form, hstack, icons, input, number_input, popover_button, section, sections,
    segmented, select, settings_group, spacer, switch, token, vstack,
};

use crate::common::{demo, hint, page};

/// A dialog with a form: fields 12 apart and as wide as the dialog, two to a row where they
/// belong together, a section label over what is optional, the answer last in the foot.
pub fn host_dialog(open: Signal<bool>) -> impl View {
    let (name, address, user) = (signal(String::new()), signal(String::new()), signal(String::new()));
    let (port, auth, agent) = (signal(22.0), signal(0usize), signal(true));
    dialog(open, "Add host", move || {
        form()
            .child(field("Name", input(name).placeholder("The address when empty")))
            .child(
                columns(2)
                    .child(field("Address", input(address).placeholder("Host name or IP")).required())
                    .child(field("Port", number_input(port).range(1.0, 65535.0))),
            )
            .child(
                columns(2)
                    .child(field("Username", input(user)).help("Asked when connecting if empty."))
                    .child(field("Auth method", select(["Auto", "Password", "Key file"], auth))),
            )
            .child(section("Options").child(checkbox(agent).label("Forward the SSH agent")))
    })
    .aside("Test connection", || {})
    .action("Cancel", Variant::Secondary, || {})
    .action("Add host", Variant::Primary, || {})
    .shortcut("Enter")
}

/// A question: what will happen as the title, one sentence, Cancel and the verb.
pub fn delete_confirm(open: Signal<bool>) -> impl View {
    let message = "Its saved password and its place in Production go with it.";
    confirm(open, "Delete prod-api-01?", message, "Delete host", || {}).danger()
}

/// A popover that sets a few things at once: a title, controls, no foot.
pub fn view_popover() -> impl View {
    let (density, wrap, numbers) = (signal(0usize), signal(true), signal(false));
    popover_button("View", move || {
        vstack()
            .width(256.0)
            .child(div().font_weight(600).child(text("View options")))
            .child(field("Density", segmented(["Compact", "Comfortable"], density)))
            .child(checkbox(wrap).label("Wrap long lines"))
            .child(checkbox(numbers).label("Show line numbers"))
    })
}

/// A card: a title with a quiet action, lines of one group, a row of buttons.
pub fn deploy_card() -> Div {
    let replicas = signal(3.0);
    card()
        .width(360.0)
        .child(card_head("api", "Deployment · production"))
        .child(hstack().child(text("Replicas")).child(spacer()).child(number_input(replicas).stepper()))
        .child(button_row().child(button("Restart")).child(button("Scale").variant(Variant::Primary)))
}

/// Settings: sections 24 apart, each a label over a card of rows; text left, control right.
pub fn settings_sections() -> Div {
    let (theme, size, blink, shape) = (signal(0usize), signal(15.0), signal(true), signal(0usize));
    let look = SettingsGroup::new("Appearance")
        .row(
            "Theme",
            "System follows the OS's dark or light mode.",
            segmented(["System", "Dark", "Light"], theme),
        )
        .row("Font size", "In pixels.", number_input(size).stepper());
    let cursor = SettingsGroup::new("Cursor")
        .row("Shape", "Until a program sets its own.", select(["Block", "Bar", "Underline"], shape))
        .row("Blink", "The cursor blinks while the terminal has the focus.", switch(blink));
    sections().max_width(720.0).child(settings_group(look)).child(settings_group(cursor))
}

/// Nothing here, and something wrong: each says so in place, with one thing to do.
pub fn states() -> Div {
    let none = empty_state(
        icons::SERVER,
        "No hosts yet",
        "Hosts you save show here.",
        button("Add host").variant(Variant::Primary),
    );
    let wrong = alert(Tone::Danger, "Could not connect", "prod-api-01 refused the key. Check the key file.");
    let frame = |content: AnyView| {
        div().border(1.0).border_color(token::LINE).radius(token::RADIUS_MD).child(content)
    };
    columns(2)
        .child(frame(AnyView::new(none)))
        .child(vstack().child(wrong).child(hint("An alert stands where the problem is.")))
}

/// Every field beside the others: one height, one border, one corner.
pub fn family() -> Div {
    let (words, n, pick, seg) = (signal(String::from("api")), signal(3.0), signal(0usize), signal(0usize));
    hstack()
        .wrap(true)
        .child(input(words).width(120.0))
        .child(select(["Production", "Staging"], pick))
        .child(number_input(n).stepper())
        .child(segmented(["List", "Board"], seg))
        .child(button("Cancel").variant(Variant::Secondary))
        .child(button("Save").variant(Variant::Primary))
}

pub fn form_dialog_page() -> Div {
    let (form_open, ask_open) = (signal(false), signal(false));
    let open_form = button("Add host…").variant(Variant::Primary).test_id("pt-form");
    let open_ask = button("Delete host…").variant(Variant::Danger).test_id("pt-ask");
    let about = "A dialog's body is a form: fields fill it, 12 apart, two to a row where they belong \
                 together. The answer is last in the foot; what else can be done is at its far left.";
    page(
        "Dialog with a form",
        about,
        [
            demo(
                "A form",
                hstack().child(open_form.on_click(move || form_open.set(true))).child(host_dialog(form_open)),
                "dialog(open, \"Add host\", || form().child(field(..)).child(columns(2).child(..).child(..)))",
            ),
            demo(
                "A question",
                hstack().child(open_ask.on_click(move || ask_open.set(true))).child(delete_confirm(ask_open)),
                "confirm(open, \"Delete prod-api-01?\", message, \"Delete host\", on_yes).danger()",
            ),
        ],
    )
}

pub fn settings_rows_page() -> Div {
    let about = "Sections 24 apart, each a title over a card of rows: what a row sets and a line about it at the left, its control at the right, centred.";
    page(
        "Settings rows",
        about,
        [demo(
            "Sections",
            settings_sections(),
            "sections().child(settings_group(..)).child(settings_group(..))",
        )],
    )
}

pub fn composition_page() -> Div {
    let about = "Views built from the helpers alone: vstack, hstack, form, columns, section, sections, \
                 button_row, card. Parts of a thing are 2 to 8 apart, things 12, sections 24.";
    page(
        "Composition",
        about,
        [
            demo("One family", family(), "hstack().child(input(..)).child(select(..)).child(button(..))"),
            demo(
                "A card",
                hstack().child(deploy_card()).child(view_popover()),
                "card().child(card_head(..)).child(hstack()..).child(button_row()..)",
            ),
            demo(
                "Empty, and wrong",
                states(),
                "empty_state(icon, title, hint, action); alert(tone, title, more)",
            ),
        ],
    )
}
