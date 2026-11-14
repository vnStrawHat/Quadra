//! Image decoding (specs/10, "Services"; specs/11, "Images"): the system's codecs behind a
//! trait, and the limits a file is held to from its header, before any pixel is decoded.

use std::fmt;

/// The most pixels a side, the most pixels in all (64 MiB of RGBA), the largest file, and
/// the most pixels a byte of file may claim (no format packs more: a header that lies makes
/// a decoder allocate first).
pub const MAX_IMAGE_SIDE: u32 = 16_384;
pub const MAX_IMAGE_PIXELS: u64 = 16 << 20;
pub const MAX_IMAGE_FILE_BYTES: usize = 256 << 20;
pub const MAX_PIXELS_PER_BYTE: u64 = 16_384;

/// What a file's header says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
}

/// Decoded pixels: premultiplied RGBA, four bytes a pixel, a row after another.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decoded {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Decoded {
    /// The pixels as a camera's orientation tag (EXIF 274) says they are seen: 1 as they
    /// are, 2 to 4 mirrored or half turned, 5 to 8 turned a quarter (6 clockwise, 8 the
    /// other way; 5 and 7 mirrored too), which swaps the sides. Anything else is 1.
    pub fn oriented(self, orientation: u16) -> Decoded {
        if !(2..=8).contains(&orientation) {
            return self;
        }
        let (w, h) = (self.width as usize, self.height as usize);
        let (to_w, to_h) = if quarter(orientation) { (h, w) } else { (w, h) };
        let mut pixels = vec![0u8; self.pixels.len()];
        for y in 0..h {
            for x in 0..w {
                let (tx, ty) = match orientation {
                    2 => (w - 1 - x, y),
                    3 => (w - 1 - x, h - 1 - y),
                    4 => (x, h - 1 - y),
                    5 => (y, x),
                    6 => (h - 1 - y, x),
                    7 => (h - 1 - y, w - 1 - x),
                    _ => (y, w - 1 - x),
                };
                let (from, to) = ((y * w + x) * 4, (ty * to_w + tx) * 4);
                pixels[to..to + 4].copy_from_slice(&self.pixels[from..from + 4]);
            }
        }
        Decoded { width: to_w as u32, height: to_h as u32, pixels }
    }
}

/// Whether an orientation tag turns the image a quarter, so that its sides swap.
pub fn quarter(orientation: u16) -> bool {
    (5..=8).contains(&orientation)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageError {
    /// The file could not be read.
    Read(String),
    /// It is no image the decoder knows, or a broken one.
    Corrupt(String),
    /// Its header asks for more than an image may be.
    TooLarge(String),
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageError::Read(why) => write!(f, "the image could not be read: {why}"),
            ImageError::Corrupt(why) => write!(f, "the image could not be decoded: {why}"),
            ImageError::TooLarge(why) => write!(f, "the image is too large: {why}"),
        }
    }
}

impl std::error::Error for ImageError {}

/// A decoder of image files. It is called on the threads of the background pool.
pub trait ImageDecoder: Send + Sync {
    /// What the header says, with no pixel decoded.
    fn info(&self, file: &[u8]) -> Result<ImageInfo, ImageError>;
    /// The pixels, scaled to `width` by `height`.
    fn decode(&self, file: &[u8], width: u32, height: u32) -> Result<Decoded, ImageError>;
}

/// Holds a file to the limits, by what its header says and how long it is.
pub fn admit(info: ImageInfo, file_bytes: usize) -> Result<(), ImageError> {
    let pixels = info.width as u64 * info.height as u64;
    let too = |what: String| Err(ImageError::TooLarge(what));
    if info.width == 0 || info.height == 0 {
        return Err(ImageError::Corrupt("it has no pixels".into()));
    }
    if file_bytes > MAX_IMAGE_FILE_BYTES {
        return too(format!("a file of {file_bytes} bytes"));
    }
    if info.width > MAX_IMAGE_SIDE || info.height > MAX_IMAGE_SIDE {
        return too(format!("{} by {} pixels", info.width, info.height));
    }
    if pixels > MAX_IMAGE_PIXELS || pixels > file_bytes as u64 * MAX_PIXELS_PER_BYTE {
        return too(format!("{pixels} pixels from {file_bytes} bytes"));
    }
    Ok(())
}

/// The decoder of the headless platform: an image is one line of text, `flat W H RRGGBBAA`,
/// and decodes to that colour at whatever size is asked for.
#[derive(Clone, Copy, Debug, Default)]
pub struct FlatImages;

