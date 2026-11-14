//! Quadra's components without a look (specs/17): what a control does, its state, its keys,
//! its focus, what it tells a reader, how its popup opens and closes. A kit draws the parts;
//! `quadra-kit` is one such kit.

pub mod action;
pub mod choice;
pub mod combo;
pub mod dialog;
pub mod disclose;
pub mod hint;
pub mod list;
pub mod live;
pub mod menu;
pub mod notices;
pub mod number;
pub mod pick;
pub mod popup;
pub mod press;
pub mod rules;
pub mod settings;
pub mod slider;
pub mod split;
pub mod table;
pub mod time;
pub mod tree;
pub mod work;

pub use action::keymap;
