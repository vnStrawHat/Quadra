//! The workbench (specs/17, "The components"; specs/behaviour/16): the pane tree, docking, and
//! the tab strip whose tabs move and tear off. The look gives a tab, a strip, the seams and the
//! veil over a pane.

mod dock;
mod pane;
mod panes;
mod tabs;

pub use dock::{Dock, DockEvent, Group, TabDrag, Zone, dock, zone_at, zone_rect};
pub use pane::{Axis, Pane, Part};
pub use panes::{LEAST, Panes, Parts, panes};
pub use tabs::{Measured, SLIDE, Strip, TEAR, Tab, strip};
