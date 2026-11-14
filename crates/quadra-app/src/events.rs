//! The app as the platform's handler: events go to the windows' trees, frames to the passes
//! and the renderer, timers to what set them.

use quadra_platform::{HitRegion, PlatformEvent, PlatformHandler, PointerKind, TimerId, WindowId};
use quadra_prim::Point;
use quadra_reactive::batch;
use quadra_render::Renderer;
use quadra_tree::{PointerInput, WindowRequest};

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
            PlatformEvent::CloseRequested => self.close_asked(window),
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

    /// Another thread woke the loop: a task of this thread has what it waited for.
    fn wake(&mut self) {
        self.request_frames();
    }

    fn hit_test(&mut self, window: WindowId, at: Point) -> HitRegion {
        let w = self.windows.iter().find(|w| w.window.id() == window);
        w.map_or(HitRegion::Client, |w| w.tree.window_region_at(at))
    }
}

impl<R: Renderer> App<R> {
    /// After every entry point: the tasks that were woken run, then a window whose tree has
    /// something to do asks for a frame, and every window does when the renderer wants one.
    pub(crate) fn request_frames(&mut self) {
        self.tasks.run_ready();
        let all = self.renderer.wants_frame();
        let mut closed = Vec::new();
        for w in &mut self.windows {
            // What a view asked of its window: a title bar's buttons.
            for request in w.tree.take_window_requests() {
                match request {
                    WindowRequest::Minimize => w.window.minimize(),
                    WindowRequest::ToggleMaximize => w.window.toggle_maximize(),
                    WindowRequest::Close if w.tree.may_close() => closed.push(w.window.id()),
                    WindowRequest::Close => {}
                    WindowRequest::CloseNow => closed.push(w.window.id()),
                }
            }
            if all || w.tree.needs_frame() {
                w.window.request_frame();
            } else if let Some(update) = w.tree.take_semantic_update() {
                // No frame is coming to send it after: a listener that just arrived, a focus
                // that moved.
                w.window.update_semantics(&update);
            }
            // Where text is typed, or that it is not: what an input method goes by.
            let caret = w.tree.text_caret();
            if std::mem::replace(&mut w.caret, caret) != caret {
                w.window.set_text_input(caret);
            }
            let cursor = w.tree.cursor();
            if std::mem::replace(&mut w.cursor, cursor) != cursor {
                w.window.set_cursor(cursor);
            }
        }
        for window in closed {
            self.close(window);
        }
    }

    /// A new device after the old one was lost: the glyph tiles are gone with it, so every
    /// window paints and uploads everything again (specs/07, "Resources and failure").
    /// The user asks a window to close: it does if what is in it says it may.
    fn close_asked(&mut self, window: WindowId) {
        if let Some(w) = self.windows.iter_mut().find(|w| w.window.id() == window)
            && w.tree.may_close()
        {
            self.close(window);
        }
    }

    pub(crate) fn recover(&mut self) {
        if let Err(e) = self.renderer.recover() {
            return (self.on_error)(&e.into()); // the windows keep their last frame
        }
        self.fonts.borrow_mut().clear_tiles();
        for w in &mut self.windows {
            w.tree.repaint_all();
            w.tree.scene_mut().invalidate_all();
            w.window.request_frame();
        }
    }
}
