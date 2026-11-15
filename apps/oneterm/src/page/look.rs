//! The settings of how terminals look and behave (specs/19, "Settings"): the pages
//! Appearance and Terminal.

use quadra::{Signal, View, effect, row, signal, untrack};
use quadra_design::{SettingsGroup, color_input, input, number_input, segmented, select, switch, token};

use crate::prefs::{self, Prefs};

pub(super) fn appearance(p: Prefs) -> Vec<SettingsGroup> {
    let theme = SettingsGroup::new("Theme")
        .row("Theme", "System follows the OS's dark or light mode.", segmented(prefs::THEMES, p.theme))
        .row(
            "Colour theme",
            "One of 1Term's themes over the mode; the kit's follows it.",
            select(prefs::COLOUR_THEMES.clone(), p.colour_theme).width(240.0),
        )
        .row(
            "Text size",
            "All of 1Term's text, the terminals' with it, in percent.",
            number_input(p.ui_text).range(75.0, 200.0).step(5.0),
        );
    let l = p.look;
    let text = SettingsGroup::new("Terminal text")
        .row(
            "Font family",
            "A family of this machine; empty, the kit's mono face.",
            input(l.font_family).placeholder("Mono").width(240.0),
        )
        .row(
            "Fallback fonts",
            "Families tried in order, separated by commas, for what the font lacks (icons of patched fonts).",
            input(l.fallbacks).placeholder("Fallback fonts").width(240.0),
        )
        .row("Font size", "In pixels.", number_input(l.font_size).range(8.0, 32.0))
        .row("Weight", "Where the face has it.", select(prefs::WEIGHTS, l.weight).width(240.0))
        .row("Line height", "Times the size.", number_input(l.line_height).range(1.0, 2.5).step(0.1))
        .row(
            "Cell width",
            "In pixels; 0, the width of the font's 0.",
            number_input(l.cell_width).range(0.0, 48.0).step(0.5).test_id("cell-width"),
        )
        .row("Gutter", "When each line was written, and its number.", switch(l.show_gutter));
    let text = prefs::PADDING.iter().zip(l.padding).fold(text, |group, (&(key, name, _), side)| {
        let id = &key["terminal.".len()..];
        group.row(name, "Room around the grid, in pixels.", number_input(side).range(0.0, 64.0).test_id(id))
    });
    let theirs = [token::EDITOR_FOREGROUND, token::CANVAS, token::EDITOR_CURSOR, token::EDITOR_SELECTION];
    let colors = prefs::COLORS.iter().zip(l.colors).enumerate().fold(
        SettingsGroup::new("Terminal colours"),
        |group, (n, ((_, name), color))| {
            let field = color_input(color).placeholder(name);
            let field = match theirs.get(n) {
                Some(&role) => field.fallback(role),
                None => field,
            };
            group.row(name, "#rrggbb over the theme's; empty, the theme's.", field)
        },
    );
    let colors = colors.row(
        "Minimum contrast",
        "Text is lightened or darkened to stand out this much from its background; 1 is off.",
        number_input(l.min_contrast).range(1.0, 21.0).step(0.5).test_id("min-contrast"),
    );
    let ansi = (0..8).fold(SettingsGroup::new("ANSI colours"), |group, n| {
        let field =
            |at: usize| color_input(l.ansi[at]).placeholder(prefs::ANSI[at].1).fallback(token::ANSI[at]);
        group.row(
            prefs::ANSI[n].1,
            "Normal and bright, #rrggbb; empty, the theme's.",
            row().gap(8.0).child(field(n)).child(field(n + 8)),
        )
    });
    vec![theme, text, colors, ansi]
}

pub(super) fn terminal(p: Prefs) -> Vec<SettingsGroup> {
    let cursor = SettingsGroup::new("Cursor & history")
        .row("Cursor shape", "Until a program sets its own.", segmented(prefs::CURSORS, p.cursor))
        .row("Cursor blink", "Whether the cursor blinks.", switch(p.blink))
        .row(
            "Scrollback lines",
            "The history each space keeps.",
            number_input(p.scrollback).range(0.0, 100_000.0).step(1000.0),
        );
    let mouse = SettingsGroup::new("Mouse & wheel")
        .row("Context menu", "The other button opens a space's menu.", switch(p.context_menu))
        .row("Copy on select", "A selection is copied when the button is released.", switch(p.copy_on_select))
        .row("Middle-click paste", "The middle button pastes.", switch(p.middle_paste))
        .row(
            "Scroll speed",
            "Times the rows of the wheel.",
            number_input(p.scroll_speed).range(0.1, 10.0).step(0.5),
        )
        .row(
            "Alternate scroll",
            "On a full-screen program the wheel sends arrows.",
            switch(p.alternate_scroll),
        );
    let shell = SettingsGroup::new("Shell")
        .row("Default shell", "What Ctrl+T and New tab start.", shell_choice(p.shell))
        .row(
            "New spaces start in",
            "Where a new tab's shell starts.",
            select(prefs::STARTS, p.start).width(240.0),
        );
    vec![cursor, mouse, shell]
}

/// The default shell as a choice of the shells found, written as its name; the first is
/// the OS's own.
fn shell_choice(shell: Signal<String>) -> impl View {
    let names: Vec<String> = std::iter::once("The OS's".to_string())
        .chain(crate::shells::found().into_iter().map(|it| it.name.into()))
        .collect();
    let at = untrack(|| names.iter().position(|it| *it == shell.get()).unwrap_or(0));
    let chosen = signal(at);
    let words = names.clone();
    effect(move || {
        let name = if chosen.get() == 0 { String::new() } else { words[chosen.get()].clone() };
        if untrack(|| shell.get()) != name {
            shell.set(name);
        }
    });
    select(names, chosen).width(240.0)
}
