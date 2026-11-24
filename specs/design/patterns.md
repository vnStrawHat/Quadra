# Patterns

Recipes: the views that are built again and again, as code. Start from the nearest one. The
code is `examples/gallery/src/pages/patterns.rs`, which the gate compiles and
`tools/shots.sh` draws (the gallery's "Patterns" pages), so a recipe here is one that runs.

None of them writes a distance: the containers and the helpers of [layout.md](layout.md) have
them. A recipe that needs `.gap(..)` or `.padding(..)` is not following its pattern.

## A dialog with a form

Fields 12 apart and as wide as the dialog; two to a row where they belong together; a section
label over what is optional; the answer last in the foot, what else can be done at its far
left.

```rust
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
```

- No field has a width: a field in a `form()` (and in a dialog's body) is as wide as its
  column. `.width(px)` on a field is for a field outside a form (a toolbar, a settings row).
- No `padding`: the dialog's body is 12 in. No `gap`: `form()` is 12 between, `columns(2)` 12.
- Every control has a `field(label, ..)` or a `.label(..)`: the label is what a screen reader
  says and what a test finds it by.
- One primary button, last. `Cancel` is secondary. A dangerous answer is `.danger()`.
- More than about eight fields is two dialogs, or a page.

## A question

```rust
pub fn delete_confirm(open: Signal<bool>) -> impl View {
    let message = "Its saved password and its place in Production go with it.";
    confirm(open, "Delete prod-api-01?", message, "Delete host", || {}).danger()
}
```

The title is the question and names the thing. The body is one sentence saying what goes with
it. The button is the verb and the noun ("Delete host"), never "OK" or "Yes".

## A popover that sets a few things

```rust
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
```

A popover has no foot and no close button: what it sets applies at once, and a press outside
closes it. It is 12 in with its lines 8 apart (the popover's own); its width is the one number
the view gives.

## A card

```rust
pub fn deploy_card() -> Div {
    let replicas = signal(3.0);
    card()
        .width(360.0)
        .child(card_head("api", "Deployment · production"))
        .child(hstack().child(text("Replicas")).child(spacer()).child(number_input(replicas).stepper()))
        .child(button_row().child(button("Restart")).child(button("Scale").variant(Variant::Primary)))
}
```

`hstack()` with a `spacer()` puts words at the left and their control at the right, centred on
one line. `button_row()` ends the card. A card never holds a card.

## Sections of settings

```rust
pub fn settings_sections() -> Div {
    let (theme, size, blink, shape) = (signal(0usize), signal(15.0), signal(true), signal(0usize));
    let look = SettingsGroup::new("Appearance")
        .row("Theme", "System follows the OS's dark or light mode.", segmented(["System", "Dark", "Light"], theme))
        .row("Font size", "In pixels.", number_input(size).stepper());
    let cursor = SettingsGroup::new("Cursor")
        .row("Shape", "Until a program sets its own.", select(["Block", "Bar", "Underline"], shape))
        .row("Blink", "The cursor blinks while the terminal has the focus.", switch(blink));
    sections().max_width(720.0).child(settings_group(look)).child(settings_group(cursor))
}
```

A setting is a row: what it sets and one line about it at the left, one control at the right.
The control keeps its own width, so the controls of a card end on one right edge. A whole
settings window is `settings(current).page(..)` (specs/13).

## Empty, and wrong

```rust
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
```

An empty place says what is not there and offers the one thing to do about it. An error stands
where the problem is, says what happened and what to try, and never only "Error".

## Controls in a row

```rust
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
```

Everything in the row is 28 tall with corners of 6 and a 1 px border, 8 apart: a toolbar or a
filter row needs nothing more than `hstack()`.

## When no pattern fits

Build it from the helpers, in this order: name the container (its inset and gap are the
kit's); list what the body holds and what each is to its neighbour (parts of one thing,
siblings, groups, sections); pick `vstack` (8), `form` (12), `section` or `sections` (24) for
each level; then go through [checklist.md](checklist.md). If the view still needs a number,
the number is a step of a ladder and is written as its token (`token::SPACE_LG`), not as a
literal.
