//! The colour picker and the file and folder pickers (specs/13, "Components"): driven through
//! the semantic tree; the dialogs are `quadra-shell`'s fake, so nothing reaches the OS.

use std::rc::Rc;

use quadra_design::{SettingsGroup, color_picker, dark, file_picker, folder_picker, settings_group, token};
use quadra_platform::{Key, Mods};
use quadra_prim::Color;
use quadra_reactive::{Signal, signal};
use quadra_shell::{Dialogs, FakeShell, FileDialog};
use quadra_testing::Harness;
use quadra_tree::{PointerInput, Role};
use quadra_view::column;

fn settle(h: &mut Harness) {
    h.frame();
    while h.frame_if_needed().is_some() {}
}

/// A picker of `value` with two presets, the default the theme's red.
fn colors(value: Signal<String>) -> Harness {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(move || {
        let red = Color::parse_hex("#e06c75").unwrap();
        let picker = color_picker(value).label("Tag").fallback(token::ANSI[1]).test_id("tag");
        let picker = picker.presets([("Red", red), ("Blue", Color::rgb(0x61afef))]);
        quadra_design::root().child(picker)
    });
    settle(&mut h);
    h
}

fn open(h: &mut Harness) {
    h.get_by_role(Role::Button, "Tag").click();
    settle(h);
}

#[test]
fn the_picker_shows_its_colour_and_a_preset_chooses_one() {
    let value = signal("#98c379".to_string());
    let mut h = colors(value);
    assert!(h.semantics().contains("Button \"Tag\" = \"#98c379\""), "{}", h.semantics());
    assert!(!h.has_text("Hex"));
    open(&mut h);
    // Opening writes nothing; the focus is in the area, which says where the colour is.
    assert_eq!(value.get_untracked(), "#98c379");
    let plane = h.get_by_role(Role::Slider, "Saturation and brightness").node();
    assert_eq!(h.tree.focused(), Some(plane));
    assert!(h.semantics().contains("saturation 38 %, brightness 76 %"), "{}", h.semantics());
    h.get_by_role(Role::Button, "Blue").click();
    settle(&mut h);
    assert_eq!(value.get_untracked(), "#61afef");
    assert_eq!(h.tree.overlays().count(), 0);
}

#[test]
fn the_area_and_the_hue_write_the_colour_and_keep_the_hue_through_black() {
    let value = signal("#ff0000".to_string());
    let mut h = colors(value);
    open(&mut h);
    h.get_by_role(Role::Slider, "Hue").set_value("120");
    settle(&mut h);
    assert_eq!(value.get_untracked(), "#00ff00");
    // A press in the middle of the area is half saturated, half bright; a drag from it past
    // the bottom is black, and the hue stays, and so does its bar.
    let plane = h.get_by_role(Role::Slider, "Saturation and brightness").node();
    let middle = h.tree.window_rect(plane).center();
    h.click(middle.x, middle.y);
    settle(&mut h);
    assert_eq!(value.get_untracked(), "#408040");
    h.pointer(PointerInput::Down, middle.x, middle.y);
    h.pointer(PointerInput::Move, middle.x, middle.y + 200.0);
    h.pointer(PointerInput::Up, middle.x, middle.y + 200.0);
    settle(&mut h);
    assert_eq!(value.get_untracked(), "#000000");
    assert!(h.semantics().contains("Slider \"Hue\" = \"120\""), "{}", h.semantics());
    // Up from black by the arrows: the green comes back.
    h.key(Key::Up, Mods::NONE);
    settle(&mut h);
    assert_eq!(value.get_untracked(), "#010301");
}

#[test]
fn the_hex_moves_the_area_and_empty_is_the_default() {
    let value = signal(String::new());
    let mut h = colors(value);
    assert!(h.semantics().contains("Button \"Tag\""));
    open(&mut h);
    assert_eq!(value.get_untracked(), "");
    // Typed, a colour moves the hue; the default's swatch empties the value again.
    h.get_by_role(Role::TextInput, "Hex").type_text("#336699");
    settle(&mut h);
    assert_eq!(value.get_untracked(), "#336699");
    assert!(h.semantics().contains("Slider \"Hue\" = \"210\""), "{}", h.semantics());
    h.get_by_role(Role::Button, "Default").click();
    settle(&mut h);
    assert_eq!((value.get_untracked(), h.tree.overlays().count()), (String::new(), 0));
}

