//! Quadra's views of data (specs/13, "Data"): what JSON and YAML say as a value, read by
//! two readers of this crate's own, and the viewer that shows one as a tree.

mod json;
mod value;
mod value_view;
mod yaml;

pub use json::parse_json;
pub use value::{ParseError, Value};
pub use value_view::{ValueView, value_view};
pub use yaml::parse_yaml;
