//! Ligatures (specs/18, "The view"): on, a token of cells is shaped as one word and its glyphs
//! stay on the grid, a ligature over the cells of its characters; off, a glyph a cell.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use quadra_reactive::{Signal, signal};
use quadra_render::{Kind, Step};
use quadra_term::vt::ResizePolicy;
use quadra_term::{Session, TermLook, TerminalWidget, terminal};
use quadra_testing::Harness;
use quadra_text::{
    FaceId, FaceMetrics, FakeBackend, FontKey, GlyphBitmap, ShapedGlyph, TextBackend, TextSystem,
};
use quadra_tree::Role;
use quadra_view::column;

/// The fake backend, with `->` one glyph of two advances, as a coding font's ligature.
struct Arrows(FakeBackend);

impl TextBackend for Arrows {
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
        let mut glyphs = self.0.shape(text, face, size);
        let mut k = 0;
        while k + 1 < glyphs.len() {
            if glyphs[k].id == u16::from(b'-') && glyphs[k + 1].id == u16::from(b'>') {
                let next = glyphs.remove(k + 1);
                glyphs[k] =
                    ShapedGlyph { id: 0xE100, advance: glyphs[k].advance + next.advance, ..glyphs[k] };
            }
            k += 1;
        }
        glyphs
    }
    fn rasterize(&mut self, face: FaceId, glyph: u16, size: f32, phase: u8) -> Option<GlyphBitmap> {
        self.0.rasterize(face, glyph, size, phase)
    }
}

/// A terminal of cells 9 px wide (the advance is 6) at size 10, on a text system of `backend`.
fn term(backend: Box<dyn TextBackend>, ligatures: bool) -> (Harness, Session, Signal<TermLook>) {
    let mut h = Harness::new();
    h.fonts.borrow_mut().text = TextSystem::new(backend);
    let look = signal(TermLook {
        size: 10.0,
        line_height: 1.4,
        cell_width: Some(9.0),
        ligatures,
        ..TermLook::default()
    });
    let slot = Rc::new(RefCell::new(None));
    let out = slot.clone();
    h.mount(move || {
        let s = Session::new(ResizePolicy::BottomAnchor);
        *out.borrow_mut() = Some(s.clone());
        column().size(400.0, 140.0).child(terminal(&s).look(look))
    });
    h.settle();
    let s = slot.borrow_mut().take().unwrap();
    (h, s, look)
}

/// The glyphs drawn, a row after another from the left: where each starts.
fn at(h: &mut Harness) -> Vec<(f32, f32)> {
    h.frame();
    let scene = h.tree.scene();
    let layers = h.renderer.steps().iter().filter_map(|step| match step {
        Step::Layer { id, .. } => Some(scene.layer(*id).instances()),
        _ => None,
    });
    let mono = layers.flatten().filter(|i| i.kind == Kind::Mono as u32);
    let mut out: Vec<(f32, f32)> = mono.map(|i| (i.bounds[1], i.bounds[0])).collect();
    out.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    out
}

fn xs(h: &mut Harness) -> Vec<f32> {
    at(h).into_iter().map(|it| it.1).collect()
}

#[test]
fn a_ligature_spans_the_cells_of_its_characters() {
    let (mut h, s, look) = term(Box::new(Arrows(FakeBackend::new())), false);
    s.feeder().feed(b"a->b", Instant::now());
    h.settle();
    // The room at the left is 8; each glyph of 6 in the middle of its cell of 9 (1.5 in, drawn
    // on the whole pixel).
    assert_eq!(xs(&mut h), [9.0, 18.0, 27.0, 36.0], "off: a glyph a cell");
    look.update(|it| it.ligatures = true);
    h.settle();
    assert_eq!(xs(&mut h), [9.0, 20.0, 36.0], "the arrow in the middle of its two cells");
}

/// The words a feed has the text system shape.
fn shaped(h: &mut Harness, s: &Session, bytes: &[u8]) -> u32 {
    h.fonts.borrow_mut().text.take_stats();
    s.feeder().feed(bytes, Instant::now());
    h.settle();
    h.frame();
    h.fonts.borrow_mut().text.take_stats().words_shaped
}

#[test]
fn a_token_is_shaped_once_and_its_glyphs_keep_their_columns() {
    let (mut h, s, look) = term(Box::new(FakeBackend::new()), false);
    s.feeder().feed("héllo wörld".as_bytes(), Instant::now());
    h.settle();
    let apart = at(&mut h);
    assert_eq!(apart.len(), 10);
    assert_eq!(shaped(&mut h, &s, b"\r\nuvq"), 3, "off: a word a cell");
    look.update(|it| it.ligatures = true);
    h.settle();
    assert_eq!(shaped(&mut h, &s, b"\r\nxyz"), 1, "on: the token; the words above are found");
    assert_eq!(at(&mut h)[..10], apart[..], "the same columns");
}

#[test]
fn a_ligature_under_the_cursor_is_drawn_apart_in_either_phase_of_its_blink() {
    let (mut h, s, _) = term(Box::new(Arrows(FakeBackend::new())), true);
    // A blinking block, then the cursor back on `-`.
    s.feeder().feed(b"\x1b[1 qa->b\x1b[2G", Instant::now());
    h.settle();
    let node = h.query(|n| n.semantics.role == Role::TextInput)[0];
    for on in [true, false] {
        let w = h.tree.widget_mut::<TerminalWidget>(node).unwrap();
        (w.focused, w.caret_on) = (true, on);
        h.tree.mark_paint(node);
        assert_eq!(xs(&mut h).len(), 4, "the cursor's cell alone, the caret on: {on}");
    }
}
