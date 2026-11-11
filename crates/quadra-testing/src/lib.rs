//! Quadra's test harness (specs/12, "The harness"): a window on the fake text backend and the
//! null renderer. Every frame is checked: what the incremental passes left must be what a
//! paint from scratch gives, and the renderer's copy must be the scene.
//!
//! Finding and driving controls through the semantic tree joins when semantics and input do.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_prim::{Color, Size};
use quadra_render::{DeviceSize, NullRenderer, RenderStats, Renderer, Step, SurfaceId, SurfaceTarget};
use quadra_text::{FakeBackend, TextSystem};
use quadra_tree::{FrameStats, Tree};

pub struct Harness {
    pub tree: Tree,
    pub renderer: NullRenderer,
    /// The text system the tree shapes with: its counters are part of what a test may assert.
    pub text: Rc<RefCell<TextSystem>>,
    surface: SurfaceId,
    /// The window's size in logical pixels, and its scale. Change them to resize.
    pub size: Size,
    pub scale: f32,
}

impl Default for Harness {
    fn default() -> Harness {
        Harness::new()
    }
}

impl Harness {
    /// A window of 800 x 600 logical pixels at scale 1.
    pub fn new() -> Harness {
        let text = Rc::new(RefCell::new(TextSystem::new(Box::new(FakeBackend::new()))));
        let mut renderer = NullRenderer::new();
        let size = DeviceSize { width: 800, height: 600 };
        let surface =
            renderer.create_surface(SurfaceTarget::Offscreen, size).expect("the null renderer has surfaces");
        let tree = Tree::new(text.clone());
        Harness { tree, renderer, text, surface, size: Size::new(800.0, 600.0), scale: 1.0 }
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
        assert!(!self.tree.needs_frame(), "a frame left work to do");
        (stats, self.renderer.stats())
    }

    /// Runs a frame if there is anything to do; `None` when there is not.
    pub fn frame_if_needed(&mut self) -> Option<(FrameStats, RenderStats)> {
        self.tree.needs_frame().then(|| self.frame())
    }
}
