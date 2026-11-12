//! The images of a document (behaviour/21, "Images"): a paragraph that is only images shows
//! them, each no wider than the page; one that cannot be shown is its words and where it is.

use std::path::{Path, PathBuf};

use quadra_kit::{icon, icons, size, token};
use quadra_layout::Align as Across;
use quadra_reactive::signal;
use quadra_ui::{Div, column, image, row, show, text};

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
