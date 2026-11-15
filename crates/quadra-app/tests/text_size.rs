//! The text size (specs/12, "Text size"): a window zoomed 1.5 times lays out and is hit as
//! one 1.5 times smaller at a scale 1.5 times larger; the system's text size zooms it more
//! until the user says it does not; a reader is told the window's own pixels.

use std::cell::Cell;
use std::rc::Rc;

use quadra_app::{App, SystemText, TextSize, text_size};
use quadra_platform::{
    Appearance, HeadlessPlatform, Mods, MouseButton, Platform, PlatformEvent, PointerKind, WindowId,
    WindowOptions,
};
use quadra_prim::{Point, Rect, Size};
use quadra_reactive::{Signal, signal};
use quadra_render::NullRenderer;
use quadra_text::FakeBackend;
use quadra_ui::{column, div, text};

struct Page {
    platform: HeadlessPlatform,
    app: App<NullRenderer>,
    window: WindowId,
    clicks: Signal<u32>,
    size: Rc<Cell<Option<Signal<TextSize>>>>,
}

/// A window of `size` at `scale`, zoomed `zoom` times: a button of 100 by 40 at its top left.
fn page(size: Size, scale: f32, zoom: f32) -> Page {
    let platform = HeadlessPlatform::new();
    let app = App::with(Rc::new(platform.clone()), NullRenderer::new(), Box::new(FakeBackend::new()));
    let mut app = app.text_size(TextSize { zoom, ..TextSize::default() });
    let (clicks, size_of) = (signal(0), Rc::new(Cell::new(None)));
    let found = size_of.clone();
    let view = move || {
        found.set(Some(text_size()));
        let button = div().size(100.0, 40.0).on_click(move |_| clicks.update(|n| *n += 1));
        column().child(button).child(text("zoomed"))
    };
    let options = WindowOptions::new("zoom").size(size.width, size.height);
    let window = app.open_window(options, view).unwrap();
    platform.resize(window, size, scale);
    platform.step(&mut app);
    Page { platform, app, window, clicks, size: size_of }
}

impl Page {
    fn root(&self) -> Rect {
        let tree = self.app.tree(self.window).unwrap();
        tree.window_rect(tree.root().unwrap())
    }

    fn click(&mut self, x: f32, y: f32) -> u32 {
        for kind in [PointerKind::Move, PointerKind::Down, PointerKind::Up] {
            let at = Point::new(x, y);
            self.platform.push(
                self.window,
                PlatformEvent::Pointer { kind, button: MouseButton::Left, at, mods: Mods::NONE },
            );
        }
        self.platform.step(&mut self.app);
        self.clicks.get_untracked()
    }

    fn settle(&mut self) {
        self.platform.waker().wake();
        while self.platform.step(&mut self.app) {}
    }
}

#[test]
fn a_zoomed_window_is_a_smaller_one_at_a_larger_scale() {
    let mut zoomed = page(Size::new(300.0, 210.0), 1.0, 1.5);
    let plain = page(Size::new(200.0, 140.0), 1.5, 1.0);
    assert_eq!(zoomed.root(), plain.root());
    assert_eq!(zoomed.root().size(), Size::new(200.0, 140.0));
    // The button is 150 window pixels wide: hit at 140, not at 160.
    assert_eq!(zoomed.click(140.0, 30.0), 1);
    assert_eq!(zoomed.click(160.0, 30.0), 1);

    // A reader is told the tree's pixels and the scale that makes them device pixels.
    zoomed.platform.push(zoomed.window, PlatformEvent::SemanticsWanted);
    zoomed.settle();
    let told = zoomed.platform.take_semantics(zoomed.window);
    let root = told.last().unwrap().nodes.iter().find(|n| n.id == told[0].root).unwrap().bounds;
    assert_eq!((root.size(), told.last().unwrap().scale), (Size::new(200.0, 140.0), Some(1.5)));
}

#[test]
fn the_systems_text_size_zooms_until_the_user_says_otherwise() {
    let mut p = page(Size::new(300.0, 240.0), 1.0, 1.0);
    p.platform.set_appearance(Appearance { text_scale: 1.25, ..Appearance::default() });
    p.platform
        .push(p.window, PlatformEvent::Appearance(Appearance { text_scale: 1.25, ..Appearance::default() }));
    p.settle();
    assert_eq!(p.root().size(), Size::new(240.0, 192.0));
    let size = p.size.get().unwrap();
    size.set(TextSize { system: SystemText::None, ..size.get_untracked() });
    p.settle();
    assert_eq!(p.root().size(), Size::new(300.0, 240.0));
    // Held between 0.5 and 4.
    assert_eq!(TextSize { zoom: 9.0, text: 0.1, system: SystemText::Text }.factors(2.0), (4.0, 0.5));
}

#[test]
fn text_alone_grows_when_a_view_asks_while_the_app_runs() {
    use quadra_ui::{by_text, uniform_list};

    let platform = HeadlessPlatform::new();
    let mut app = App::with(Rc::new(platform.clone()), NullRenderer::new(), Box::new(FakeBackend::new()));
    let found = Rc::new(Cell::new(None));
    let size = found.clone();
    let view = move || {
        size.set(Some(text_size()));
        let rows = uniform_list(|| 10, 20.0, signal(0.0), |at| text(move || format!("row {}", at.get())));
        column()
            .child(text("words"))
            .child(div().height(by_text(28.0)))
            .child(div().height(200.0).child(rows))
    };
    let window = app.open_window(WindowOptions::new("text").size(400.0, 400.0), view).unwrap();
    platform.step(&mut app);
    let look = |app: &App<NullRenderer>| {
        let tree = app.tree(window).unwrap();
        let root = tree.root().unwrap();
        let [words, boxed, list] = [0, 1, 2].map(|i| tree.children(root)[i]);
        let rows: Vec<f32> = tree
            .semantic_tree()
            .nodes
            .iter()
            .filter(|n| n.semantics.name.starts_with("row "))
            .map(|n| tree.window_rect(tree.node_of(n.id).unwrap()).y - tree.window_rect(list).y)
            .take(2)
            .collect();
        (tree.inherited(words).style.size, tree.rect(boxed).height, rows)
    };
    assert_eq!(look(&app), (14.0, 28.0, vec![0.0, 20.0]));
    let size = found.get().unwrap();
    size.set(TextSize { text: 1.5, ..size.get_untracked() });
    platform.waker().wake();
    while platform.step(&mut app) {}
    assert_eq!(look(&app), (21.0, 42.0, vec![0.0, 30.0]));
}
