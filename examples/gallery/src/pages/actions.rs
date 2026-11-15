//! What starts an action: the button, the icon button and the toggle button.

use quadra::{Align, Div, Justify, div, row, signal, text};
use quadra_design::{Size, Variant, button, icon_button, icons, label, token, tooltip};

use crate::common::{demo, line, page};

pub fn button_page() -> Div {
    let clicks = signal(0);
    let count = move || clicks.update(|n| *n += 1);
    let variants = [
        ("Deploy", Variant::Primary),
        ("Secondary", Variant::Secondary),
        ("Outline", Variant::Outline),
        ("Ghost", Variant::Ghost),
        ("Delete", Variant::Danger),
        ("Link", Variant::Link),
    ];
    let variants = variants.map(|(name, variant)| button(name).variant(variant).on_click(count));
    let clicked = text(move || format!("clicked {}\u{D7}", clicks.get()));
    let clicked = clicked.size(12.0).color(token::TEXT_3);
    let sizes = [("xs \u{B7} 18", Size::Xs), ("sm \u{B7} 22", Size::Sm), ("md \u{B7} 28", Size::Md)];
    let sizes = line().children(sizes.map(|(name, size)| button(name).size(size)));
    let with_icons = line()
        .child(button("Pull").icon(icons::DOWNLOAD))
        .child(button("Save").icon(icons::SAVE).variant(Variant::Primary))
        .child(button("Restart").icon(icons::REFRESH_CW).variant(Variant::Outline))
        .child(button("Logs").icon(icons::EXTERNAL_LINK).variant(Variant::Ghost));
    let states = line()
        .child(button("Disabled").disabled(true))
        .child(button("Disabled").variant(Variant::Primary).disabled(true))
        .child(button("Applying").variant(Variant::Primary).loading(true));
    let footer = row()
        .gap(8.0)
        .justify(Justify::End)
        .child(button("Cancel").variant(Variant::Ghost))
        .child(button("View diff").variant(Variant::Outline))
        .child(button("Apply").variant(Variant::Primary));
    let about =
        "Starts an action. Use one primary button per view; Enter and Space activate a focused button.";
    page(
        "Button",
        about,
        [
            demo(
                "Variants",
                line().children(variants).child(clicked),
                "button(\"Deploy\").variant(Variant::Primary).on_click(move || ..)",
            ),
            demo(
                "Sizes",
                sizes.child(button("lg \u{B7} 32").size(Size::Lg)),
                "button(\"..\").size(Size::Sm)",
            ),
            demo("With icons", with_icons, "button(\"Pull\").icon(icons::DOWNLOAD)"),
            demo("States", states, ".disabled(true)  .loading(true)"),
            demo("Dialog footer", footer, ""),
        ],
    )
}

pub fn icon_button_page() -> Div {
    let sizes = [("xs", Size::Xs), ("sm", Size::Sm), ("md", Size::Md), ("lg", Size::Lg)];
    let sizes = sizes.map(|(name, size)| tooltip(name, icon_button(icons::SETTINGS, name).size(size)));
    let small = |body, name: &'static str| tooltip(name, icon_button(body, name).size(Size::Xs));
    let actions = row().align_items(Align::Center).height(28.0).padding_x(8.0).bg(token::PANEL);
    let actions = actions.border(1.0).border_color(token::LINE).child(label("Pods")).child(div().grow(1.0));
    let actions = actions
        .child(small(icons::FILTER, "Filter"))
        .child(small(icons::REFRESH_CW, "Refresh"))
        .child(small(icons::ELLIPSIS, "More"));
    let md = |body, name: &'static str| icon_button(body, name).size(Size::Md);
    let variants = line()
        .child(tooltip("Ghost (default)", md(icons::PLUS, "Ghost")))
        .child(tooltip("Outline", md(icons::PLUS, "Outline").variant(Variant::Outline)))
        .child(tooltip("Primary", md(icons::PLUS, "Primary").variant(Variant::Primary)))
        .child(tooltip("Delete", md(icons::TRASH_2, "Delete").variant(Variant::Danger)))
        .child(tooltip("Disabled", md(icons::X, "Disabled").disabled(true)));
    let about =
        "A square button with only an icon. Always give it a tooltip; it is also the accessible name.";
    page(
        "Icon button",
        about,
        [
            demo(
                "Sizes",
                line().children(sizes),
                "tooltip(\"Settings\", icon_button(icons::SETTINGS, \"Settings\"))",
            ),
            demo("Panel actions", actions, ""),
            demo("Variants", variants, ""),
        ],
    )
}

pub fn toggle_button_page() -> Div {
    let (bold, italic, follow, wrap, kind) =
        (signal(true), signal(false), signal(true), signal(false), signal(0usize));
    let flip = |on: quadra::Signal<bool>| move || on.update(|on| *on = !*on);
    let format = line()
        .child(tooltip(
            "Bold",
            icon_button(icons::BOLD, "Bold").size(Size::Md).toggled(bold).on_click(flip(bold)),
        ))
        .child(tooltip(
            "Italic",
            icon_button(icons::ITALIC, "Italic").size(Size::Md).toggled(italic).on_click(flip(italic)),
        ));
    let small = |name: &str| button(name.to_string()).variant(Variant::Ghost).size(Size::Sm);
    let options = line()
        .child(small("Follow").icon(icons::ARROW_DOWN).toggled(follow).on_click(flip(follow)))
        .child(small("Wrap lines").toggled(wrap).on_click(flip(wrap)));
    let kinds = ["All", "Languages", "Themes", "Icon Themes", "Snippets"];
    let chip =
        |(i, name): (usize, &str)| small(name).toggled(move || kind.get() == i).on_click(move || kind.set(i));
    let chips = row().gap(4.0).children(kinds.into_iter().enumerate().map(chip).collect::<Vec<_>>());
    let about = "A button that stays pressed. The pressed state turns its label and icon the accent.";
    page(
        "Toggle button",
        about,
        [
            demo(
                "Formatting",
                format,
                "icon_button(icons::BOLD, \"Bold\").toggled(bold).on_click(move || ..)",
            ),
            demo("Log viewer options", options, ""),
            demo("Filter chips", chips, "button(\"All\").size(Size::Sm).toggled(move || kind.get() == 0)"),
        ],
    )
}