/// A file picker and a folder picker whose dialogs are `shell`'s.
fn paths(shell: Rc<FakeShell>, file: Signal<String>, folder: Signal<String>) -> Harness {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        let (files, folders) = (shell.clone(), shell);
        let key = file_picker(file, move |_| files.open_files(&FileDialog::new("Identity file")).pop());
        let logs = folder_picker(folder, move |at| folders.pick_folder(&FileDialog::new("Logs").folder(at)));
        let key = key.label("Identity file").width(240.0);
        // In a row of settings, whose control is as wide as it is.
        let logs = settings_group(SettingsGroup::new("").row("Logs", "", logs.label("Logs")));
        quadra_design::root().child(column().gap(8.0).child(key).child(logs))
    });
    settle(&mut h);
    h
}

#[test]
fn browse_asks_the_dialog_and_a_choice_is_the_path() {
    let (shell, file, folder) = (FakeShell::new(), signal(String::new()), signal("C:\\logs".to_string()));
    let mut h = paths(shell.clone(), file, folder);
    shell.choose(&["C:\\keys\\id_ed25519"]);
    h.get_by_role(Role::Button, "Browse for a file").click();
    settle(&mut h);
    assert_eq!(file.get_untracked(), "C:\\keys\\id_ed25519");
    // Given up, the dialog leaves the path as it was.
    h.get_by_role(Role::Button, "Browse for a folder").click();
    settle(&mut h);
    assert_eq!(folder.get_untracked(), "C:\\logs");
    assert_eq!(shell.asked(), ["open_files: Identity file", "pick_folder: Logs"]);
    // The path can be typed too, in a field of its own width in a row of settings.
    let field = h.get_by_role(Role::TextInput, "Logs").node();
    assert!(h.tree.window_rect(field).width > 200.0, "{:?}", h.tree.window_rect(field));
    h.get_by_role(Role::TextInput, "Logs").set_value("D:\\logs");
    settle(&mut h);
    assert_eq!(folder.get_untracked(), "D:\\logs");
}

#[test]
fn an_empty_picker_shows_the_code_of_the_colour_that_stands_instead() {
    let (value, theme) = (signal(String::new()), Rc::new(dark().theme()));
    let red = theme.color(token::ANSI[1]);
    let mut h = Harness::new();
    h.tree.set_theme(theme.clone());
    h.mount(move || {
        // What an app gives its views: the window's theme.
        quadra_reactive::provide(signal(theme));
        quadra_design::root().child(color_picker(value).label("Tag").fallback(token::ANSI[1]))
    });
    settle(&mut h);
    let code = format!(
        "#{:02x}{:02x}{:02x}",
        (red.r * 255.0).round() as u8,
        (red.g * 255.0).round() as u8,
        (red.b * 255.0).round() as u8
    );
    assert!(h.has_text(&code), "{code}");
    value.set("#336699".into());
    settle(&mut h);
    assert!(h.has_text("#336699") && !h.has_text(&code));
}

#[test]
fn a_preset_is_said_by_its_name_and_empty_by_the_placeholder_told() {
    let value = signal(String::new());
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        quadra_reactive::provide(signal(Rc::new(dark().theme())));
        let picker = color_picker(value).label("Tag").fallback(token::ANSI[1]).placeholder("By group");
        quadra_design::root().child(picker.presets([("Blue", Color::rgb(0x61afef))]))
    });
    settle(&mut h);
    assert!(h.has_text("By group"), "{}", h.semantics());
    // A preset's colour, however it is written; a reader is still told the colour.
    value.set("#61AFEF".into());
    settle(&mut h);
    assert!(h.semantics().contains("Text \"Blue\""), "{}", h.semantics());
    assert!(h.semantics().contains("Button \"Tag\" = \"#61AFEF\""), "{}", h.semantics());
    value.set("#336699".into());
    settle(&mut h);
    assert!(h.has_text("#336699"));
}
