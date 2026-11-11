//! The workbench (specs/behaviour/16): what the window of a tool is made of around its
//! content. An app that is not a workbench does not need it.

mod chrome;
mod dock;
mod pane;
mod panes;
mod tabs;

pub use chrome::{Activity, ActivityBar, activity_bar, panel_header};
pub use dock::{Dock, DockEvent, Group, TabDrag, Zone, dock, zone_at};
pub use pane::{Axis, Pane};
pub use panes::{Panes, panes};
pub use tabs::{Tab, TabStrip, tab_strip};
