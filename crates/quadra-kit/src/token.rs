//! The roles of the design language (specs/behaviour/15, "Tokens"): what a style names in
//! place of a colour, and a theme file gives a colour to under the same name in lower case.

use quadra_tree::Token;

/// Declares the roles, numbered in the order they are written, and their names in a theme
/// file in that order.
macro_rules! roles {
    ($($(#[$doc:meta])* $name:ident = $key:literal;)*) => {
        #[allow(non_camel_case_types, clippy::upper_case_acronyms)]
        #[repr(u16)]
        enum Number { $($name),* }

        $($(#[$doc])* pub const $name: Token = Token(Number::$name as u16);)*

        /// The name of each role in a theme file, in the order of their numbers.
        pub const NAMES: &[&str] = &[$($key),*];
    };
}

roles! {
    /// The deepest plane: the editor, the terminal, the active tab.
    CANVAS = "canvas";
    /// Side panels and surfaces.
    PANEL = "panel";
    /// A ghost control or a row under the pointer.
    PANEL_HOVER = "panel_hover";
    /// The fill of filled controls.
    RAISED = "raised";
    RAISED_HOVER = "raised_hover";
    /// Pressed, selected, and highlighted by the keyboard.
    PRESS = "press";
    /// Menus, tooltips, popovers, dialogs, toasts.
    OVERLAY_BG = "overlay_bg";
    /// Behind a modal dialog.
    SCRIM = "scrim";
    /// Dividers, hairlines, the grid of tables.
    LINE = "line";
    /// The outlines of controls, overlays and cards; the seams of tabs and panes.
    LINE_STRONG = "line_strong";
    /// The border of what has the focus. Unset, it is the accent.
    BORDER_FOCUSED = "border_focused";
    /// The title bar, and the window behind empty panes. Unset, it is the panel.
    CHROME = "chrome";
    /// The status bar. Unset, it is the chrome.
    STATUS_BG = "status_bg";
    /// The tab bar and inactive tabs. Unset, it is the panel.
    TAB_BG = "tab_bg";
    /// Scroll thumbs. Unset, it is the text at 30 %.
    SCROLL_THUMB = "scroll_thumb";
    /// Text fields, steppers and dropdown triggers. Unset, it is the canvas.
    INPUT_BG = "input_bg";
    /// Selected text, in fields and in text that can be selected. Unset, the accent's tint.
    SELECTION = "selection";
    /// The labels of inactive tabs. Unset, the muted text.
    TAB_FG = "tab_fg";
    /// The label of the active tab, and of one under the pointer. Unset, the text.
    TAB_ACTIVE_FG = "tab_active_fg";
    /// Table headers. Unset, the panel.
    TABLE_HEAD_BG = "table_head_bg";
    /// Their labels. Unset, the muted text.
    TABLE_HEAD_FG = "table_head_fg";
    /// The lines between the rows of a table. Unset, the strong line.
    TABLE_ROW_BORDER = "table_row_border";
    /// Every other row of a striped list. Unset, there are no stripes.
    LIST_EVEN = "list_even";
    /// A scroll thumb under the pointer or dragged. Unset, the thumb's own.
    SCROLL_THUMB_HOVER = "scroll_thumb_hover";
    /// Text and icons.
    TEXT = "text";
    /// Muted: inactive tabs, secondary labels, shortcuts.
    TEXT_2 = "text_2";
    /// Placeholders, hints, separators.
    TEXT_3 = "text_3";
    TEXT_DISABLED = "text_disabled";
    /// On an accent fill.
    TEXT_ON_ACCENT = "text_on_accent";
    /// On the fills of feedback colours. Unset, as on the accent.
    TEXT_ON_STATUS = "text_on_status";
    /// The caret of the editor, the switch, the progress fill, the dirty dot.
    ACCENT = "accent";
    ACCENT_HOVER = "accent_hover";
    ACCENT_PRESS = "accent_press";
    /// The fill of a tinted button.
    ACCENT_TINT = "accent_tint";
    /// That fill under the pointer and while pressed. Unset, the tint a little towards the accent.
    ACCENT_TINT_HOVER = "accent_tint_hover";
    /// Selected or toggled labels, links, matches.
    ACCENT_TEXT = "accent_text";
    OK = "ok";
    WARN = "warn";
    ERROR = "error";
    INFO = "info";
    /// The tints of banners and tinted buttons. Unset, each is its colour at 10 %.
    OK_TINT = "ok_tint";
    WARN_TINT = "warn_tint";
    ERROR_TINT = "error_tint";
    /// The error's tint under the pointer. Unset, the tint a little towards the error.
    ERROR_TINT_HOVER = "error_tint_hover";
    /// A filled error control under the pointer. Unset, the error a little towards the canvas.
    ERROR_HOVER = "error_hover";
    /// A cyan for kinds and marks. Unset, the colour of types in code.
    CYAN = "cyan";
    /// The states of a workload: each also has a shape.
    RUNNING = "running";
    PENDING = "pending";
    FAILED = "failed";
    SUCCEEDED = "succeeded";
    TERMINATING = "terminating";
    UNKNOWN = "unknown";
    /// Hues that tell kinds apart (specs/behaviour/15, principle 9): the state of a file,
    /// kinds of files and objects, counts of diagnostics. Never alone: a shape, letter or
    /// label goes with each.
    HUE_RED = "hue_red";
    HUE_ORANGE = "hue_orange";
    HUE_YELLOW = "hue_yellow";
    HUE_GREEN = "hue_green";
    HUE_CYAN = "hue_cyan";
    HUE_BLUE = "hue_blue";
    HUE_PURPLE = "hue_purple";
    /// The colours of code: a theme file's `syntax`, under these names without `syntax_`.
    SYNTAX_KEYWORD = "syntax_keyword";
    SYNTAX_CONTROL = "syntax_control";
    SYNTAX_STRING = "syntax_str";
    SYNTAX_NUMBER = "syntax_number";
    SYNTAX_CONSTANT = "syntax_constant";
    SYNTAX_TYPE = "syntax_typ";
    SYNTAX_FUNCTION = "syntax_function";
    SYNTAX_VARIABLE = "syntax_variable";
    SYNTAX_PROPERTY = "syntax_property";
    SYNTAX_OPERATOR = "syntax_operator";
    SYNTAX_PUNCTUATION = "syntax_punctuation";
    SYNTAX_COMMENT = "syntax_comment";
    SYNTAX_TAG = "syntax_tag";
    SYNTAX_ATTRIBUTE = "syntax_attribute";
    SYNTAX_REGEXP = "syntax_regexp";
    SYNTAX_ESCAPE = "syntax_escape";
    SYNTAX_INVALID = "syntax_invalid";
    /// The sixteen colours text asks for by number (a log, a terminal): a theme file's
    /// `terminal.ansi`, black, red, green, yellow, blue, magenta, cyan, white, then the
    /// bright ones.
    ANSI_0 = "ansi_0";
    ANSI_1 = "ansi_1";
    ANSI_2 = "ansi_2";
    ANSI_3 = "ansi_3";
    ANSI_4 = "ansi_4";
    ANSI_5 = "ansi_5";
    ANSI_6 = "ansi_6";
    ANSI_7 = "ansi_7";
    ANSI_8 = "ansi_8";
    ANSI_9 = "ansi_9";
    ANSI_10 = "ansi_10";
    ANSI_11 = "ansi_11";
    ANSI_12 = "ansi_12";
    ANSI_13 = "ansi_13";
    ANSI_14 = "ansi_14";
    ANSI_15 = "ansi_15";
    /// The tints of a line that was added and of one that was removed: a theme file's
    /// `editor.diff_add` and `editor.diff_remove`.
    DIFF_ADD = "diff_add";
    DIFF_REMOVE = "diff_remove";
    /// The code editor's: a theme file's `editor`, each `editor.<name>` the role
    /// `editor_<name>` (its text, where no kind of token colours it, is a variable's).
    EDITOR_CURSOR = "editor_cursor";
    /// The band of a line that has a cursor.
    EDITOR_LINE = "editor_line_highlight";
    /// The numbers of the lines, and those of the lines that have a cursor.
    EDITOR_GUTTER = "editor_gutter";
    EDITOR_GUTTER_ACTIVE = "editor_gutter_active";
    EDITOR_SELECTION = "editor_selection";
    /// What finding found, and the match that is chosen.
    EDITOR_FIND = "editor_find_match";
    EDITOR_FIND_CURRENT = "editor_find_current";
    /// Behind the bracket at a cursor and the one that matches it.
    EDITOR_BRACKET = "editor_bracket_match";
    /// The colours of the series of a chart: a theme file's `series`. Unset, the accent.
    SERIES_1 = "series_1";
    SERIES_2 = "series_2";
    SERIES_3 = "series_3";
    SERIES_4 = "series_4";
    SERIES_5 = "series_5";
    SERIES_6 = "series_6";
    SERIES_7 = "series_7";
    SERIES_8 = "series_8";
}

/// The sixteen colours text asks for by number, in order.
pub const ANSI: [Token; 16] = [
    ANSI_0, ANSI_1, ANSI_2, ANSI_3, ANSI_4, ANSI_5, ANSI_6, ANSI_7, ANSI_8, ANSI_9, ANSI_10, ANSI_11,
    ANSI_12, ANSI_13, ANSI_14, ANSI_15,
];

/// The hues by kind: red, orange, yellow, green, cyan, blue, purple.
pub const HUES: [Token; 7] = [HUE_RED, HUE_ORANGE, HUE_YELLOW, HUE_GREEN, HUE_CYAN, HUE_BLUE, HUE_PURPLE];

/// The colours of the series of a chart, in order.
pub const SERIES: [Token; 8] =
    [SERIES_1, SERIES_2, SERIES_3, SERIES_4, SERIES_5, SERIES_6, SERIES_7, SERIES_8];
