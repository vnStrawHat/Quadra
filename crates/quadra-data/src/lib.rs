//! Quadra's views of data (specs/13, "Data"): what JSON and YAML say as a value, read by
//! two readers of this crate's own, and the viewer that shows one as a tree; a log an app
//! appends lines to, and the view that follows it.

mod ansi;
mod json;
mod log;
mod log_view;
mod value;
mod value_view;
mod yaml;
mod yaml_view;

pub use ansi::{Ink, Pen};
pub use json::parse_json;
pub use log::{Log, clock};
pub use log_view::{LogView, log_view};
pub use value::{ParseError, Value};
pub use value_view::{ValueView, value_view};
pub use yaml::{parse_yaml, to_yaml};
pub use yaml_view::{YamlView, yaml_view};
