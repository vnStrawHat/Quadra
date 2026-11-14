//! The frames of a GIF made whole: each is drawn where its header says over what the frames
//! before it left, as their disposal says, and scaled to the size shown.

use std::time::Duration;

use quadra_platform::{Decoded, ImageError, ImageInfo, MAX_IMAGE_PIXELS};
use windows::Win32::Graphics::Imaging::{GUID_WICPixelFormat32bppPRGBA, IWICBitmapDecoder, IWICBitmapSource};
use windows::core::{Interface, w};

use super::{Opened, converted, corrupt, number, pixels, scaled, size};

/// The size of a GIF's screen, which its frames are drawn on.
pub(super) fn screen(decoder: &IWICBitmapDecoder) -> Option<ImageInfo> {
    // SAFETY: a plain COM call on an object of this thread.
    let reader = unsafe { decoder.GetMetadataQueryReader() }.ok()?;
    let width = number(&reader, w!("/logscrdesc/Width"))?;
    let height = number(&reader, w!("/logscrdesc/Height"))?;
    (width > 0 && height > 0).then_some(ImageInfo { width, height })
}

/// At most `most` frames of a GIF, whole, at `width` by `height`; none when it says no
/// screen, or more frames than an image may take (then only the first shows).
pub(super) fn gif(
    opened: &Opened,
    width: u32,
    height: u32,
    most: u32,
) -> Result<Option<Vec<(Decoded, Duration)>>, ImageError> {
    let Some(screen) = screen(&opened.decoder) else { return Ok(None) };
    // SAFETY: a plain COM call on an object of this thread.
    let count = unsafe { opened.decoder.GetFrameCount() }.map_err(corrupt)?.min(most);
    let (cw, ch) = (screen.width as usize, screen.height as usize);
    let each = (cw * ch) as u64 + width as u64 * height as u64;
    if count > 1 && each * count as u64 > MAX_IMAGE_PIXELS {
        return Ok(None);
    }
    let mut canvas = vec![0u8; cw * ch * 4];
    let mut frames = Vec::with_capacity(count as usize);
    for index in 0..count {
        // SAFETY: plain COM calls on objects of this thread.
        let frame = unsafe { opened.decoder.GetFrame(index) }.map_err(corrupt)?;
        let meta = unsafe { frame.GetMetadataQueryReader() }.map_err(corrupt)?;
        let said = |name| number(&meta, name).unwrap_or(0) as usize;
        let (left, top, disposal) =
            (said(w!("/imgdesc/Left")), said(w!("/imgdesc/Top")), said(w!("/grctlext/Disposal")));
        // In hundredths of a second; less than two is shown for a tenth, as browsers do.
        let delay = match said(w!("/grctlext/Delay")) {
            d if d < 2 => 10,
            d => d,
        };
        let own = size(&frame)?;
        let source: IWICBitmapSource = frame.cast().map_err(corrupt)?;
        let source = converted(&opened.factory, &source, &GUID_WICPixelFormat32bppPRGBA)?;
        let drawn = pixels(&source, own.width, own.height)?;
        let before = (disposal == 3).then(|| canvas.clone());
        let rect = paste(&mut canvas, (cw, ch), &drawn, own.width as usize, (left, top));
        frames
            .push((shown(opened, &canvas, screen, width, height)?, Duration::from_millis(delay as u64 * 10)));
        match (disposal, before) {
            (3, Some(before)) => canvas = before,
            (2, _) => clear(&mut canvas, cw, rect),
            _ => {}
        }
    }
    Ok(Some(frames))
}

/// Draws a frame's pixels at `at` on the canvas, where they are not transparent (a GIF's
/// pixels are wholly one or the other), and gives the part of the canvas it covers.
fn paste(
    canvas: &mut [u8],
    (cw, ch): (usize, usize),
    frame: &[u8],
    fw: usize,
    (left, top): (usize, usize),
) -> [usize; 4] {
    let fh = frame.len() / (fw * 4).max(1);
    let (x1, y1) = ((left + fw).min(cw), (top + fh).min(ch));
    let (x0, y0) = (left.min(x1), top.min(y1));
    for y in y0..y1 {
        for x in x0..x1 {
            let from = ((y - top) * fw + x - left) * 4;
            if frame[from + 3] > 0 {
                let to = (y * cw + x) * 4;
                canvas[to..to + 4].copy_from_slice(&frame[from..from + 4]);
            }
        }
    }
    [x0, y0, x1, y1]
}

/// Makes a part of the canvas transparent again.
fn clear(canvas: &mut [u8], cw: usize, [x0, y0, x1, y1]: [usize; 4]) {
    for y in y0..y1 {
        canvas[(y * cw + x0) * 4..(y * cw + x1) * 4].fill(0);
    }
}

/// The canvas at the size shown.
fn shown(
    opened: &Opened,
    canvas: &[u8],
    screen: ImageInfo,
    width: u32,
    height: u32,
) -> Result<Decoded, ImageError> {
    if (screen.width, screen.height) == (width, height) {
        return Ok(Decoded { width, height, pixels: canvas.to_vec() });
    }
    let format = &GUID_WICPixelFormat32bppPRGBA;
    // SAFETY: the bitmap copies `canvas`, rows of `screen.width * 4` bytes.
    let bitmap = unsafe {
        opened.factory.CreateBitmapFromMemory(screen.width, screen.height, format, screen.width * 4, canvas)
    }
    .map_err(corrupt)?;
    let source: IWICBitmapSource = bitmap.cast().map_err(corrupt)?;
    let source = converted(&opened.factory, &scaled(&opened.factory, &source, width, height)?, format)?;
    Ok(Decoded { width, height, pixels: pixels(&source, width, height)? })
}
