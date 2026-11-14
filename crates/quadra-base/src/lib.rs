//! Quadra's components without a look (specs/17): what a control does, its state, its keys,
//! its focus, what it tells a reader, how its popup opens and closes. A kit draws the parts;
//! `quadra-kit` is one such kit.

pub mod action;
pub mod choice;
pub mod popup;

pub use action::keymap;
