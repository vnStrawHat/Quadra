//! The app (specs/10, "App start-up"): it owns the platform, the renderer, the fonts and the
//! windows, and is the platform's handler: events go to the windows' trees, frames to the
//! passes and the renderer.
//!
//! Input dispatch, tasks and timers join with their crates (specs/09, specs/11).

mod crash;
mod events;
pub mod i18n;
mod locale;
pub mod log;
mod remember;
mod report;
mod router;
mod services;
mod settings;
pub mod time;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use quadra_async::{Executor, Timers};
use quadra_platform::{
    Appearance, Cursor, Platform, PlatformError, PlatformWindow, WindowHandle, WindowId, WindowOptions,
};
use quadra_prim::{Color, Rect};
use quadra_reactive::{Scope, Signal, provide, signal};
use quadra_render::{DeviceSize, Pixels, RenderError, Renderer, SurfaceId, SurfaceTarget};
use quadra_text::TextBackend;
use quadra_tree::{Fonts, Keymap, Theme, Tree};
use quadra_ui::{Images, View, mount_with};

pub use i18n::{Arg, I18n, Language, language, t, t_with};
pub use locale::Locale;
pub use report::{AppError, FrameReport, Result};
pub use router::{Router, router};
pub use services::AppDirs;
pub use settings::{Settings, Stored};

type FrameHook = Box<dyn FnMut(&FrameReport)>;

/// One window: the OS window, its tree, and where its frames are drawn.
pub(crate) struct AppWindow {
    /// Everything its view created: ended when the window closes.
    scope: Scope,
    pub(crate) tree: Tree,
    surface: SurfaceId,
    /// The caret and the cursor the window was last told of.
    caret: Option<Rect>,
    cursor: Cursor,
    pub(crate) window: Box<dyn PlatformWindow>,
    /// The name its place is kept under, if it is.
    remember: Option<String>,
}

pub struct App<R: Renderer> {
    pub(crate) platform: Rc<dyn Platform>,
    /// The renderer, open to tests and tools: its counters, and losing its device.
    pub renderer: R,
    fonts: Rc<RefCell<Fonts>>,
    pub(crate) windows: Vec<AppWindow>,
    /// The scope of everything the app creates.
    root: Scope,
    clear: Color,
    /// The key bindings every window of the app uses.
    keymap: Rc<Keymap>,
    /// What the platform's timers mean (specs/11).
    pub(crate) timers: Rc<Timers>,
    /// The tasks of the app's views, run between events.
    pub(crate) tasks: Rc<Executor>,
    /// The theme of every window: views find the signal in their context, and whoever
    /// writes it gives the app another theme.
    theme: Signal<Rc<Theme>>,
    /// The system's appearance, in the context of views too.
    pub(crate) appearance: Signal<Appearance>,
    on_error: Box<dyn FnMut(&AppError)>,
    on_presented: Option<FrameHook>,
    /// Where the app keeps its files, once it is told (specs/15).
    pub(crate) dirs: Option<AppDirs>,
    pub(crate) settings: Rc<Settings>,
    pub(crate) args: Vec<String>,
    pub(crate) quit_on_last: bool,
}

impl<R: Renderer> App<R> {
    /// An app on these backends. `quadra::App::new` picks the ones of the OS.
    pub fn with(platform: Rc<dyn Platform>, renderer: R, text: Box<dyn TextBackend>) -> App<R> {
        // What every view of the app finds in its context: timers, tasks, the clipboard, the
        // theme.
        let (root, timers, tasks) =
            (Scope::child(), Timers::new(platform.clone()), Executor::new(&*platform));
        let (theme, settings, appearance) = root.run(|| {
            provide(timers.clone());
            provide(tasks.clone());
            provide(Images::new(platform.image_decoder()));
            provide(platform.clipboard());
            provide(platform.clone()); // the user's time zone (`time::local_now`)
            let theme = signal(Rc::<Theme>::default());
            provide(theme);
            let settings = Settings::memory();
            provide(settings.clone());
            let appearance = signal(platform.appearance());
            provide(appearance);
            (theme, settings, appearance)
        });
        App {
            platform,
            renderer,
            fonts: Fonts::new(text),
            windows: Vec::new(),
            root,
            clear: Color::WHITE,
            keymap: Rc::new(Keymap::default()),
            timers,
            tasks,
            theme,
            appearance,
            on_error: Box::new(|e| log::error("app", e)),
            on_presented: None,
            dirs: None,
            settings,
            args: std::env::args().skip(1).collect(),
            quit_on_last: true,
        }
    }

