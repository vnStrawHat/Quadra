//! A window for tests: a tree, a null renderer, and a check after every frame that the
//! incremental passes left exactly what painting from scratch gives.
#![allow(dead_code)]

use std::cell::RefCell;
use std::rc::Rc;

use quadra_layout::{Direction, Length};
use quadra_prim::{Color, Size};
use quadra_render::{DeviceSize, NullRenderer, RenderStats, Renderer, Step, SurfaceId, SurfaceTarget};
use quadra_text::FakeBackend;
use quadra_tree::{Fonts, FrameStats, NodeId, PaintStyle, Style, Text, TextValues, Tree};

pub struct Window {
    pub tree: Tree,
    pub renderer: NullRenderer,
    pub fonts: Rc<RefCell<Fonts>>,
    surface: SurfaceId,
    pub size: Size,
    pub scale: f32,
}

impl Window {
    pub fn new() -> Window {
        let fonts = Fonts::new(Box::new(FakeBackend::new()));
        let mut renderer = NullRenderer::new();
        let surface = renderer
            .create_surface(SurfaceTarget::Offscreen, DeviceSize { width: 800, height: 600 })
            .unwrap();
        let tree = Tree::new(fonts.clone());
        Window { tree, fonts, renderer, surface, size: Size::new(800.0, 600.0), scale: 1.0 }
    }

    /// Runs a frame and draws it. The layer must be what a paint from scratch gives, and the
    /// renderer's copy must be the layer.
    pub fn frame(&mut self) -> (FrameStats, RenderStats) {
        let stats = self.tree.frame(self.size, self.scale);
        self.renderer.sync_atlas(self.fonts.borrow_mut().glyphs.core_mut()).unwrap();
        self.renderer.draw(self.surface, self.tree.scene_mut(), Color::WHITE).unwrap();
        assert_eq!(self.tree.verify_paint(), None);
        let scene = self.tree.scene();
        for step in self.renderer.steps() {
            if let Step::Layer { id, .. } = *step {
                assert_eq!(self.renderer.uploaded(scene.id(), id), Some(scene.layer(id).instances()));
            }
        }
        assert!(!self.tree.needs_frame(), "a frame leaves nothing to do");
        (stats, self.renderer.stats())
    }

    /// A box of a fixed size and a background.
    pub fn block(&mut self, w: f32, h: f32, color: u32) -> NodeId {
        self.tree.insert(block_style(w, h, color))
    }

    /// A text node at size 10: 6 px a character on the fake backend, 14 px lines.
    pub fn text(&mut self, text: &str) -> NodeId {
        let text_values = TextValues { size: Some(10.0), ..TextValues::default() };
        let style = Style { text: text_values, ..Style::default() };
        self.tree.insert_widget(style, Box::new(Text::new(text)))
    }

    /// A column that sizes to its children.
    pub fn column(&mut self, children: &[NodeId]) -> NodeId {
        let layout = quadra_layout::Style { direction: Direction::Column, ..Default::default() };
        let id = self.tree.insert(Style { layout, ..Style::default() });
        self.tree.set_children(id, children);
        id
    }
}

pub fn block_style(w: f32, h: f32, color: u32) -> Style {
    let layout = quadra_layout::Style { width: Length::Px(w), height: Length::Px(h), ..Default::default() };
    let paint = PaintStyle { background: Color::rgb(color), ..PaintStyle::default() };
    Style { layout, paint, ..Style::default() }
}
