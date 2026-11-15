//! The images of a document (behaviour/21, "Images"): a paragraph that is only images shows
//! them, each no wider than the page; one that cannot be shown is its words and where it is.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use quadra_async::spawn;
use quadra_design::{icon, icons, size, token};
use quadra_layout::Align as Across;
use quadra_prim::Size;
use quadra_reactive::{signal, use_context};
use quadra_tree::Image;
use quadra_view::{
    Div, ImageSource, Images, column, dynamic, image, image_info, image_pixels, row, show, text,
};

use crate::inline::Span;

/// Whether a paragraph says nothing but images.
pub(crate) fn only_images(parts: &[Span]) -> bool {
    parts.iter().any(|part| part.image.is_some())
        && parts.iter().all(|part| part.image.is_some() || part.text.trim().is_empty())
}

/// An image that does not show: its words, and where it would be read from.
fn missing(alt: &str, target: &str) -> Div {
    let said = if alt.is_empty() { target.to_string() } else { format!("{alt} ({target})") };
    let mark = icon(icons::IMAGE).size(14.0).color(token::TEXT_3);
    let line = row().gap(6.0).align_items(Across::Center).text_color(token::TEXT_3);
    line.text_size(size::TEXT_SM.size).child(mark).child(text(said))
}

/// A file beside the document, or anywhere on disk. Nothing is fetched: what is elsewhere
/// is its words.
fn local(target: &str, base: Option<&Path>) -> Option<PathBuf> {
    if target.contains("://") || target.starts_with("data:") || target.is_empty() {
        return None;
    }
    let path = Path::new(target);
    Some(match base.filter(|_| path.is_relative()) {
        Some(base) => base.join(path),
        None => path.to_path_buf(),
    })
}

pub(crate) fn pictures(parts: &[Span], base: Option<&Path>) -> Div {
    let mut out = column().gap(8.0).align_items(Across::Start);
    for part in parts {
        let Some(target) = &part.image else { continue };
        let (alt, target) = (part.text.clone(), target.clone());
        out = out.child(match local(&target, base) {
            None => missing(&alt, &target),
            Some(path) => {
                let failed = signal(false);
                let view = image(path).label(alt.clone()).radius(size::RADIUS_MD);
                let view = view.on_load(move |loaded| failed.set(loaded.is_err()));
                let note = show(move || failed.get(), move || missing(&alt, &target));
                column().align_items(Across::Start).child(view).child(note)
            }
        });
    }
    out
}

/// What an image among words shows as: so large, and its pixels once they are decoded.
pub(super) type Shown = Vec<Option<(Size, Option<Rc<Image>>)>>;

/// The most a side of an image among words is: larger, it is made smaller in its shape.
const MOST: f32 = 320.0;

/// A paragraph with images among its words. Each image is its words in brackets until its
/// file says how large it is; then a box of its line that large (no side over 320 px), and
/// its pixels when they are decoded. What is elsewhere stays its words.
pub(super) fn in_words(
    source: &str,
    parts: &[Span],
    base: Option<&Path>,
    words: impl Fn(&str, &Shown) -> Div + 'static,
) -> Div {
    let paths: Vec<Option<PathBuf>> =
        parts.iter().filter_map(|part| part.image.as_ref().map(|target| local(target, base))).collect();
    let shown = signal(vec![None; paths.len()]);
    if let Some(images) = use_context::<Rc<Images>>() {
        spawn(async move {
            for (k, path) in paths.into_iter().enumerate() {
                let Some(source) = path.map(ImageSource::Path) else { continue };
                let Ok(info) = image_info(&images, &source).await else { continue };
                let (w, h) = (info.width as f32, info.height as f32);
                let fit = (MOST / w.max(h)).min(1.0);
                let size = Size::new((w * fit).round().max(1.0), (h * fit).round().max(1.0));
                shown.update(|all: &mut Shown| all[k] = Some((size, None)));
                // Twice as many pixels for a screen of twice the scale, never more than the file.
                let want = ((size.width * 2.0).min(w) as u32, (size.height * 2.0).min(h) as u32);
                if let Ok(pixels) = image_pixels(&images, &source, want).await {
                    shown.update(|all: &mut Shown| all[k] = Some((size, Some(pixels))));
                }
            }
        })
        .detach();
    }
    let source = source.to_string();
    column().child(dynamic(move || shown.with(|shown| words(&source, shown))))
}
