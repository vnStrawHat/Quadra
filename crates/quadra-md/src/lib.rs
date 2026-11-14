//! Quadra's Markdown (specs/13, "Markdown"; what it reads is behaviour/21): the parser of
//! blocks and of the inlines in them, the outline of a document, and the view that shows
//! one. The parser is plain functions over text: a document that is not ours costs what its
//! length costs, and never more.

mod block;
mod inline;
mod lines;
pub mod mermaid;
mod outline;
pub mod tex;
mod view;

pub use block::{Align, Block, Kind, parse};
pub use inline::{Span, parse_inline};
pub use outline::{Heading, anchor, block_at, outline, plain, words};
pub use view::{Markdown, Place, markdown};
