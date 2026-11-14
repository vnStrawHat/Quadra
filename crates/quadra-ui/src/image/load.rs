//! An image loaded by hand, for what shows one without a view of its own: a box of a line
//! of rich text (`inline_image`). Read and decoded on the pool as the view's are, through
//! the app's `Images`.

use std::rc::Rc;

use quadra_async::background;
use quadra_platform::{ImageError, ImageInfo};
use quadra_prim::{Point, Rect, Size};
use quadra_text::TextStyle;
use quadra_tree::{GlyphTile, Image, InlineBox, MeasureCx, PaintCx, paint_image};

use super::{ImageSource, Images, read};

/// The size a file says it is, from its header. `images` is the app's, taken where a view is
/// mounted (`use_context::<Rc<Images>>()`): without an app around there is none.
pub async fn image_info(images: &Images, source: &ImageSource) -> Result<ImageInfo, ImageError> {
    if let Some((info, _)) = source.key().and_then(|key| images.known(&key)) {
        return Ok(info);
    }
    let (_, info) = read(images, source).await?;
    if let Some(key) = source.key() {
        images.remember(&key, info);
    }
    Ok(info)
}

/// The pixels of a file at a size, shared with the views that show it at that size.
pub async fn image_pixels(
    images: &Images,
    source: &ImageSource,
    (width, height): (u32, u32),
) -> Result<Rc<Image>, ImageError> {
    let key = source.key();
    if let Some(image) = key.as_ref().and_then(|key| images.get(key, (width, height))) {
        return Ok(image);
    }
    let (file, _) = read(images, source).await?;
    let decoder = images.decoder.clone();
    let decoded = background(move || decoder.decode(&file, width, height)).await;
    let image = Image::new(decoded.map_err(|panic| ImageError::Corrupt(panic.0))??);
    Ok(match &key {
        Some(key) => images.put(key, (width, height), image),
        None => image,
    })
}

/// A box of a line that is an image: its bottom on the baseline, so large; empty until it
/// is given its pixels.
pub struct InlineImage {
    size: Size,
    image: Option<Rc<Image>>,
    words: String,
    tiles: Vec<GlyphTile>,
}

/// An image in a line of rich text, `size` large, with what it says to a reader.
pub fn inline_image(size: Size, image: Option<Rc<Image>>, words: impl Into<String>) -> InlineImage {
    InlineImage { size, image, words: words.into(), tiles: Vec::new() }
}

impl InlineBox for InlineImage {
    fn measure(&mut self, _: &mut MeasureCx, _: &TextStyle) -> (f32, f32, f32) {
        (self.size.width, self.size.height, 0.0)
    }

    fn paint(&mut self, cx: &mut PaintCx, origin: Point, _: quadra_prim::Color) {
        let Some(image) = self.image.clone() else { return };
        let s = cx.scale;
        let at = Point::new(origin.x * s, (origin.y - self.size.height) * s);
        let dest = Rect::new(
            at.x.round(),
            at.y.round(),
            (self.size.width * s).round(),
            (self.size.height * s).round(),
        );
        let (clip, mut tiles) = (cx.clip, Vec::new());
        paint_image(cx, &image, dest, &clip, &mut tiles);
        self.tiles = tiles; // the old ones are let go after the new are held
    }

    fn says(&self) -> &str {
        &self.words
    }
}