    /// The colour under everything a window draws.
    pub fn clear_color(mut self, color: Color) -> App<R> {
        self.clear = color;
        self
    }

    /// Registers bundled fonts with the text system, and returns the family of each file.
    pub fn load_fonts(&mut self, files: &[&[u8]]) -> Result<Vec<String>> {
        let loaded = self.fonts.borrow_mut().text.backend().load_fonts(files);
        loaded.map_err(AppError::Text)
    }

    /// The theme the roles in styles get their colours from (specs/13), in every window. A
    /// view changes it by writing the `Signal<Rc<Theme>>` of its context.
    pub fn theme(self, theme: Theme) -> App<R> {
        self.theme.set(Rc::new(theme));
        self
    }

    /// The app's key bindings: its defaults, then the user's (specs/09). Windows opened
    /// before and after use them.
    pub fn keymap(mut self, keymap: Keymap) -> App<R> {
        self.keymap = Rc::new(keymap);
        for w in &mut self.windows {
            w.tree.set_keymap(self.keymap.clone());
        }
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
        let options = self.recall(options);
        let window = self.platform.open_window(&options)?;
        let target = match window.handle() {
            WindowHandle::Win32 { hwnd } => SurfaceTarget::Win32 { hwnd },
            WindowHandle::Headless => SurfaceTarget::Offscreen,
        };
        let surface = self.renderer.create_surface(target, device_size(&*window))?;
        let scope = self.root.run(Scope::child);
        let mut tree = Tree::new(self.fonts.clone());
        tree.set_keymap(self.keymap.clone());
        let clock = self.platform.clone();
        tree.set_clock(move || clock.now());
        // The window has the app's theme, now and whenever it is another one.
        let (theme, handle) = (self.theme, tree.handle());
        tree.set_theme(theme.get_untracked());
        scope.run(|| {
            let writer = handle.clone();
            handle.bind(move || {
                let theme = theme.get();
                writer.with(move |tree| tree.set_theme(theme));
            });
        });
        scope.run(|| mount_with(&mut tree, view));
        window.request_frame();
        let id = window.id();
        let (caret, cursor, remember) = (None, Cursor::Arrow, options.remember);
        self.windows.push(AppWindow { scope, tree, surface, caret, cursor, window, remember });
        Ok(id)
    }

    /// Runs the platform's loop until the last window closes.
    pub fn run(mut self) -> Result<()> {
        let platform = self.platform.clone();
        let ran = platform.run(&mut self);
        self.settings.save_now(); // what waited to be written
        Ok(ran?)
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    /// A window's tree, for tests and tools.
    pub fn tree(&self, window: WindowId) -> Option<&Tree> {
        self.windows.iter().find(|w| w.window.id() == window).map(|w| &w.tree)
    }

    /// What a window shows, read back: for tools that save pictures and for pixel tests. Only
    /// a window that is drawn offscreen can be read.
    pub fn read_pixels(&mut self, window: WindowId) -> Result<Pixels> {
        let surface = self.windows.iter().find(|w| w.window.id() == window).map(|w| w.surface);
        let surface =
            surface.ok_or_else(|| PlatformError(format!("{window:?} is not a window of the app")))?;
        Ok(self.renderer.read_pixels(surface)?)
    }

    pub(crate) fn close(&mut self, window: WindowId) {
        let Some(at) = self.windows.iter().position(|w| w.window.id() == window) else { return };
        let closed = self.windows.remove(at);
        self.remember(closed.remember.as_deref(), &*closed.window);
        closed.scope.dispose();
        self.renderer.forget_scene(closed.tree.scene().id());
        self.renderer.drop_surface(closed.surface);
        if self.windows.is_empty() && self.quit_on_last {
            self.platform.quit();
        }
    }

    pub(crate) fn draw(&mut self, window: WindowId) {
        let Some(w) = self.windows.iter_mut().find(|w| w.window.id() == window) else { return };
        let device = device_size(&*w.window);
        if device.width == 0 || device.height == 0 {
            return; // minimized: nothing to show
        }
        let stats = w.tree.frame(w.window.size(), w.window.scale());
        if let Some(update) = w.tree.take_semantic_update() {
            w.window.update_semantics(&update); // the a11y pass: only while something listens
        }
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
