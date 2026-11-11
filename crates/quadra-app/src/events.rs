//! The app as the platform's handler: events go to the windows' trees, frames to the passes
//! and the renderer, timers to what set them.

use quadra_platform::{HitRegion, PlatformEvent, PlatformHandler, PointerKind, TimerId, WindowId};
use quadra_prim::Point;
use quadra_reactive::batch;
use quadra_render::Renderer;
use quadra_tree::PointerInput;

use crate::App;

impl<R: Renderer> PlatformHandler for App<R> {
    fn event(&mut self, window: WindowId, event: PlatformEvent) {
        batch(|| match event {
            PlatformEvent::Resize { .. } => {
                if let Some(w) = self.windows.iter_mut().find(|w| w.window.id() == window) {
                    w.tree.set_maximized(w.window.is_maximized());
                    w.window.request_frame();
                }
            }
            PlatformEvent::CloseRequested => self.close(window),
            PlatformEvent::Pointer { kind, button, at, mods } => {
                let input = match kind {
                    PointerKind::Move => PointerInput::Move,
                    PointerKind::Down => PointerInput::Down,
                    PointerKind::Up => PointerInput::Up,
                    PointerKind::Leave => PointerInput::Leave,
                };
                let now = self.platform.now();
                if let Some(w) = self.windows.iter_mut().find(|w| w.window.id() == window) {
                    w.tree.pointer(input, button, at, mods, now);
                }
            }
            PlatformEvent::Wheel { delta, precise, at, mods } => {
                if let Some(w) = self.windows.iter_mut().find(|w| w.window.id() == window) {
                    w.tree.wheel(delta, precise, at, mods);
                }
            }
            PlatformEvent::Key { key, code, mods, down, repeat } => {
                if let Some(w) = self.windows.iter_mut().find(|w| w.window.id() == window) {
                    w.tree.key(key, code, mods, down, repeat);
                }
            }
            PlatformEvent::Focus(active) => {
                if let Some(w) = self.windows.iter_mut().find(|w| w.window.id() == window) {
                    w.tree.set_active(active);
                }
            }
            PlatformEvent::Text(text) => {
                if let Some(w) = self.windows.iter_mut().find(|w| w.window.id() == window) {
                    w.tree.text(&text);
                }
            }
            PlatformEvent::Composition { text, caret } => {
                if let Some(w) = self.windows.iter_mut().find(|w| w.window.id() == window) {
                    w.tree.composition(&text, caret);
                }
            }
            PlatformEvent::SemanticsWanted => {
                if let Some(w) = self.windows.iter_mut().find(|w| w.window.id() == window) {
                    w.tree.listen(true);
                }
            }
            PlatformEvent::Semantic { node, action } => {
                if let Some(w) = self.windows.iter_mut().find(|w| w.window.id() == window)
                    && let Some(node) = w.tree.node_of(node)
                {
                    w.tree.act(node, &action);
                }
            }
        });
        self.request_frames();
    }

    fn frame(&mut self, window: WindowId) {
        self.draw(window);
        self.request_frames();
    }

    fn timer(&mut self, id: TimerId) {
        self.timers.fire(id);
        self.request_frames();
    }

    fn wake(&mut self) {
        self.request_frames();
    }

    fn hit_test(&mut self, window: WindowId, at: Point) -> HitRegion {
        let w = self.windows.iter().find(|w| w.window.id() == window);
        w.map_or(HitRegion::Client, |w| w.tree.window_region_at(at))
    }
}
