//! Drawn characters and smoothing (specs/18, "The view"): box drawing and blocks are drawn to
//! their cells' box, not taken from the font; glyphs are smoothed by pixel unless the look
//! says otherwise.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use quadra_reactive::{Signal, signal};
use quadra_render::{Kind, Step};
use quadra_term::vt::ResizePolicy;
use quadra_term::{Session, TermLook, terminal};
use quadra_testing::Harness;
use quadra_text::{
    FaceId, FaceMetrics, FakeBackend, FontKey, GRAYSCALE, GlyphBitmap, ShapedGlyph, TextBackend, TextSystem,
};
use quadra_view::column;

/// The glyph and phase of each glyph rasterized.
type Asked = Rc<RefCell<Vec<(u16, u8)>>>;

/// The fake backend, keeping what it was asked to rasterize.
struct Told(FakeBackend, Asked);

impl TextBackend for Told {
    fn default_family(&self) -> String {
        self.0.default_family()
    }
    fn families(&self) -> Vec<String> {
        self.0.families()
    }
    fn load_fonts(&mut self, files: &[&[u8]]) -> Result<Vec<String>, String> {
        self.0.load_fonts(files)
    }
    fn face(&mut self, key: &FontKey) -> Option<FaceId> {
        self.0.face(key)
    }
    fn metrics(&self, face: FaceId) -> FaceMetrics {
        self.0.metrics(face)
    }
    fn shape(&mut self, text: &str, face: FaceId, size: f32) -> Vec<ShapedGlyph> {
        self.0.shape(text, face, size)
    }
    fn rasterize(&mut self, face: FaceId, glyph: u16, size: f32, phase: u8) -> Option<GlyphBitmap> {
        self.1.borrow_mut().push((glyph, phase));
        self.0.rasterize(face, glyph, size, phase)
    }
}

/// A terminal of 400 x 140 at size 10 (cells 6 x 14, padding 8), and what was rasterized.
fn term() -> (Harness, Session, Signal<TermLook>, Asked) {
    let mut h = Harness::new();
    let told = Rc::new(RefCell::new(Vec::new()));
    h.fonts.borrow_mut().text = TextSystem::new(Box::new(Told(FakeBackend::new(), told.clone())));
    let look = signal(TermLook { size: 10.0, line_height: 1.4, ..TermLook::default() });
    let slot = Rc::new(RefCell::new(None));
    let out = slot.clone();
    h.mount(move || {
        let s = Session::new(ResizePolicy::BottomAnchor);
        *out.borrow_mut() = Some(s.clone());
        column().size(400.0, 140.0).child(terminal(&s).look(look))
    });
    h.settle();
    let s = slot.borrow_mut().take().unwrap();
    (h, s, look, told)
}

/// The bounds of the coverage drawn (glyphs and masks), in device pixels.
fn coverage(h: &mut Harness) -> Vec<[f32; 4]> {
    h.frame();
    let scene = h.tree.scene();
    let layers = h.renderer.steps().iter().filter_map(|step| match step {
        Step::Layer { id, .. } => Some(scene.layer(*id).instances()),
        _ => None,
    });
    layers.flatten().filter(|i| i.kind == Kind::Mono as u32).map(|i| i.bounds).collect()
}

#[test]
fn box_drawing_and_blocks_fill_their_cells_and_are_not_the_fonts() {
    let (mut h, s, _, told) = term();
    s.feeder().feed("a─╭▀█".as_bytes(), Instant::now());
    h.settle();
    let drawn = coverage(&mut h);
    for col in 1..5 {
        let cell = [8.0 + col as f32 * 6.0, 8.0, 6.0, 14.0];
        assert!(drawn.contains(&cell), "column {col} drawn over its whole cell: {drawn:?}");
    }
    let asked: Vec<u16> = told.borrow().iter().map(|it| it.0).collect();
    assert!(asked.contains(&u16::from(b'a')));
    assert!(asked.iter().all(|&g| g < 0x2500), "no glyph of the font for them: {asked:?}");
}

#[test]
fn glyphs_are_smoothed_by_pixel_unless_the_look_says_otherwise() {
    let (mut h, s, look, told) = term();
    s.feeder().feed(b"ab", Instant::now());
    h.settle();
    coverage(&mut h);
    assert!(!told.borrow().is_empty());
    assert!(told.borrow().iter().all(|it| it.1 & GRAYSCALE != 0), "grayscale by default");
    told.borrow_mut().clear();
    look.update(|it| it.grayscale = false);
    h.settle();
    coverage(&mut h);
    assert!(!told.borrow().is_empty(), "rasterized again, as the system smooths");
    assert!(told.borrow().iter().all(|it| it.1 & GRAYSCALE == 0));
}
