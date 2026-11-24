//! The faces bundled with the kit (specs/design, "Type"): IBM Plex Sans for the UI and
//! Lilex for code, both under the SIL Open Font License (`fonts/OFL-plex-sans.txt`,
//! `fonts/OFL-lilex.txt`).

/// The family of UI text.
pub const UI: &str = "IBM Plex Sans";
/// The family of code, and of values shown inside the UI (paths, numbers).
pub const MONO: &str = "Lilex";

/// The font files: regular, italic, semibold and semibold italic of the UI face, then
/// regular, italic, bold and bold italic of the mono face.
pub fn files() -> [&'static [u8]; 8] {
    [
        include_bytes!("../fonts/IBMPlexSans-Regular.ttf"),
        include_bytes!("../fonts/IBMPlexSans-Italic.ttf"),
        include_bytes!("../fonts/IBMPlexSans-SemiBold.ttf"),
        include_bytes!("../fonts/IBMPlexSans-SemiBoldItalic.ttf"),
        include_bytes!("../fonts/Lilex-Regular.ttf"),
        include_bytes!("../fonts/Lilex-Italic.ttf"),
        include_bytes!("../fonts/Lilex-Bold.ttf"),
        include_bytes!("../fonts/Lilex-BoldItalic.ttf"),
    ]
}
