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
    /// Dividers, and the outlines of overlays and inputs.
    LINE = "line";
    /// The borders of tabs and panes, and an outline under the pointer.
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
    /// Selected text. Unset, fields select in the accent's tint.
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
}
