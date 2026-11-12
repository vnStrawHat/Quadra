//! Accents, spaces, delimiters, and the alphabets of math (`\mathbb`, `\mathcal`): a letter
//! in an alphabet is a character of its own in Unicode.

/// The alphabet a command chooses for the letters in its argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Font {
    /// Letters slanted, as variables are set.
    Normal,
    Rm,
    Bf,
    BoldSlanted,
    Sans,
    Mono,
    Double,
    Script,
    Fraktur,
}

/// The mark an accent draws over its argument; none for a line over or under it.
pub const ACCENTS: &[(&str, &str)] = &[
    ("hat", "ˆ"),
    ("widehat", "ˆ"),
    ("check", "ˇ"),
    ("tilde", "˜"),
    ("widetilde", "˜"),
    ("acute", "´"),
    ("grave", "`"),
    ("dot", "˙"),
    ("ddot", "¨"),
    ("breve", "˘"),
    ("bar", "¯"),
    ("vec", "→"),
    ("overline", ""),
    ("underline", ""),
];

/// Spaces, in eighteenths of an em.
pub const SPACES: &[(&str, f32)] = &[
    (",", 3.0),
    (":", 4.0),
    (">", 4.0),
    (";", 5.0),
    ("!", -3.0),
    (" ", 6.0),
    ("quad", 18.0),
    ("qquad", 36.0),
    ("enspace", 9.0),
    ("thinspace", 3.0),
];

/// What `\left`, `\right` and `\big` take, by what follows them; a `.` is no delimiter.
pub const DELIMITERS: &[(&str, &str)] = &[
    ("(", "("),
    (")", ")"),
    ("[", "["),
    ("]", "]"),
    ("|", "|"),
    ("/", "/"),
    (".", ""),
    ("\\{", "{"),
    ("\\}", "}"),
    ("\\|", "‖"),
    ("\\lbrace", "{"),
    ("\\rbrace", "}"),
    ("\\langle", "⟨"),
    ("\\rangle", "⟩"),
    ("\\lvert", "|"),
    ("\\rvert", "|"),
    ("\\vert", "|"),
    ("\\lVert", "‖"),
    ("\\rVert", "‖"),
    ("\\Vert", "‖"),
    ("\\lfloor", "⌊"),
    ("\\rfloor", "⌋"),
    ("\\lceil", "⌈"),
    ("\\rceil", "⌉"),
];

pub const FONTS: &[(&str, Font)] = &[
    ("mathrm", Font::Rm),
    ("mathup", Font::Rm),
    ("mathbf", Font::Bf),
    ("mathit", Font::Normal),
    ("boldsymbol", Font::BoldSlanted),
    ("bm", Font::BoldSlanted),
    ("mathsf", Font::Sans),
    ("mathtt", Font::Mono),
    ("mathbb", Font::Double),
    ("mathcal", Font::Script),
    ("mathscr", Font::Script),
    ("mathfrak", Font::Fraktur),
];

/// Letters that are not in the block of their alphabet: they were in Unicode before it.
const HOLES: &[(Font, char, char)] = &[
    (Font::Double, 'C', 'ℂ'),
    (Font::Double, 'H', 'ℍ'),
    (Font::Double, 'N', 'ℕ'),
    (Font::Double, 'P', 'ℙ'),
    (Font::Double, 'Q', 'ℚ'),
    (Font::Double, 'R', 'ℝ'),
    (Font::Double, 'Z', 'ℤ'),
    (Font::Script, 'B', 'ℬ'),
    (Font::Script, 'E', 'ℰ'),
    (Font::Script, 'F', 'ℱ'),
    (Font::Script, 'H', 'ℋ'),
    (Font::Script, 'I', 'ℐ'),
    (Font::Script, 'L', 'ℒ'),
    (Font::Script, 'M', 'ℳ'),
    (Font::Script, 'R', 'ℛ'),
    (Font::Script, 'e', 'ℯ'),
    (Font::Script, 'g', 'ℊ'),
    (Font::Script, 'o', 'ℴ'),
    (Font::Fraktur, 'C', 'ℭ'),
    (Font::Fraktur, 'H', 'ℌ'),
    (Font::Fraktur, 'I', 'ℑ'),
    (Font::Fraktur, 'R', 'ℜ'),
    (Font::Fraktur, 'Z', 'ℨ'),
];

/// Where an alphabet starts: its capital A, its small a, its digit 0 (0: it has no digits).
fn bases(font: Font) -> Option<[u32; 3]> {
    Some(match font {
        Font::Bf => [0x1D400, 0x1D41A, 0x1D7CE],
        Font::BoldSlanted => [0x1D468, 0x1D482, 0x1D7CE],
        Font::Sans => [0x1D5A0, 0x1D5BA, 0x1D7E2],
        Font::Mono => [0x1D670, 0x1D68A, 0x1D7F6],
        Font::Double => [0x1D538, 0x1D552, 0x1D7D8],
        Font::Script => [0x1D49C, 0x1D4B6, 0],
        Font::Fraktur => [0x1D504, 0x1D51E, 0],
        Font::Normal | Font::Rm => return None,
    })
}

fn offset(base: u32, c: char, first: char) -> char {
    char::from_u32(base + (c as u32 - first as u32)).unwrap_or(c)
}

/// A letter or a digit in an alphabet; itself where the alphabet has no form of it.
pub fn styled(c: char, font: Font) -> char {
    if font == Font::Normal {
        return slanted(c);
    }
    let Some([upper, lower, digit]) = bases(font) else { return c };
    if let Some(hole) = HOLES.iter().find(|hole| hole.0 == font && hole.1 == c) {
        return hole.2;
    }
    match c {
        'A'..='Z' => offset(upper, c, 'A'),
        'a'..='z' => offset(lower, c, 'a'),
        '0'..='9' if digit != 0 => offset(digit, c, '0'),
        _ => c,
    }
}

/// The slanted form of a Latin letter or of a small Greek one.
pub fn slanted(c: char) -> char {
    match c {
        'h' => '\u{210E}', // the one letter that is not in the block
        'a'..='z' => offset(0x1D44E, c, 'a'),
        'A'..='Z' => offset(0x1D434, c, 'A'),
        '\u{3B1}'..='\u{3C9}' => offset(0x1D6FC, c, '\u{3B1}'),
        '\u{3F5}' => '\u{1D716}',
        '\u{3D1}' => '\u{1D717}',
        '\u{3D5}' => '\u{1D719}',
        '\u{3F1}' => '\u{1D71A}',
        '\u{3D6}' => '\u{1D71B}',
        _ => c,
    }
}