impl FlatImages {
    /// The file of a flat image, as long as a file of that many pixels is at least.
    pub fn file(width: u32, height: u32, rgba: u32) -> Vec<u8> {
        let mut file = format!("flat {width} {height} {rgba:08x}").into_bytes();
        let least = (width as u64 * height as u64 / MAX_PIXELS_PER_BYTE) as usize + 1;
        file.resize(file.len().max(least), b' ');
        file
    }

    fn read(file: &[u8]) -> Option<(ImageInfo, u32)> {
        let text = std::str::from_utf8(file).ok()?;
        let mut words = text.split_ascii_whitespace();
        if words.next()? != "flat" {
            return None;
        }
        let (width, height) = (words.next()?.parse().ok()?, words.next()?.parse().ok()?);
        Some((ImageInfo { width, height }, u32::from_str_radix(words.next()?, 16).ok()?))
    }
}

impl ImageDecoder for FlatImages {
    fn info(&self, file: &[u8]) -> Result<ImageInfo, ImageError> {
        Self::read(file).map(|(info, _)| info).ok_or_else(|| ImageError::Corrupt("not a flat image".into()))
    }

    fn decode(&self, file: &[u8], width: u32, height: u32) -> Result<Decoded, ImageError> {
        let (_, rgba) = Self::read(file).ok_or_else(|| ImageError::Corrupt("not a flat image".into()))?;
        let [r, g, b, a] = rgba.to_be_bytes();
        let pre = |c: u8| ((c as u32 * a as u32 + 127) / 255) as u8;
        let pixel = [pre(r), pre(g), pre(b), a];
        Ok(Decoded { width, height, pixels: pixel.repeat(width as usize * height as usize) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_header_that_asks_too_much_is_refused_before_a_pixel() {
        let ok = ImageInfo { width: 4000, height: 3000 };
        assert_eq!(admit(ok, 2 << 20), Ok(()));
        // A side, the pixels in all, and more pixels than its bytes could hold.
        assert!(matches!(
            admit(ImageInfo { width: 20_000, height: 1 }, 1 << 20),
            Err(ImageError::TooLarge(_))
        ));
        assert!(matches!(
            admit(ImageInfo { width: 8192, height: 8192 }, 1 << 20),
            Err(ImageError::TooLarge(_))
        ));
        assert!(matches!(admit(ok, 100), Err(ImageError::TooLarge(_))));
        assert!(matches!(admit(ImageInfo { width: 0, height: 9 }, 100), Err(ImageError::Corrupt(_))));

        let file = FlatImages::file(3, 2, 0xff00_0080);
        assert_eq!(FlatImages.info(&file), Ok(ImageInfo { width: 3, height: 2 }));
        let half = FlatImages.decode(&file, 2, 1).unwrap();
        assert_eq!((half.width, half.height, half.pixels), (2, 1, vec![128, 0, 0, 128, 128, 0, 0, 128]));
        assert!(FlatImages.info(b"GIF89a").is_err());
    }

    #[test]
    fn an_orientation_tag_turns_and_mirrors_the_pixels() {
        // Two by one: a red pixel, then a blue one.
        let (r, b) = ([255, 0, 0, 255], [0, 0, 255, 255]);
        let wide = Decoded { width: 2, height: 1, pixels: [r, b].concat() };
        let seen = |o: u16| {
            let d = wide.clone().oriented(o);
            (d.width, d.height, d.pixels)
        };
        assert_eq!(seen(1), (2, 1, [r, b].concat()));
        assert_eq!(seen(2), (2, 1, [b, r].concat()));
        assert_eq!(seen(3), (2, 1, [b, r].concat()));
        assert_eq!(seen(4), (2, 1, [r, b].concat()));
        // A quarter clockwise puts the left at the top; the other way, at the bottom.
        assert_eq!(seen(6), (1, 2, [r, b].concat()));
        assert_eq!(seen(8), (1, 2, [b, r].concat()));
        assert_eq!(seen(5), (1, 2, [r, b].concat()));
        assert_eq!(seen(7), (1, 2, [b, r].concat()));
        assert_eq!(seen(0), seen(1));
        // On two rows, a quarter clockwise moves the bottom-left corner to the top-left.
        let (g, w) = ([0, 255, 0, 255], [255; 4]);
        let square = Decoded { width: 2, height: 2, pixels: [r, b, g, w].concat() };
        assert_eq!(square.oriented(6).pixels, [g, r, w, b].concat());
    }
}
