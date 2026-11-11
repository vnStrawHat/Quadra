//! Vector graphics on the CPU: paths, the part of SVG that icons and simple art need, and
//! rasterizing to coverage. Drawings may come from files nobody checked: anything reads and
//! renders without a panic, and in a time its size bounds.

mod data;
mod path;
mod raster;
mod svg;

pub use data::numbers;
pub use path::{Path, Polyline};
pub use raster::Coverage;
pub use svg::{Shape, Svg};
