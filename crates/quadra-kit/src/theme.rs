//! Themes (specs/behaviour/15, "Themes"): a colour for each role, read from a JSON file. A
//! file names its mode and lists colours under the names of the roles; it starts from the
//! built-in theme of its mode, so a new theme lists only what differs. The built-in themes
//! are such files.

use std::fmt;
use std::sync::OnceLock;

use quadra_prim::Color;
use quadra_tree::{Cast, Theme, Token};
use serde_json::{Map, Value};

use crate::token::{self, NAMES};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Dark,
    Light,
    HighContrast,
}

/// What is wrong with a theme file, with the path of the value: `palette.canvass`,
/// `terminal.ansi[3]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeError(pub String);

impl fmt::Display for ThemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the theme is wrong: {}", self.0)
    }
}

impl std::error::Error for ThemeError {}

/// A theme of the kit: its name, its mode, and a colour for each role.
#[derive(Clone, Debug, PartialEq)]
pub struct KitTheme {
    pub name: String,
    pub mode: Mode,
    colors: Vec<Color>,
}

const KEYS: [&str; 8] = ["name", "mode", "palette", "syntax", "editor", "terminal", "series", "controls"];

/// Roles a theme may split off. One that a file does not list follows that file's own
/// colours, not the colour the built-in theme derived for itself.
const DERIVED: [Token; 14] = [
    token::ACCENT_TINT_HOVER,
    token::ERROR_TINT_HOVER,
    token::INPUT_BG,
    token::SELECTION,
    token::TAB_FG,
    token::TAB_ACTIVE_FG,
    token::TABLE_HEAD_BG,
    token::TABLE_HEAD_FG,
    token::TABLE_ROW_BORDER,
    token::LIST_EVEN,
    token::SCROLL_THUMB_HOVER,
    token::TEXT_ON_STATUS,
    token::ERROR_HOVER,
    token::CYAN,
];

/// The shadows of the levels above the window (specs/behaviour/15, "Elevation"): how far
/// down each one is, how blurred, and how black; for a menu, then a dialog, under a light
/// theme and under a dark one, where a light shadow would not show.
type Set = &'static [(f32, f32, f32)];
const SHADOWS: [[Set; 2]; 2] = [
    [
        &[(2.0, 3.0, 0.12), (1.0, 0.0, 0.06)],
        &[(2.0, 3.0, 0.12), (3.0, 6.0, 0.08), (6.0, 12.0, 0.04), (1.0, 0.0, 0.12)],
    ],
    [&[(6.0, 16.0, 0.34), (1.0, 3.0, 0.26)], &[(18.0, 44.0, 0.44), (4.0, 10.0, 0.30)]],
];

fn casts(set: &[(f32, f32, f32)]) -> Vec<Cast> {
    set.iter().map(|&(down, blur, alpha)| Cast { down, blur, color: Color::BLACK.alpha(alpha) }).collect()
}

fn wrong<T>(path: &str, what: &str) -> Result<T, ThemeError> {
    Err(ThemeError(format!("{path}: {what}")))
}

fn color(value: &Value, path: &str) -> Result<Color, ThemeError> {
    let parsed = value.as_str().and_then(Color::parse_hex);
    parsed.map_or_else(|| wrong(path, "a colour is \"#RRGGBB\" or \"#RRGGBBAA\""), Ok)
}

/// The object under a key of a theme file: empty when the key is not there.
fn section<'a>(
    file: &'a Map<String, Value>,
    key: &str,
) -> Result<Option<&'a Map<String, Value>>, ThemeError> {
    match file.get(key) {
        None => Ok(None),
        Some(Value::Object(section)) => Ok(Some(section)),
        Some(_) => wrong(key, "an object of colours"),
    }
}

impl KitTheme {
    pub fn color(&self, token: Token) -> Color {
        self.colors.get(token.0 as usize).copied().unwrap_or(Color::TRANSPARENT)
    }

