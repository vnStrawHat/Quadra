//! The kit's themes (specs/behaviour/15, "Themes"): the two built-in ones give every role a
//! colour, a file lists only what differs from the theme of its mode, roles a file leaves
//! unset follow its own colours, and what is wrong in a file is named by its path.

use quadra_kit::{Mode, dark, light, theme_from_json, token};
use quadra_prim::Color;
use quadra_tree::{Paint, Token};

#[test]
fn the_built_in_themes_give_every_role_its_colour() {
    let (dark, light) = (dark(), light());
    assert_eq!((dark.name.as_str(), dark.mode), ("One Dark Pro", Mode::Dark));
    assert_eq!((light.name.as_str(), light.mode), ("One Light", Mode::Light));
    assert_eq!(
        (dark.color(token::CANVAS), dark.color(token::TEXT)),
        (Color::rgb(0x23272E), Color::rgb(0xDCE0E5))
    );
    assert_eq!(
        (light.color(token::CANVAS), light.color(token::TEXT)),
        (Color::rgb(0xFAFAFA), Color::rgb(0x242529))
    );
    // Every role has a colour, but for the three that mean "none" when they have none.
    let none = [token::SCRIM, token::SELECTION, token::LIST_EVEN];
    for theme in [&dark, &light] {
        for (number, name) in token::NAMES.iter().enumerate() {
            let role = Token(number as u16);
            assert!(none.contains(&role) || !theme.color(role).is_transparent(), "{}: {name}", theme.name);
        }
    }
    // Roles the files leave unset follow other roles.
    assert_eq!(dark.color(token::TAB_FG), dark.color(token::TEXT_2));
    assert_eq!(dark.color(token::INPUT_BG), dark.color(token::CANVAS));
    assert_eq!(dark.color(token::CYAN), Color::rgb(0xE5C07B)); // the colour of types in code
    assert_eq!(dark.color(token::ERROR_HOVER), Color::rgb(0xAA3C37)); // the error, 15 % towards the canvas
    assert_eq!(light.color(token::TABLE_ROW_BORDER), light.color(token::LINE_STRONG));
    // What a tree styles with: the same colours, by the roles' numbers.
    let colours = dark.theme();
    assert_eq!(colours.resolve(Paint::from(token::ACCENT)), Color::rgb(0x74ADE8));
    assert_eq!(colours.resolve(token::ACCENT.alpha(0.5)), Color::rgb(0x74ADE8).alpha(0.5));
}

#[test]
fn a_theme_file_lists_what_differs_from_the_theme_of_its_mode() {
    let night = r##"{
        "name": "Night",
        "mode": "dark",
        "palette": { "canvas": "#16181D", "accent": "#7AA2F7", "text_2": "#99AABB80" },
        "syntax": { "keyword": "#BB9AF7" },
        "series": ["#7AA2F7"]
    }"##;
    let (night, dark) = (theme_from_json(night).unwrap(), dark());
    assert_eq!((night.name.as_str(), night.mode), ("Night", Mode::Dark));
    assert_eq!(night.color(token::CANVAS), Color::rgb(0x16181D));
    assert_eq!(night.color(token::TEXT_2), Color::rgba(0x99AABB80));
    assert_eq!(night.color(token::TEXT), dark.color(token::TEXT)); // not listed: as in the dark theme
    // Roles it does not list follow its own colours, not those the dark theme derived.
    assert_eq!(night.color(token::INPUT_BG), Color::rgb(0x16181D));
    assert_eq!(night.color(token::TAB_FG), Color::rgba(0x99AABB80));
    assert_eq!(night.color(token::CYAN), dark.color(token::CYAN)); // it gave types no colour of its own

    let day = theme_from_json(r##"{ "mode": "light", "palette": { "input_bg": "#FFFFFF" } }"##).unwrap();
    assert_eq!((day.name.as_str(), day.color(token::TEXT)), ("One Light", light().color(token::TEXT)));
    assert_eq!(day.color(token::INPUT_BG), Color::WHITE); // a role that is listed is kept
}

#[test]
fn what_is_wrong_in_a_file_is_named_by_its_path() {
    let error = |text: &str| theme_from_json(text).unwrap_err().0;
    assert!(error("[]").starts_with("the file: a theme is a JSON object"));
    assert!(error("{").contains("EOF"));
    assert!(
        error(r#"{ "mode": "dark", "colours": {} }"#)
            .starts_with("colours: not a key of a theme (name, mode,")
    );
    assert_eq!(error(r#"{ "mode": "dim" }"#), "mode: \"dark\", \"light\" or \"high_contrast\"");
    assert_eq!(error(r#"{ "mode": "dark", "name": 3 }"#), "name: a text");
    assert_eq!(error(r#"{ "mode": "dark", "palette": [] }"#), "palette: an object of colours");
    assert_eq!(
        error(r##"{ "mode": "dark", "palette": { "canvass": "#000000" } }"##),
        "palette.canvass: not a role of the palette"
    );
    let colour = "a colour is \"#RRGGBB\" or \"#RRGGBBAA\"";
    assert_eq!(
        error(r##"{ "mode": "dark", "palette": { "canvas": "#12345" } }"##),
        format!("palette.canvas: {colour}")
    );
    assert_eq!(error(r#"{ "mode": "dark", "editor": { "cursor": 7 } }"#), format!("editor.cursor: {colour}"));
    assert_eq!(
        error(r##"{ "mode": "dark", "terminal": { "ansi": ["#000000"] } }"##),
        "terminal.ansi: 16 colours"
    );
    let ansi: Vec<&str> = (0..16).map(|i| if i == 3 { "\"red\"" } else { "\"#000000\"" }).collect();
    let file = format!(r#"{{ "mode": "dark", "terminal": {{ "ansi": [{}] }} }}"#, ansi.join(","));
    assert_eq!(error(&file), format!("terminal.ansi[3]: {colour}"));
    assert_eq!(
        error(r##"{ "mode": "dark", "series": ["#000000", "blue"] }"##),
        format!("series[1]: {colour}")
    );
}
