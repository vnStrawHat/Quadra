//! Quadra's test harness (specs/12, "The harness"): a window on the fake text backend and the
//! null renderer. Every frame is checked: what the incremental passes left must be what a
//! paint from scratch gives, and the renderer's copy must be the scene.
//!
//! Finding and driving controls through the semantic tree joins when semantics and input do.

mod find;
pub mod fuzz;

use std::cell::RefCell;
use std::rc::Rc;

use std::time::{Duration, Instant};

use quadra_async::{Executor, Timers};
use quadra_platform::{
    Clipboard, HeadlessPlatform, Key, Mods, MouseButton, Platform, PlatformEvent, PlatformHandler, TimerId,
    WindowId,
};
use quadra_prim::{Color, Point, Size};
use quadra_reactive::provide;
use quadra_render::{DeviceSize, NullRenderer, RenderStats, Renderer, Step, SurfaceId, SurfaceTarget};
use quadra_text::FakeBackend;
use quadra_tree::{Fonts, FrameStats, NodeId, PointerInput, Tree};
use quadra_view::{Images, View, mount_with};

pub use find::Found;

pub struct Harness {
    pub tree: Tree,
    pub renderer: NullRenderer,
    /// The text system and glyph atlases the tree uses: their counters are part of what a
    /// test may assert.
    pub fonts: Rc<RefCell<Fonts>>,
    surface: SurfaceId,
    /// The window's size in logical pixels, and its scale. Change them to resize.
    pub size: Size,
    pub scale: f32,
    /// The clock, moved by [`Harness::advance`] and never by time itself, and what runs on
    /// it: the timers of views mounted with [`Harness::mount`].
    platform: HeadlessPlatform,
    timers: Rc<Timers>,
    tasks: Rc<Executor>,
    /// The images those views find: flat test images, decoded on the pool as in an app.
    pub images: Rc<Images>,
    /// The clipboard those views find: a variable, never the real one.
    pub clipboard: Rc<dyn Clipboard>,
}

/// Gives the platform's timers to what set them, and runs the tasks they wake.
struct Fire(Rc<Timers>, Rc<Executor>);

impl PlatformHandler for Fire {
    fn event(&mut self, _: WindowId, _: PlatformEvent) {}
    fn frame(&mut self, _: WindowId) {}
    fn timer(&mut self, id: TimerId) {
        self.0.fire(id);
        self.1.run_ready();
    }
    fn wake(&mut self) {
        self.1.run_ready();
    }
}

impl Default for Harness {
    fn default() -> Harness {
        Harness::new()
    }
}

impl Harness {
    /// A window of 800 x 600 logical pixels at scale 1.
    pub fn new() -> Harness {
        let fonts = Fonts::new(Box::new(FakeBackend::new()));
        let mut renderer = NullRenderer::new();
        let size = DeviceSize { width: 800, height: 600 };
        let surface =
            renderer.create_surface(SurfaceTarget::Offscreen, size).expect("the null renderer has surfaces");
        let mut tree = Tree::new(fonts.clone());
        let platform = HeadlessPlatform::new();
        let (timers, tasks) = (Timers::new(Rc::new(platform.clone())), Executor::new(&platform));
        let (size, clipboard, clock) = (Size::new(800.0, 600.0), platform.clipboard(), platform.clone());
        tree.set_clock(move || clock.now()); // what animates goes by the harness's time
        let images = Images::new(platform.image_decoder());
        Harness {
            tree,
            renderer,
            fonts,
            surface,
            size,
            scale: 1.0,
            platform,
            timers,
            tasks,
            images,
            clipboard,
        }
    }

    /// Builds a view as an app's window does, where it finds timers, tasks and the clipboard
    /// in its context, and mounts it as the root.
    pub fn mount<V: View>(&mut self, build: impl FnOnce() -> V) -> NodeId {
        let (timers, tasks, clipboard) = (self.timers.clone(), self.tasks.clone(), self.clipboard.clone());
        let images = self.images.clone();
        mount_with(&mut self.tree, move || {
            provide(timers);
            provide(tasks);
            provide(images);
            provide(clipboard);
            build()
        })
    }

    /// The harness's time.
    pub fn now(&self) -> Instant {
        self.platform.now()
    }

