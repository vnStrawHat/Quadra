//! Rasterization: a glyph becomes coverage, per pixel or per subpixel, or a colour image.

use std::mem::ManuallyDrop;

use quadra_text::{FaceId, GRAYSCALE, GlyphBitmap, GlyphFormat};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_GLYPH_OFFSET, DWRITE_GLYPH_RUN, DWRITE_GRID_FIT_MODE_DEFAULT, DWRITE_MEASURING_MODE_NATURAL,
    DWRITE_RENDERING_MODE_NATURAL_SYMMETRIC, DWRITE_TEXT_ANTIALIAS_MODE,
    DWRITE_TEXT_ANTIALIAS_MODE_CLEARTYPE, DWRITE_TEXT_ANTIALIAS_MODE_GRAYSCALE, DWRITE_TEXTURE_ALIASED_1x1,
    DWRITE_TEXTURE_CLEARTYPE_3x1, DWRITE_TEXTURE_TYPE,
};

use super::DirectWrite;

/// Coverage of one colour layer of a colour glyph.
struct ColorLayer {
    coverage: GlyphBitmap,
    rgba: [f32; 4],
}

impl DirectWrite {
    /// `phase` quarters of a pixel move the glyph right before it is sampled; `GRAYSCALE` in
    /// it smooths by pixel whatever the system does.
    pub(super) fn raster(&mut self, face: FaceId, glyph: u16, size: f32, phase: u8) -> Option<GlyphBitmap> {
        let face = self.faces.get(face.0 as usize)?;
        let (gray, phase) = (phase & GRAYSCALE != 0, phase & !GRAYSCALE);
        let (offset, advance, x) = (DWRITE_GLYPH_OFFSET::default(), 0.0f32, phase as f32 / 4.0);
        let mut run = DWRITE_GLYPH_RUN {
            fontFace: ManuallyDrop::new(Some(face.face.clone())),
            fontEmSize: size,
            glyphCount: 1,
            glyphIndices: &glyph,
            glyphAdvances: &advance,
            glyphOffsets: &offset,
            ..Default::default()
        };
        let colored = if face.color { self.raster_color(&run, x) } else { None };
        let bitmap = colored.or_else(|| {
            if self.cleartype && !gray {
                let per_stripe = self.coverage(
                    &run,
                    x,
                    0.0,
                    DWRITE_TEXT_ANTIALIAS_MODE_CLEARTYPE,
                    DWRITE_TEXTURE_CLEARTYPE_3x1,
                )?;
                Some(self.subpixel(per_stripe))
            } else {
                self.coverage(&run, x, 0.0, DWRITE_TEXT_ANTIALIAS_MODE_GRAYSCALE, DWRITE_TEXTURE_ALIASED_1x1)
            }
        });
        // SAFETY: the run owns one reference to the face, released here and not used after.
        unsafe { ManuallyDrop::drop(&mut run.fontFace) };
        bitmap
    }

    /// The coverage of a glyph run with its pen at (x, y): one byte a pixel, or three for
    /// the stripes of each pixel.
    fn coverage(
        &self,
        run: &DWRITE_GLYPH_RUN,
        x: f32,
        y: f32,
        antialias: DWRITE_TEXT_ANTIALIAS_MODE,
        texture: DWRITE_TEXTURE_TYPE,
    ) -> Option<GlyphBitmap> {
        // SAFETY: the run and what it points at are alive for the calls; the buffer has the
        // size of the bounds times the bytes of a pixel, which is what the texture takes.
        unsafe {
            let analysis = self
                .factory2
                .CreateGlyphRunAnalysis(
                    run,
                    None,
                    DWRITE_RENDERING_MODE_NATURAL_SYMMETRIC,
                    DWRITE_MEASURING_MODE_NATURAL,
                    DWRITE_GRID_FIT_MODE_DEFAULT,
                    antialias,
                    x,
                    y,
                )
                .ok()?;
            let bounds = analysis.GetAlphaTextureBounds(texture).ok()?;
            let (width, height) =
                ((bounds.right - bounds.left).max(0) as u32, (bounds.bottom - bounds.top).max(0) as u32);
            let mut bitmap = GlyphBitmap { left: bounds.left, top: bounds.top, ..GlyphBitmap::default() };
            if width > 0 && height > 0 {
                let channels = if texture == DWRITE_TEXTURE_CLEARTYPE_3x1 { 3 } else { 1 };
                bitmap.pixels = vec![0; (width * height * channels) as usize];
                analysis.CreateAlphaTexture(texture, &bounds, &mut bitmap.pixels).ok()?;
                (bitmap.width, bitmap.height) = (width, height);
            }
            Some(bitmap)
        }
    }

