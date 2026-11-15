//! Window state (specs/15): a window opened with `remember(key)` opens where it was when it
//! last closed, kept in the settings as `window.<key>`.

use quadra_platform::{Placement, PlatformWindow, WindowOptions};
use quadra_render::Renderer;
use serde_json::{Value, json};

use crate::{App, Stored};

impl Stored for Placement {
    fn to_json(&self) -> Value {
        json!({ "x": self.x, "y": self.y, "width": self.width, "height": self.height, "maximized": self.maximized })
    }

    fn from_json(value: &Value) -> Option<Self> {
        let int = |key: &str| value.get(key)?.as_i64().and_then(|v| i32::try_from(v).ok());
        let maximized = value.get("maximized")?.as_bool()?;
        Some(Placement {
            x: int("x")?,
            y: int("y")?,
            width: int("width")?,
            height: int("height")?,
            maximized,
        })
    }
}

fn key(name: &str) -> String {
    format!("window.{name}")
}

impl<R: Renderer> App<R> {
    /// The options with the place the window was last at, if it is remembered.
    pub(crate) fn recall(&self, mut options: WindowOptions) -> WindowOptions {
        if let Some(name) = &options.remember
            && options.placement.is_none()
        {
            options.placement = self.settings.value(&key(name), None::<Placement>).get_untracked();
        }
        options
    }

    /// Keeps where a window is: one that closes, and one still open as the app ends.
    pub(crate) fn remember(&self, name: Option<&str>, window: &dyn PlatformWindow) {
        if let Some(name) = name {
            self.settings.value(&key(name), None).set(Some(window.placement()));
        }
    }

    /// The app ends: where the windows still open are is kept, and what waited to be
    /// written is written.
    pub(crate) fn ended(&self) {
        for open in &self.windows {
            self.remember(open.remember.as_deref(), &*open.window);
        }
        self.settings.save_now();
    }
}
