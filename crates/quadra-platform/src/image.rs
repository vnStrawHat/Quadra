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
}
