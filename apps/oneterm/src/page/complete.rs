//! The settings of command completion (specs/19, "Completion"), a group of the Terminal page.

use quadra_design::{SettingsGroup, number_input, select, switch};

use crate::complete::{CompletePrefs, FAMILIES};
use crate::model::Model;

pub(super) fn completion(model: &Model) -> SettingsGroup {
    let c: CompletePrefs = model.ui().complete.prefs;
    SettingsGroup::new("Completion")
        .row("Completion", "Suggestions under the cursor as a command is typed.", switch(c.enabled))
        .row(
            "Accept with Tab",
            "Tab chooses a suggestion, then takes it; off, Tab goes to the shell.",
            switch(c.accept_tab),
        )
        .row(
            "History kept",
            "Lines run this session, kept for each kind of shell; 0 keeps none.",
            number_input(c.max_history).range(0.0, 5000.0).step(50.0).label("History kept"),
        )
        .row(
            "Characters before suggesting",
            "Of a command, before its names are suggested.",
            number_input(c.min_prefix).range(0.0, 10.0).label("Characters before suggesting"),
        )
        .row(
            "Rows shown",
            "Before the list scrolls.",
            number_input(c.max_visible).range(1.0, 20.0).label("Rows shown"),
        )
        .row("From the history", "The commands run this session.", switch(c.memory))
        .row("From hand-written catalogs", "Commands and their options written for 1Term.", switch(c.manual))
        .row("From generated catalogs", "Commands read from the tools' own help.", switch(c.external))
        .row("Fuzzy matching", "Also what has the typed letters in order.", switch(c.fuzzy))
        .row(
            "Options of the commands above",
            "A subcommand also offers its command's options.",
            switch(c.ancestors),
        )
        .row(
            "Off in full-screen programs",
            "Nothing on the alternate screen: vim, less, htop.",
            switch(c.off_in_alt_screen),
        )
        .row(
            "Only at a prompt",
            "Where the shell marks a command being typed, or a prompt shows.",
            switch(c.only_at_prompt),
        )
        .row("Unix commands on Windows", "Cmd and PowerShell also offer coreutils.", switch(c.coreutils))
        .row(
            "Shell family",
            "The kind of shell completed for; detected from the shell.",
            select(FAMILIES, c.family).width(240.0),
        )
        .row(
            "Redact secrets",
            "Passwords and tokens are taken out of a line before it is kept.",
            switch(c.redact),
        )
}
