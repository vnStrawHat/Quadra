//! The gallery (specs/13, "Components"): every component of the kit on a page of its own,
//! in every variant and state. It is the kit's acceptance test: its pages are compared with
//! the reference pictures (specs/12, "Snapshots").

mod common;
mod pages;
mod registry;
mod shell;

pub use registry::{PAGES, PageInfo};
pub use shell::{Gallery, OpenPalette};

/// The gallery's keys: the kit's, the editor's, and Ctrl-K for the command palette wherever
/// the focus is.
pub fn keymap() -> quadra::Keymap {
    let mut keys = quadra_kit::keymap();
    keys.bind("ctrl-k", OpenPalette).expect("the binding is well formed");
    quadra_code::bind_keys(&mut keys);
    keys
}
