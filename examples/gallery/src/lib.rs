//! The gallery (specs/13, "Components"): every component of the kit on a page of its own,
//! in every variant and state. It is the kit's acceptance test: its pages are compared with
//! the reference pictures (specs/12, "Snapshots").

mod common;
mod pages;
mod registry;
mod shell;

pub use common::notes;
pub use registry::{PAGES, PageInfo};
pub use shell::{Gallery, OpenPalette, Zoom};

/// The gallery's keys: the kit's, the editor's, Ctrl-K for the command palette wherever the
/// focus is, and Ctrl-=, Ctrl-minus and Ctrl-0 for the size of the whole window.
pub fn keymap() -> quadra::Keymap {
    let mut keys = quadra_design::keymap();
    keys.bind("ctrl-k", OpenPalette).expect("the binding is well formed");
    for (stroke, step) in [("ctrl-=", 1), ("ctrl-minus", -1), ("ctrl-0", 0)] {
        keys.bind(stroke, Zoom(step)).expect("the binding is well formed");
    }
    quadra_code::bind_keys(&mut keys);
    keys
}
