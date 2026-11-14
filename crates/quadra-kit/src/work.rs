//! The workbench (specs/behaviour/16): what the window of a tool is made of around its
//! content. An app that is not a workbench does not need it.

mod chrome;
mod panes;
mod tabs;

pub use chrome::{Activity, ActivityBar, activity_bar, panel_header};
pub use panes::panes;
pub use quadra_base::work::{Axis, Dock, DockEvent, Group, Pane, Panes, Tab, TabDrag, Zone, dock, zone_at};
pub use tabs::{TabStrip, tab_strip};