    /// Gives a role another colour: an app's own accent.
    pub fn set(&mut self, token: Token, color: Color) {
        if let Some(slot) = self.colors.get_mut(token.0 as usize) {
            *slot = color;
        }
    }

    /// What a tree resolves the roles in its styles with.
    pub fn theme(&self) -> Theme {
        let deep = self.mode == Mode::Dark;
        Theme::new(self.colors.clone())
            .with_shadows(SHADOWS[deep as usize].iter().map(|set| casts(set)).collect())
    }

    fn empty() -> KitTheme {
        let colors = vec![Color::TRANSPARENT; NAMES.len()];
        KitTheme { name: String::new(), mode: Mode::Dark, colors }
    }

    /// A theme file over `self`: the colours it lists replace this theme's.
    fn merged(mut self, text: &str) -> Result<KitTheme, ThemeError> {
        let file: Value = serde_json::from_str(text).map_err(|e| ThemeError(e.to_string()))?;
        let Value::Object(file) = file else { return wrong("the file", "a theme is a JSON object") };
        if let Some(key) = file.keys().find(|key| !KEYS.contains(&key.as_str())) {
            return wrong(key, &format!("not a key of a theme ({})", KEYS.join(", ")));
        }
        self.mode = match file.get("mode").and_then(Value::as_str) {
            Some("dark") => Mode::Dark,
            Some("light") => Mode::Light,
            Some("high_contrast") => Mode::HighContrast,
            _ => return wrong("mode", "\"dark\", \"light\" or \"high_contrast\""),
        };
        if let Some(name) = file.get("name") {
            self.name = name.as_str().map_or_else(|| wrong("name", "a text"), |name| Ok(name.to_string()))?;
        }
        let palette = section(&file, "palette")?;
        let listed = |role: &Token| palette.is_some_and(|p| p.contains_key(NAMES[role.0 as usize]));
        for role in DERIVED.iter().filter(|role| !listed(role)) {
            self.set(*role, Color::TRANSPARENT);
        }
        for (key, value) in palette.into_iter().flatten() {
            let path = format!("palette.{key}");
            let Some(role) = NAMES.iter().position(|name| name == key) else {
                return wrong(&path, "not a role of the palette");
            };
            self.colors[role] = color(value, &path)?;
        }
        for (name, value) in section(&file, "syntax")?.into_iter().flatten() {
            let (path, role) = (format!("syntax.{name}"), format!("syntax_{name}"));
            let Some(role) = NAMES.iter().position(|name| *name == role) else {
                return wrong(&path, "not a colour of code");
            };
            self.colors[role] = color(value, &path)?;
        }
        // ponytail: the other roles of the terminal come with its crate; until then their
        // colours are only checked to be colours, as the editor's that it does not use.
        for key in ["editor", "terminal"] {
            for (name, value) in section(&file, key)?.into_iter().flatten() {
                let path = format!("{key}.{name}");
                match value {
                    Value::Array(list) if key == "terminal" && name == "ansi" => {
                        if list.len() != 16 {
                            return wrong(&path, "16 colours");
                        }
                        for (i, value) in list.iter().enumerate() {
                            self.colors[token::ANSI[i].0 as usize] = color(value, &format!("{path}[{i}]"))?;
                        }
                    }
                    value => {
                        let read = color(value, &path)?;
                        // The tints of a diff are roles of the kit already.
                        let own = format!("{key}_{name}");
                        let diff = |role: &&str| name.starts_with("diff_") && *role == name;
                        if let Some(role) = NAMES.iter().position(|role| *role == own || diff(role)) {
                            self.colors[role] = read;
                        }
                    }
                }
            }
        }
        match file.get("series") {
            None => {}
            Some(Value::Array(list)) => {
                // A file's series replace all of the theme's: what it lists fewer is unset.
                token::SERIES.iter().for_each(|role| self.set(*role, Color::TRANSPARENT));
                for (i, value) in list.iter().enumerate() {
                    let read = color(value, &format!("series[{i}]"))?;
                    if let Some(role) = token::SERIES.get(i) {
                        self.set(*role, read);
                    }
                }
            }
            Some(_) => return wrong("series", "a list of colours"),
        }
        Ok(self.finished())
    }

