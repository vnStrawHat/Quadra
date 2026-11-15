//! The settings of how terminals look and behave (specs/19, "Settings"): the pages
//! Appearance and Terminal.

use quadra::{Signal, View, effect, signal, untrack};
use quadra_design::{SettingsGroup, input, number_input, segmented, select, switch};

use crate::prefs::{self, Prefs};

pub(super) fn appearance(p: Prefs) -> Vec<SettingsGroup> {
    let theme = SettingsGroup::new("Theme").row(
        "Theme",
        "System follows the OS's dark or light mode.",
        segmented(prefs::THEMES, p.theme),
    );
    let text = SettingsGroup::new("Terminal text")
        .row(
            "Font family",
            "A family of this machine; empty, the kit's mono face.",
            input(p.font_family).placeholder("Mono").width(240.0),
        )
        .row("Font size", "In pixels.", number_input(p.font_size).range(8.0, 32.0))
        .row("Line height", "Times the size.", number_input(p.line_height).range(1.0, 2.5).step(0.1));
    let colors = prefs::COLORS.iter().zip(p.colors).fold(
        SettingsGroup::new("Terminal colours"),
        |group, ((_, name), color)| {
            group.row(
                name,
                "#rrggbb over the theme's; empty, the theme's.",
                input(color).placeholder("Theme").width(120.0),
            )
        },
    );
    vec![theme, text, colors]
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
