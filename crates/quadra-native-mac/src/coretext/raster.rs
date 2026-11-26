//! Rasterizing a glyph (specs/08): Core Text draws it into a bitmap just large enough, as
//! coverage a pixel, or in its own colours when its font has them. macOS smooths no font by
//! subpixel, so there is no bitmap of stripes here.

use std::ptr::NonNull;

use objc2_core_foundation::CGPoint;
use objc2_core_graphics::{CGBitmapContextCreate, CGColorSpace, CGContext, CGImageAlphaInfo};
use objc2_core_text::CTFontOrientation;
use quadra_text::{FaceId, GlyphBitmap, GlyphFormat};

use super::CoreText;

impl CoreText {
    pub(super) fn raster(&mut self, face: FaceId, glyph: u16, size: f32, phase: u8) -> Option<GlyphBitmap> {
        let color = self.faces.get(face.0 as usize)?.color;
        let font = self.at(face, size)?;
        let mut glyph = glyph;
        let one = NonNull::from(&mut glyph);
        // Quarters of a pixel to the right. The flag for smoothing by pixel changes nothing:
        // that is the one way here.
        let shift = f64::from(phase & 3) / 4.0;
        // SAFETY: the ink of one glyph, read through a valid pointer; no rectangle a glyph is
        // asked for.
        let ink = unsafe {
            font.bounding_rects_for_glyphs(CTFontOrientation::Default, one, std::ptr::null_mut(), 1)
        };
        if !(ink.size.width > 0.0 && ink.size.height > 0.0) {
            return Some(GlyphBitmap::default());
        }
        // The pixels the ink may touch, from the pen on the baseline, y up; one more each
        // way for what smoothing adds.
        let left = (ink.origin.x + shift).floor() as i32 - 1;
        let bottom = ink.origin.y.floor() as i32 - 1;
        let right = (ink.origin.x + ink.size.width + shift).ceil() as i32 + 1;
        let top = (ink.origin.y + ink.size.height).ceil() as i32 + 1;
        let (width, height) = ((right - left) as usize, (top - bottom) as usize);
        let (bytes, space, layout, format) = match color {
            true => {
                (4, CGColorSpace::new_device_rgb()?, CGImageAlphaInfo::PremultipliedLast, GlyphFormat::Color)
            }
            false => (1, CGColorSpace::new_device_gray()?, CGImageAlphaInfo::None, GlyphFormat::Coverage),
        };
        let mut pixels = vec![0u8; width * height * bytes];
        // SAFETY: the context draws into `pixels`, which is as large as it is told and is
        // not touched while it lives: it is dropped at the end of the block. The glyph and
        // its place are read through valid pointers.
        unsafe {
            let data = pixels.as_mut_ptr().cast();
            let context =
                CGBitmapContextCreate(data, width, height, 8, width * bytes, Some(&space), layout.0)?;
            let it = Some(&*context);
            CGContext::set_should_antialias(it, true);
            // No smoothing by subpixel, which also draws a glyph bolder; a glyph stands
            // where its pen is, to a part of a pixel.
            CGContext::set_allows_font_smoothing(it, false);
            CGContext::set_should_smooth_fonts(it, false);
            CGContext::set_allows_font_subpixel_positioning(it, true);
            CGContext::set_should_subpixel_position_fonts(it, true);
            CGContext::set_allows_font_subpixel_quantization(it, false);
            CGContext::set_should_subpixel_quantize_fonts(it, false);
            // White on nothing: what a pixel holds is how much of it the glyph covers.
            CGContext::set_gray_fill_color(it, 1.0, 1.0);
            let mut pen = CGPoint { x: shift - f64::from(left), y: -f64::from(bottom) };
            font.draw_glyphs(one, NonNull::from(&mut pen), 1, &context);
        }
        // The bitmap's first row is its top one; `top` is counted down from the baseline.
        Some(GlyphBitmap { left, top: -top, width: width as u32, height: height as u32, format, pixels })
    }
}
