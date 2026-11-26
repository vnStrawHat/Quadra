//! The system's image codecs (Image I/O) as Quadra's decoder: every format macOS reads, and
//! no decoder of untrusted files of our own. Core Graphics draws a frame at the size asked
//! for.

use std::time::Duration;

use objc2_core_foundation::{
    CFData, CFDictionary, CFNumber, CFRetained, CFString, CFType, CGPoint, CGRect, CGSize, ConcreteType,
    kCFAllocatorNull,
};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGColorSpace, CGContext, CGImage, CGImageAlphaInfo, CGInterpolationQuality,
    kCGColorSpaceSRGB,
};
use objc2_image_io::{
    CGImageSource, kCGImagePropertyGIFDelayTime, kCGImagePropertyGIFDictionary,
    kCGImagePropertyGIFUnclampedDelayTime, kCGImagePropertyOrientation, kCGImagePropertyPixelHeight,
    kCGImagePropertyPixelWidth,
};
use quadra_platform::{Decoded, ImageDecoder, ImageError, ImageInfo, MAX_IMAGE_PIXELS, quarter};

/// The decoder. It holds nothing.
pub struct ImageIo;

fn corrupt(why: &str) -> ImageError {
    ImageError::Corrupt(why.to_string())
}

/// A value of a dictionary of properties by its key, if it is of the type asked for.
fn value<'a, T: ConcreteType>(of: &'a CFDictionary, key: &CFString) -> Option<&'a T> {
    // SAFETY: the key is a string, as the keys of these dictionaries are. A value is an
    // object of Core Foundation, read while the dictionary lives and only as the type it
    // says it is.
    unsafe {
        let value = of.value((key as *const CFString).cast()).cast::<CFType>();
        value.as_ref()?.downcast_ref::<T>()
    }
}

fn number(of: &CFDictionary, key: &CFString) -> Option<f64> {
    value::<CFNumber>(of, key)?.as_f64()
}

/// A file opened: its frames, and what the first says of itself. It reads the file's bytes,
/// and is dropped before who gave them returns.
struct Opened {
    source: CFRetained<CGImageSource>,
    count: usize,
    /// The size of the pixels as they are stored.
    stored: ImageInfo,
    /// The camera's orientation tag (EXIF 274), 1 when there is none.
    turn: u16,
}

fn open(file: &[u8]) -> Result<Opened, ImageError> {
    // SAFETY: the data reads `file` and frees nothing (the allocator that does nothing);
    // the source that keeps it is dropped by the caller before it returns. The keys are
    // Image I/O's constants.
    unsafe {
        let data = CFData::with_bytes_no_copy(None, file.as_ptr(), file.len() as isize, kCFAllocatorNull)
            .ok_or_else(|| corrupt("it could not be read"))?;
        let source = CGImageSource::with_data(&data, None).ok_or_else(|| corrupt("it is no image"))?;
        let properties = source
            .properties_at_index(0, None)
            .ok_or_else(|| corrupt("it is no image the system has a codec for"))?;
        let side = |key| number(&properties, key).filter(|it| *it >= 1.0).map(|it| it as u32);
        let (Some(width), Some(height)) =
            (side(kCGImagePropertyPixelWidth), side(kCGImagePropertyPixelHeight))
        else {
            return Err(corrupt("it says no size"));
        };
        let turn = number(&properties, kCGImagePropertyOrientation).map_or(1, |it| it as u16);
        Ok(Opened { count: source.count(), source, stored: ImageInfo { width, height }, turn })
    }
}

/// An image's pixels at a size: premultiplied RGBA in sRGB, the top row first.
fn pixels(image: &CGImage, width: u32, height: u32) -> Result<Vec<u8>, ImageError> {
    let (w, h) = (width as usize, height as usize);
    let mut pixels = vec![0u8; w * h * 4];
    // SAFETY: the context draws into `pixels`, which is as large as it is told and is not
    // touched while it lives: it is dropped at the end of the block.
    unsafe {
        let space =
            CGColorSpace::with_name(Some(kCGColorSpaceSRGB)).ok_or_else(|| corrupt("no colour space"))?;
        let layout = CGImageAlphaInfo::PremultipliedLast.0;
        let context = CGBitmapContextCreate(pixels.as_mut_ptr().cast(), w, h, 8, w * 4, Some(&space), layout)
            .ok_or_else(|| corrupt("no bitmap of that size"))?;
        CGContext::set_interpolation_quality(Some(&context), CGInterpolationQuality::High);
        let all =
            CGRect { origin: CGPoint { x: 0.0, y: 0.0 }, size: CGSize { width: w as f64, height: h as f64 } };
        CGContext::draw_image(Some(&context), all, Some(image));
    }
    Ok(pixels)
}

impl Opened {
    /// A frame at the size it is seen at: the stored pixels are scaled to it unturned, then
    /// turned as the tag says.
    fn frame(&self, index: usize, width: u32, height: u32) -> Result<Decoded, ImageError> {
        let (width, height) = if quarter(self.turn) { (height, width) } else { (width, height) };
        // SAFETY: a source that is alive; an index it has no frame at gives none.
        let image = unsafe { self.source.image_at_index(index, None) }
            .ok_or_else(|| corrupt("a frame could not be decoded"))?;
        Ok(Decoded { width, height, pixels: pixels(&image, width, height)? }.oriented(self.turn))
    }

    /// How long a frame of a GIF shows, none for a frame of anything else. Less than two
    /// hundredths of a second is shown for a tenth, as browsers do.
    fn time(&self, index: usize) -> Option<Duration> {
        // SAFETY: as above; the keys are Image I/O's constants.
        unsafe {
            let properties = self.source.properties_at_index(index, None)?;
            let gif = value::<CFDictionary>(&properties, kCGImagePropertyGIFDictionary)?;
            let said = number(gif, kCGImagePropertyGIFUnclampedDelayTime)
                .or_else(|| number(gif, kCGImagePropertyGIFDelayTime))
                .unwrap_or(0.0);
            let hundredths = (said * 100.0).round();
            Some(Duration::from_millis(if (2.0..=65_535.0).contains(&hundredths) {
                hundredths as u64 * 10
            } else {
                100
            }))
        }
    }
}

impl ImageDecoder for ImageIo {
    /// The size as the image is seen: a photograph turned a quarter by its tag swaps sides.
    fn info(&self, file: &[u8]) -> Result<ImageInfo, ImageError> {
        let Opened { stored, turn, .. } = open(file)?;
        Ok(match quarter(turn) {
            true => ImageInfo { width: stored.height, height: stored.width },
            false => stored,
        })
    }

    fn decode(&self, file: &[u8], width: u32, height: u32) -> Result<Decoded, ImageError> {
        open(file)?.frame(0, width, height)
    }

    // ponytail: a GIF's frames alone, as on Windows; the other formats that move (APNG,
    // WebP, HEICS) show their first frame until someone asks for more.
    fn frames(&self, file: &[u8], width: u32, height: u32) -> Result<Vec<(Decoded, Duration)>, ImageError> {
        let opened = open(file)?;
        let all = width as u64 * height as u64 * opened.count as u64;
        if opened.count < 2 || all > MAX_IMAGE_PIXELS || opened.time(0).is_none() {
            return Ok(vec![(opened.frame(0, width, height)?, Duration::ZERO)]);
        }
        // Image I/O gives each frame whole: drawn over the ones before it.
        (0..opened.count)
            .map(|at| Ok((opened.frame(at, width, height)?, opened.time(at).unwrap_or_default())))
            .collect()
    }
}