    /// Moves the harness's time forward: the timers that come due fire, in order.
    pub fn advance(&mut self, by: Duration) {
        self.platform.advance(by, &mut Fire(self.timers.clone(), self.tasks.clone()));
    }

    /// Runs tasks and frames until no task is ready, the pool has brought what it was given
    /// and nothing is left to draw (specs/12). The clock does not move: what waits for a
    /// timer waits on.
    pub fn settle(&mut self) {
        loop {
            while self.tasks.run_ready() {}
            assert!(quadra_async::wait_idle(Duration::from_secs(30)), "work on the pool did not end");
            if self.tasks.run_ready() {
                continue;
            }
            // What animates asks for frames without end: it gets one.
            if self.frame_if_needed().is_none() || self.tree.animating() {
                return;
            }
        }
    }

    /// A pointer event of the window, with the main button and no modifier.
    pub fn pointer(&mut self, input: PointerInput, x: f32, y: f32) {
        self.tree.pointer(input, MouseButton::Left, Point::new(x, y), Mods::NONE, self.now());
    }

    /// Turns the wheel by lines at a point: positive is away from the user, towards the
    /// start of the content.
    pub fn wheel(&mut self, x: f32, y: f32, lines: f32) {
        self.tree.wheel(Point::new(0.0, lines), false, Point::new(x, y), Mods::NONE);
    }

    /// Presses a key and releases it, with modifiers held.
    pub fn key(&mut self, key: Key, mods: Mods) {
        self.tree.key(key, 0, mods, true, false);
        self.tree.key(key, 0, mods, false, false);
    }

    /// Types text, as the keyboard or an input method commits it.
    pub fn type_text(&mut self, text: &str) {
        self.tree.text(text);
    }

    /// Shows text an input method is composing, with the caret at its end; an empty text
    /// ends the composition. What it commits is typed with [`Harness::type_text`].
    pub fn compose(&mut self, text: &str) {
        self.tree.composition(text, text.len(), 0..0);
    }

    /// The same, with a clause being converted: bytes of the text.
    pub fn compose_clause(&mut self, text: &str, target: std::ops::Range<usize>) {
        self.tree.composition(text, text.len(), target);
    }

    /// Moves the pointer to a point, presses the main button and releases it.
    pub fn click(&mut self, x: f32, y: f32) {
        self.pointer(PointerInput::Move, x, y);
        self.pointer(PointerInput::Down, x, y);
        self.pointer(PointerInput::Up, x, y);
    }

    /// Runs one frame and draws it, and returns what the tree visited and what the renderer
    /// did. Panics if incremental paint left anything else than a paint from scratch would,
    /// if the renderer's copy of a layer is not the scene's, or if the frame left work.
    pub fn frame(&mut self) -> (FrameStats, RenderStats) {
        let stats = self.tree.frame(self.size, self.scale);
        let device = DeviceSize {
            width: (self.size.width * self.scale).round() as u32,
            height: (self.size.height * self.scale).round() as u32,
        };
        self.renderer.resize_surface(self.surface, device).expect("the surface exists");
        // One statement: the fonts must be free again before the paint check below.
        let synced = self.renderer.sync_atlas(self.fonts.borrow_mut().glyphs.core_mut());
        synced.expect("the null renderer takes tiles");
        self.renderer
            .draw(self.surface, self.tree.scene_mut(), Color::WHITE)
            .expect("the null renderer draws");
        if let Some(difference) = self.tree.verify_paint() {
            panic!("incremental paint differs from a paint from scratch: {difference}");
        }
        let scene = self.tree.scene();
        for step in self.renderer.steps() {
            if let Step::Layer { id, .. } = *step {
                let uploaded = self.renderer.uploaded(scene.id(), id);
                assert!(
                    uploaded == Some(scene.layer(id).instances()),
                    "the renderer's copy of {id:?} is stale"
                );
            }
        }
        assert!(self.tree.animating() || !self.tree.needs_frame(), "a frame left work to do");
        (stats, self.renderer.stats())
    }

    /// Runs a frame if there is anything to do; `None` when there is not.
    pub fn frame_if_needed(&mut self) -> Option<(FrameStats, RenderStats)> {
        self.tree.needs_frame().then(|| self.frame())
    }
}
