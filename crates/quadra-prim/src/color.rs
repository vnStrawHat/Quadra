//! Colour: straight (not premultiplied) sRGB with alpha, components in 0..=1.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
    pub const BLACK: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const WHITE: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };

    /// An opaque colour from `0xRRGGBB`.
    pub const fn rgb(hex: u32) -> Color {
        Color::rgba((hex << 8) | 0xFF)
    }

    /// A colour from `0xRRGGBBAA`.
    pub const fn rgba(hex: u32) -> Color {
        Color {
            r: ((hex >> 24) & 0xFF) as f32 / 255.0,
            g: ((hex >> 16) & 0xFF) as f32 / 255.0,
            b: ((hex >> 8) & 0xFF) as f32 / 255.0,
            a: (hex & 0xFF) as f32 / 255.0,
        }
    }

    /// From a hue in degrees and saturation, lightness and alpha in 0..=1.
    pub fn hsla(h: f32, s: f32, l: f32, a: f32) -> Color {
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let hh = h.rem_euclid(360.0) / 60.0;
        let x = c * (1.0 - (hh.rem_euclid(2.0) - 1.0).abs());
        let (r, g, b) = match hh as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = l - c / 2.0;
        Color { r: r + m, g: g + m, b: b + m, a }
    }

    /// Reads `#RRGGBB` or `#RRGGBBAA`.
    pub fn parse_hex(s: &str) -> Option<Color> {
        let digits = s.strip_prefix('#')?;
        // from_str_radix would take a sign; a colour has none.
        if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let n = u32::from_str_radix(digits, 16).ok()?;
        match digits.len() {
            6 => Some(Color::rgb(n)),
            8 => Some(Color::rgba(n)),
            _ => None,
        }
    }

    /// The same colour with another alpha.
    pub fn alpha(self, a: f32) -> Color {
        Color { a, ..self }
    }

    pub fn is_transparent(self) -> bool {
        self.a <= 0.0
    }

    pub fn lerp(self, to: Color, t: f32) -> Color {
        let mix = |a: f32, b: f32| a + (b - a) * t;
        Color { r: mix(self.r, to.r), g: mix(self.g, to.g), b: mix(self.b, to.b), a: mix(self.a, to.a) }
    }

    /// The colour `t` of the way to `to`, mixed in Oklab with alpha premultiplied, as design
    /// tools mix the stops of a gradient: lightness, hue and alpha go evenly.
    pub fn mix_oklab(self, to: Color, t: f32) -> Color {
        let (from, to) = (self.oklab(), to.oklab());
        let a = self.a + (to[3] - self.a) * t;
        if a <= 0.0 {
            return Color::TRANSPARENT;
        }
        let mix = |i: usize| (from[i] * self.a + (to[i] * to[3] - from[i] * self.a) * t) / a;
        Color::from_oklab([mix(0), mix(1), mix(2)], a)
    }

    /// Lightness, the two axes of hue, and alpha: the published matrices, digits and all.
    #[allow(clippy::excessive_precision)]
    fn oklab(self) -> [f32; 4] {
        let lin = |c: f32| if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) };
        let (r, g, b) = (lin(self.r), lin(self.g), lin(self.b));
        let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
        let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
        let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
        [
            0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
            1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
            0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
            self.a,
        ]
    }

    #[allow(clippy::excessive_precision)]
    fn from_oklab([ok_l, ok_a, ok_b]: [f32; 3], a: f32) -> Color {
        let l = (ok_l + 0.3963377774 * ok_a + 0.2158037573 * ok_b).powi(3);
        let m = (ok_l - 0.1055613458 * ok_a - 0.0638541728 * ok_b).powi(3);
        let s = (ok_l - 0.0894841775 * ok_a - 1.2914855480 * ok_b).powi(3);
        let enc = |c: f32| {
            let c = c.clamp(0.0, 1.0);
            if c <= 0.0031308 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 }
        };
        Color {
            r: enc(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s),
            g: enc(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s),
            b: enc(-0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s),
            a,
        }
    }

    /// `[r, g, b, a]`, as GPU instance data wants it.
    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// `#rrggbbaa`.
    pub fn hex(self) -> String {
        let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        format!("#{:02x}{:02x}{:02x}{:02x}", byte(self.r), byte(self.g), byte(self.b), byte(self.a))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips() {
        assert_eq!(Color::rgb(0x2563EB).hex(), "#2563ebff");
        assert_eq!(Color::rgba(0x11223380).hex(), "#11223380");
        assert_eq!(Color::parse_hex("#2563EB"), Some(Color::rgb(0x2563EB)));
        assert_eq!(Color::parse_hex("#11223380"), Some(Color::rgba(0x11223380)));
    }

    #[test]
    fn parse_hex_rejects_what_is_not_a_colour() {
        for bad in ["", "#", "2563EB", "#2563E", "#2563EBF", "#+2563E", "#2563EG", "#2563EBFFF", "#é563E"] {
            assert_eq!(Color::parse_hex(bad), None, "{bad}");
        }
    }

    #[test]
    fn hsla_and_lerp() {
        assert_eq!(Color::hsla(0.0, 1.0, 0.5, 1.0).hex(), "#ff0000ff");
        assert_eq!(Color::hsla(120.0, 1.0, 0.5, 1.0).hex(), "#00ff00ff");
        assert_eq!(Color::hsla(-120.0, 1.0, 0.5, 1.0).hex(), "#0000ffff");
        assert_eq!(Color::BLACK.lerp(Color::WHITE, 0.5).hex(), "#808080ff");
        assert!(Color::WHITE.alpha(0.0).is_transparent() && !Color::BLACK.is_transparent());
    }

    #[test]
    fn oklab_mixes_keep_the_ends_and_go_through_a_violet() {
        let (red, blue) = (Color::rgb(0xFF0000), Color::rgb(0x0000FF));
        assert_eq!((red.mix_oklab(blue, 0.0).hex(), red.mix_oklab(blue, 1.0).hex()), (red.hex(), blue.hex()));
        // Not the dark #800080 of encoded values: lighter, and bluer.
        assert_eq!(red.mix_oklab(blue, 0.5).hex(), "#8c53a2ff");
        assert_eq!(Color::BLACK.mix_oklab(Color::WHITE, 0.5).hex(), "#636363ff");
        // A transparent end has no colour of its own: the other's shows as it fades.
        assert_eq!(red.mix_oklab(Color::TRANSPARENT, 0.5).hex(), "#ff000080");
    }
}
