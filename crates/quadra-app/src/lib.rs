//! The app (specs/10, "App start-up"): it owns the platform, the renderer, the fonts and the
//! windows, and is the platform's handler: events go to the windows' trees, frames to the
//! passes and the renderer.
//!
//! Input dispatch, tasks and timers join with their crates (specs/09, specs/11).

mod report;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use quadra_platform::{
    Platform, PlatformEvent, PlatformHandler, PlatformWindow, PointerKind, TimerId, WindowHandle, WindowId,
    WindowOptions,
};
use quadra_prim::Color;
use quadra_reactive::{Scope, batch};
use quadra_render::{DeviceSize, RenderError, Renderer, SurfaceId, SurfaceTarget};
use quadra_text::TextBackend;
use quadra_tree::{Fonts, PointerInput, Tree};
use quadra_ui::{View, mount_with};

pub use report::{AppError, FrameReport, Result};

type FrameHook = Box<dyn FnMut(&FrameReport)>;

/// One window: the OS window, its tree, and where its frames are drawn.
struct AppWindow {
    /// Everything its view created: ended when the window closes.
    scope: Scope,
    tree: Tree,
    surface: SurfaceId,
    window: Box<dyn PlatformWindow>,
}

pub struct App<R: Renderer> {
    platform: Rc<dyn Platform>,
    /// The renderer, open to tests and tools: its counters, and losing its device.
    pub renderer: R,
    fonts: Rc<RefCell<Fonts>>,
    windows: Vec<AppWindow>,
    /// The scope of everything the app creates.
    root: Scope,
    clear: Color,
    on_error: Box<dyn FnMut(&AppError)>,
    on_presented: Option<FrameHook>,
}

impl<R: Renderer> App<R> {
    /// An app on these backends. `quadra::App::new` picks the ones of the OS.
    pub fn with(platform: Rc<dyn Platform>, renderer: R, text: Box<dyn TextBackend>) -> App<R> {
        App {
            platform,
            renderer,
            fonts: Fonts::new(text),
            windows: Vec::new(),
            root: Scope::child(),
            clear: Color::WHITE,
            on_error: Box::new(|e| eprintln!("quadra: {e}")),
            on_presented: None,
        }
    }

    /// The colour under everything a window draws.
    pub fn clear_color(mut self, color: Color) -> App<R> {
        self.clear = color;
        self
    }

    /// What the app could not handle itself is given to `hook`: a device that could not be
    /// made again, a failed draw. The default prints it.
    pub fn on_error(mut self, hook: impl FnMut(&AppError) + 'static) -> App<R> {
        self.on_error = Box::new(hook);
        self
    }

    /// `hook` runs each time a frame of a window has been handed to the display: what
    /// measures start-up, or records frames, listens here.
    pub fn on_presented(mut self, hook: impl FnMut(&FrameReport) + 'static) -> App<R> {
        self.on_presented = Some(Box::new(hook));
        self
    }

    /// Opens a window showing a view. `view` runs once.
    pub fn window<V: View>(mut self, options: WindowOptions, view: impl FnOnce() -> V) -> Result<App<R>> {
        self.open_window(options, view)?;
        Ok(self)
    }

    pub fn open_window<V: View>(
        &mut self,
        options: WindowOptions,
        view: impl FnOnce() -> V,
    ) -> Result<WindowId> {
        let window = self.platform.open_window(&options)?;
        let target = match window.handle() {
            WindowHandle::Win32 { hwnd } => SurfaceTarget::Win32 { hwnd },
            WindowHandle::Headless => SurfaceTarget::Offscreen,
        };
        let surface = self.renderer.create_surface(target, device_size(&*window))?;
        let scope = self.root.run(Scope::child);
        let mut tree = Tree::new(self.fonts.clone());
        scope.run(|| mount_with(&mut tree, view));
        window.request_frame();
        let id = window.id();
        self.windows.push(AppWindow { scope, tree, surface, window });
        Ok(id)
    }

    /// Runs the platform's loop until the last window closes.
    pub fn run(mut self) -> Result<()> {
        let platform = self.platform.clone();
        Ok(platform.run(&mut self)?)
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    /// A window's tree, for tests and tools.
    pub fn tree(&self, window: WindowId) -> Option<&Tree> {
        self.windows.iter().find(|w| w.window.id() == window).map(|w| &w.tree)
    }

    fn close(&mut self, window: WindowId) {
        let Some(at) = self.windows.iter().position(|w| w.window.id() == window) else { return };
        let closed = self.windows.remove(at);
        closed.scope.dispose();
        self.renderer.forget_scene(closed.tree.scene().id());
        self.renderer.drop_surface(closed.surface);
        if self.windows.is_empty() {
            self.platform.quit();
        }
    }

    fn draw(&mut self, window: WindowId) {
        let Some(w) = self.windows.iter_mut().find(|w| w.window.id() == window) else { return };
        let device = device_size(&*w.window);
        if device.width == 0 || device.height == 0 {
            return; // minimized: nothing to show
        }
        let stats = w.tree.frame(w.window.size(), w.window.scale());
        let (renderer, fonts, started) = (&mut self.renderer, &self.fonts, Instant::now());
        let mut frame = || {
            renderer.resize_surface(w.surface, device)?;
            renderer.sync_atlas(fonts.borrow_mut().glyphs.core_mut())?;
            renderer.sync_atlas(fonts.borrow_mut().color_glyphs.core_mut())?;
            renderer.draw(w.surface, w.tree.scene_mut(), self.clear)?;
            let drawn = started.elapsed();
            renderer.present(w.surface)?;
            Ok((drawn, started.elapsed() - drawn))
        };
        match frame() {
            Ok((draw, present)) => {
                if let Some(hook) = &mut self.on_presented {
                    let (passes, render) = (w.tree.times(), self.renderer.stats());
                    hook(&FrameReport { window, stats, passes, render, draw, present });
                }
            }
            Err(RenderError::DeviceLost) => self.recover(),
            Err(e) => (self.on_error)(&e.into()),
        }
    }

    /// A new device after the old one was lost: the glyph tiles are gone with it, so every
    /// window paints and uploads everything again (specs/07, "Resources and failure").
    fn recover(&mut self) {
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

    /// After every entry point: a window whose tree has something to do asks for a frame,
    /// and every window does when the renderer wants one.
    fn request_frames(&mut self) {
        let all = self.renderer.wants_frame();
        for w in self.windows.iter().filter(|w| all || w.tree.needs_frame()) {
            w.window.request_frame();
        }
    }
}

impl<R: Renderer> Drop for App<R> {
    fn drop(&mut self) {
        self.root.dispose();
    }
}

/// The size of a window's content in device pixels.
fn device_size(window: &dyn PlatformWindow) -> DeviceSize {
    let (size, scale) = (window.size(), window.scale());
    DeviceSize { width: (size.width * scale).round() as u32, height: (size.height * scale).round() as u32 }
}

impl<R: Renderer> PlatformHandler for App<R> {
    fn event(&mut self, window: WindowId, event: PlatformEvent) {
        batch(|| match event {
            PlatformEvent::Resize { .. } => {
                if let Some(w) = self.windows.iter().find(|w| w.window.id() == window) {
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
            _ => {} // keys and text come with focus (specs/09)
        });
        self.request_frames();
    }

    fn frame(&mut self, window: WindowId) {
        self.draw(window);
        self.request_frames();
    }

    fn timer(&mut self, _: TimerId) {
        self.request_frames();
    }

    fn wake(&mut self) {
        self.request_frames();
    }
}
