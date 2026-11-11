//! What an app gets from the kit before any component: its fonts and its theme, and at the
//! root of a window the canvas and the text of the design language for everything in it.

use std::rc::Rc;

use quadra_app::App;
use quadra_kit::{dark, fonts, light, size, token};
use quadra_platform::{HeadlessPlatform, WindowOptions};
use quadra_render::NullRenderer;
use quadra_text::FakeBackend;
use quadra_ui::text;

#[test]
fn a_window_of_the_kit_has_the_canvas_and_the_text_of_the_theme() {
    let platform = HeadlessPlatform::new();
    let app = App::with(Rc::new(platform.clone()), NullRenderer::new(), Box::new(FakeBackend::new()));
    let mut app = quadra_kit::install(app, &light()).expect("the fonts load");
    let view = || quadra_kit::root().child(text("Hello")).child(text("small").size(size::TEXT_SM.size));
    let window = app.open_window(WindowOptions::new("kit").size(400.0, 300.0), view).unwrap();
    platform.step(&mut app);

    let tree = app.tree(window).unwrap();
    let (root, light) = (tree.root().unwrap(), light());
    assert_eq!(tree.paint_style(root).background, light.color(token::CANVAS));
    assert_eq!(tree.rect(root).size(), quadra_prim::Size::new(400.0, 300.0)); // it fills the window
    let (hello, small) = (tree.children(root)[0], tree.children(root)[1]);
    let at = tree.inherited(hello);
    assert_eq!((&*at.style.family, at.style.size, at.style.weight), (fonts::UI, 14.0, 400));
    assert_eq!((at.color, tree.rect(hello).height), (light.color(token::TEXT), 23.0)); // 14 on 23
    assert_eq!(tree.inherited(small).style.size, 12.0); // its own size, the rest from the root
    assert_ne!(dark().color(token::CANVAS), light.color(token::CANVAS));
}
