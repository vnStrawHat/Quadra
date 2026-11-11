//! What an app is told: errors as values, and a report of each frame.

use std::fmt;
use std::time::Duration;

use quadra_platform::{PlatformError, WindowId};
use quadra_render::{RenderError, RenderStats};
use quadra_tree::{FrameStats, PassTimes};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppError {
    Platform(PlatformError),
    Render(RenderError),
    /// The text backend could not start, in its own words.
    Text(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Platform(e) => e.fmt(f),
            AppError::Render(e) => e.fmt(f),
            AppError::Text(what) => write!(f, "the text backend failed: {what}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<PlatformError> for AppError {
    fn from(e: PlatformError) -> AppError {
        AppError::Platform(e)
    }
}

impl From<RenderError> for AppError {
    fn from(e: RenderError) -> AppError {
        AppError::Render(e)
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

/// A frame that was handed to the display: what it visited and how long each part took.
#[derive(Clone, Copy, Debug)]
pub struct FrameReport {
    pub window: WindowId,
    pub stats: FrameStats,
    pub passes: PassTimes,
    pub render: RenderStats,
    /// Uploading and drawing, as far as the CPU is concerned.
    pub draw: Duration,
    pub present: Duration,
}