    /// Three bytes a pixel become RGBA: the stripes in screen order, their maximum in alpha.
    fn subpixel(&self, mut bitmap: GlyphBitmap) -> GlyphBitmap {
        let mut rgba = Vec::with_capacity(bitmap.pixels.len() / 3 * 4);
        for stripes in bitmap.pixels.chunks_exact(3) {
            let (r, g, b) = if self.bgr {
                (stripes[2], stripes[1], stripes[0])
            } else {
                (stripes[0], stripes[1], stripes[2])
            };
            rgba.extend_from_slice(&[r, g, b, r.max(g).max(b)]);
        }
        (bitmap.pixels, bitmap.format) = (rgba, GlyphFormat::Subpixel);
        bitmap
    }

    /// A glyph with colour layers: each layer is rasterized as coverage and the layers are
    /// composed in order. `None` when the glyph has none: it is drawn as plain text.
    fn raster_color(&self, run: &DWRITE_GLYPH_RUN, x: f32) -> Option<GlyphBitmap> {
        let mut layers = Vec::new();
        // SAFETY: the run is alive for the call; the enumerator's current run is valid until
        // it moves on, and is only read before that.
        unsafe {
            let colors = self
                .factory2
                .TranslateColorGlyphRun(x, 0.0, run, None, DWRITE_MEASURING_MODE_NATURAL, None, 0)
                .ok()?;
            while colors.MoveNext().ok()?.as_bool() {
                let layer = &*colors.GetCurrentRun().ok()?;
                let coverage = self.coverage(
                    &layer.glyphRun,
                    layer.baselineOriginX,
                    layer.baselineOriginY,
                    DWRITE_TEXT_ANTIALIAS_MODE_GRAYSCALE,
                    DWRITE_TEXTURE_ALIASED_1x1,
                );
                let Some(coverage) = coverage.filter(|c| c.width > 0) else { continue };
                // Palette index 0xFFFF is "the text colour": black here, since tiles are
                // shared by text of every colour.
                let c = layer.runColor;
                let rgba =
                    if layer.paletteIndex == 0xFFFF { [0.0, 0.0, 0.0, 1.0] } else { [c.r, c.g, c.b, c.a] };
                layers.push(ColorLayer { coverage, rgba });
            }
        }
        compose(&layers)
    }
}

/// Paints the layers in order, each over the ones before, into one premultiplied RGBA bitmap
/// that covers them all.
fn compose(layers: &[ColorLayer]) -> Option<GlyphBitmap> {
    let edge = |pick: fn(&GlyphBitmap) -> i32| layers.iter().map(move |l| pick(&l.coverage));
    let (left, top) = (edge(|c| c.left).min()?, edge(|c| c.top).min()?);
    let right = edge(|c| c.left + c.width as i32).max()?;
    let bottom = edge(|c| c.top + c.height as i32).max()?;
    let (width, height) = ((right - left) as usize, (bottom - top) as usize);
    let mut pixels = vec![0.0f32; width * height * 4];
    for layer in layers {
        let c = &layer.coverage;
        for (i, &covered) in c.pixels.iter().enumerate() {
            let cover = covered as f32 / 255.0 * layer.rgba[3];
            if cover == 0.0 {
                continue;
            }
            let (x, y) = (i % c.width as usize, i / c.width as usize);
            let at = ((c.top - top) as usize + y) * width + (c.left - left) as usize + x;
            let px = &mut pixels[at * 4..at * 4 + 4];
            for (value, colour) in px.iter_mut().zip([layer.rgba[0], layer.rgba[1], layer.rgba[2], 1.0]) {
                *value = colour * cover + *value * (1.0 - cover);
            }
        }
    }
    Some(GlyphBitmap {
        left,
        top,
        width: width as u32,
        height: height as u32,
        format: GlyphFormat::Color,
        pixels: pixels.iter().map(|v| (v * 255.0 + 0.5) as u8).collect(),
    })
}