    /// Gives the roles a theme left unset the colours they follow.
    fn finished(mut self) -> KitTheme {
        use token::*;
        let unset = |theme: &KitTheme, role: Token| theme.color(role).is_transparent();
        let follows = [
            (CHROME, PANEL),
            (STATUS_BG, CHROME),
            (TAB_BG, PANEL),
            (INPUT_BG, CANVAS),
            (BORDER_FOCUSED, ACCENT),
            (TAB_FG, TEXT_2),
            (TAB_ACTIVE_FG, TEXT),
            (TABLE_HEAD_BG, PANEL),
            (TABLE_HEAD_FG, TEXT_2),
            (TABLE_ROW_BORDER, LINE_STRONG),
            (TEXT_ON_STATUS, TEXT_ON_ACCENT),
            (CYAN, SYNTAX_TYPE),
            (SELECTION, ACCENT_TINT),
        ];
        for (role, other) in follows.into_iter().chain(SERIES.map(|series| (series, ACCENT))) {
            if unset(&self, role) {
                self.set(role, self.color(other));
            }
        }
        let tints = [(OK_TINT, self.color(OK).alpha(0.1)), (WARN_TINT, self.color(WARN).alpha(0.1))];
        let tints = tints.into_iter().chain([
            (ERROR_TINT, self.color(ERROR).alpha(0.1)),
            (SCROLL_THUMB, self.color(TEXT).alpha(0.3)),
            // Whole bytes, as a file would hold it: a theme that is written out reads back equal.
            (
                ERROR_HOVER,
                Color::parse_hex(&self.color(ERROR).lerp(self.color(CANVAS), 0.15).hex()).unwrap_or_default(),
            ),
        ]);
        for (role, derived) in tints {
            if unset(&self, role) {
                self.set(role, derived);
            }
        }
        // A tint under the pointer is pulled towards its colour; after the tints are known.
        for (role, tint, color) in
            [(ACCENT_TINT_HOVER, ACCENT_TINT, ACCENT), (ERROR_TINT_HOVER, ERROR_TINT, ERROR)]
        {
            if unset(&self, role) {
                self.set(role, self.color(tint).lerp(self.color(color), 0.08));
            }
        }
        if unset(&self, SCROLL_THUMB_HOVER) {
            self.set(SCROLL_THUMB_HOVER, self.color(SCROLL_THUMB));
        }
        self
    }
}

/// Reads a theme: `{"name", "mode": "dark" | "light" | "high_contrast", "palette": {..},
/// "syntax": {..}, "editor": {..}, "terminal": {.., "ansi": [16]}, "series": [..]}`, each
/// colour `"#RRGGBB"` or `"#RRGGBBAA"`. What a file leaves out is as in [`light`] when its
/// mode is light, and as in [`dark`] otherwise.
pub fn theme_from_json(text: &str) -> Result<KitTheme, ThemeError> {
    // The mode decides what the file starts from, so it is read before the rest.
    let light_mode = serde_json::from_str::<Value>(text).ok().is_some_and(|file| file["mode"] == "light");
    if light_mode { light() } else { dark() }.merged(text)
}

fn built_in(text: &str) -> KitTheme {
    KitTheme::empty().merged(text).unwrap_or_else(|e| panic!("a built-in theme: {e}"))
}

/// One Dark Pro: the dark theme of the design language.
pub fn dark() -> KitTheme {
    static THEME: OnceLock<KitTheme> = OnceLock::new();
    THEME.get_or_init(|| built_in(include_str!("../themes/one-dark-pro.json"))).clone()
}

/// One Light: its light theme.
pub fn light() -> KitTheme {
    static THEME: OnceLock<KitTheme> = OnceLock::new();
    THEME.get_or_init(|| built_in(include_str!("../themes/one-light.json"))).clone()
}
