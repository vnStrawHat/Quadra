//! The settings (specs/21, "Settings" and "About"): a window of their own, the kit's view of
//! pages; each control writes its setting, which the main window follows.

use std::path::PathBuf;
use std::rc::Rc;

use quadra::{Align, Div, Signal, column, div, effect, row, signal, text, untrack, use_context};
use quadra_design::{
    SettingsGroup, SettingsPage, Tone, Variant, button, icons, number_input, segmented, select, settings,
    switch, token,
};
use quadra_shell::{Associations, FileTypes, same_program};

use crate::model::Model;
use crate::prefs::{self, Prefs};

/// Each page: its name, its icon and what it is about.
const PAGES: [(&str, &str, &str); 5] = [
    ("Appearance", icons::PALETTE, "How the app looks."),
    ("Editor", icons::CODE, "The source, in Raw and side by side."),
    ("Preview", icons::EYE, "The document as it reads."),
    ("Layouts", icons::COLUMNS_2, "What a file opens in, and the panes side by side."),
    ("About", icons::INFO, "Which Markdown this is, and the files it opens."),
];

/// What Markdown registers for: the extensions it is the app of.
pub fn file_types(exe: PathBuf) -> FileTypes {
    FileTypes {
        name: "Markdown".into(),
        vendor: "Quadra".into(),
        description: "Reads and edits Markdown files.".into(),
        document: "Markdown document".into(),
        extensions: vec![".md".into(), ".markdown".into()],
        exe,
    }
}

/// Opens the settings in a window of their own, or brings it to the front. It closes with
/// the main window.
pub(crate) fn open_window(windows: &quadra::Windows, model: &Model) {
    let model = model.clone();
    let options = quadra::WindowOptions::new("Markdown Settings").size(980.0, 680.0);
    let options = options.custom_frame().remember("settings");
    windows.open(options, move || {
        let bar = quadra_design::titlebar("Markdown Settings").title_size(14.0);
        quadra_design::root().child(bar).child(page(&model, 0))
    });
}

/// The pages, in the kit's layout of cards, the page `first` (of Appearance, Editor, Preview,
/// Layouts, About) showing.
pub fn page(model: &Model, first: usize) -> Div {
    let (p, d, current) = (model.prefs, Prefs::defaults(), signal(first));
    let mut view = settings(current).search(signal(String::new())).cards();
    for ((name, body, about), groups) in PAGES.into_iter().zip(groups(p, d, model)) {
        let page = SettingsPage::new(name).icon(body).about(about);
        view = view.page(groups.into_iter().fold(page, SettingsPage::group));
    }
    column().grow(1.0).min_height(0.0).child(view)
}

fn groups(p: Prefs, d: Prefs, model: &Model) -> [Vec<SettingsGroup>; 5] {
    let theme = SettingsGroup::new("Theme")
        .row("Theme", "Dark or light, or as the system is.", segmented(prefs::THEMES, p.theme))
        .resets(p.theme, d.theme.get_untracked());
    let editor = SettingsGroup::new("Text")
        .row(
            "Text size",
            "The source's text, from 12 to 20 px.",
            number_input(p.text_size).stepper().range(12.0, 20.0).step(1.0).label("Text size"),
        )
        .resets(p.text_size, d.text_size.get_untracked())
        .row(
            "Wrap lines",
            "A long line goes on under itself; off, the editor scrolls sideways.",
            switch(p.wrap),
        )
        .resets(p.wrap, d.wrap.get_untracked());
    let preview = SettingsGroup::new("Page")
        .row(
            "Text width",
            "The column of the text: 720, 900 or 1200 px, or the pane's width.",
            select(prefs::WIDTHS, p.width).width(240.0),
        )
        .resets(p.width, d.width.get_untracked());
    let layouts = SettingsGroup::new("Layouts")
        .row("A file opens in", "The layout of a file opened.", select(prefs::OPENS, p.opens_in).width(240.0))
        .resets(p.opens_in, d.opens_in.get_untracked())
        .row(
            "Scroll together",
            "Side by side, the panes follow each other by their headings.",
            switch(p.together),
        )
        .resets(p.together, d.together.get_untracked());
    [vec![theme], vec![editor], vec![preview], vec![layouts], about(model)]
}

/// Which Markdown this is, and the button that makes it the default for `.md` files.
fn about(model: &Model) -> Vec<SettingsGroup> {
    let exe = std::env::current_exe().unwrap_or_default();
    let mark = row().align_items(Align::Center).gap(token::SPACE_XL).child(crate::logo::logo(64.0)).child(
        column()
            .gap(token::SPACE_XS)
            .child(div().font_weight(600).text_size(18.0).child(text("Markdown")))
            .child(div().text_color(token::TEXT_2).child(text("Reads and edits Markdown files."))),
    );
    let app = SettingsGroup::new("Markdown")
        .wide(mark)
        .row("Version", "", text(env!("CARGO_PKG_VERSION")))
        .row("Program", "", text(exe.display().to_string()))
        .row("Built with", "", text("Quadra, a native UI framework on Direct3D 11 and DirectWrite"));
    vec![app, default_app(model, exe)]
}

/// Whether Markdown opens `.md` files, and Set as default.
fn default_app(model: &Model, exe: PathBuf) -> SettingsGroup {
    let shell = use_context::<Rc<dyn Associations>>();
    let types = file_types(exe.clone());
    let (is_default, pressed): (Signal<bool>, Signal<u64>) = (signal(false), signal(0));
    let (asks, window, notices) = (shell.clone(), use_context::<quadra::Window>(), model.notices);
    // Read again whenever the window comes to the front (the user chose on the system's
    // page), and after the button.
    effect(move || {
        pressed.get();
        if let Some(window) = &window {
            window.is_active();
        }
        let now = asks.as_ref().and_then(|it| it.opener_of(".md")).is_some_and(|it| same_program(&it, &exe));
        if untrack(|| is_default.get()) != now {
            is_default.set(now);
        }
    });
    let says = move || {
        if is_default.get() { "Markdown opens .md files." } else { "Another app opens .md files, or none." }
    };
    let set = button("Set as default\u{2026}").variant(Variant::Secondary).on_click(move || {
        let Some(shell) = &shell else { return };
        match shell.register(&types) {
            Ok(()) => {
                shell.choose_default(&types);
                pressed.update(|n| *n += 1);
            }
            Err(e) => {
                notices.toast(Tone::Danger, format!("Could not register Markdown for .md files: {e}"));
            }
        }
    });
    let note = "Windows lets the user alone choose: this registers Markdown for .md and .markdown, \
                then opens Default apps, where you choose it.";
    SettingsGroup::new("Default app")
        .row(".md files", note, set)
        .wide(div().text_color(token::TEXT_2).child(text(says)))
}
