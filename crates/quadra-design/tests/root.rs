//! What an app gets from the kit before any component: its fonts and its theme, and at the
//! root of a window the canvas and the text of the design language for everything in it.

use std::rc::Rc;

use quadra_app::App;
use quadra_design::{dark, fonts, light, size, token};
use quadra_platform::{HeadlessPlatform, WindowOptions};
use quadra_render::NullRenderer;
use quadra_text::FakeBackend;
use quadra_ui::text;

#[test]
fn a_window_of_the_kit_has_the_canvas_and_the_text_of_the_theme() {
    let platform = HeadlessPlatform::new();
    let app = App::with(Rc::new(platform.clone()), NullRenderer::new(), Box::new(FakeBackend::new()));
    let mut app = quadra_design::install(app, &light()).expect("the fonts load");
    let view = || quadra_design::root().child(text("Hello")).child(text("small").size(size::TEXT_SM.size));
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

#[test]
fn installed_for_the_system_the_theme_follows_its_appearance() {
    use quadra_platform::Appearance;
    let platform = HeadlessPlatform::new();
    platform.set_appearance(Appearance { dark: true, ..Appearance::default() });
    let app = App::with(Rc::new(platform.clone()), NullRenderer::new(), Box::new(FakeBackend::new()));
    let mut app = quadra_design::install_system(app).expect("the fonts load");
    let window = app.open_window(WindowOptions::new("kit").size(400.0, 300.0), quadra_design::root).unwrap();
    platform.step(&mut app);
    let canvas = |app: &App<NullRenderer>| {
        let tree = app.tree(window).unwrap();
        tree.paint_style(tree.root().unwrap()).background
    };
    assert_eq!(canvas(&app), dark().color(token::CANVAS));
    // The user turns the system light: the window is told, and the theme follows.
    platform.set_appearance(Appearance::default());
    platform.step(&mut app);
    platform.step(&mut app);
    assert_eq!(canvas(&app), light().color(token::CANVAS));
}

#[test]
fn under_high_contrast_the_roles_take_the_systems_colours() {
    use quadra_design::{Mode, high_contrast, system_theme};
    use quadra_platform::{Appearance, SystemColors};
    use quadra_prim::Color;
    // Night sky: black windows, white text, a light purple highlight.
    let colors = SystemColors {
        window: Color::rgb(0x000000),
        text: Color::rgb(0xFFFFFF),
        highlight: Color::rgb(0xD6B4FD),
        highlight_text: Color::rgb(0x2B2B2B),
        link: Color::rgb(0x8080FF),
        disabled: Color::rgb(0xA6A6A6),
        button: Color::rgb(0x000000),
        button_text: Color::rgb(0xFFFFFF),
    };
    let theme = high_contrast(&colors);
    assert_eq!(theme.mode, Mode::HighContrast);
    for (role, color) in [
        (token::CANVAS, colors.window),
        (token::TEXT, colors.text),
        (token::TEXT_3, colors.text),
        (token::LINE, colors.text),
        (token::ACCENT, colors.highlight),
        (token::TEXT_ON_ACCENT, colors.highlight_text),
        (token::BORDER_FOCUSED, colors.highlight),
        (token::TEXT_DISABLED, colors.disabled),
        (token::SYNTAX_KEYWORD, colors.text),
    ] {
        assert_eq!(theme.color(role), color);
    }
    // The system's theme is it while high contrast is on, and the dark one or the light one
    // otherwise; the system's accent is taken only when asked for.
    let on = Appearance { high_contrast: true, colors: Some(colors), ..Appearance::default() };
    assert_eq!(system_theme(&on), theme);
    let dark_on = Appearance { dark: true, accent: Some(Color::rgb(0x00FF00)), ..on };
    assert_eq!(system_theme(&Appearance { high_contrast: false, ..dark_on }), dark());
    let green = dark().with_accent(Color::rgb(0x00FF00));
    assert_eq!(
        (green.color(token::ACCENT), green.color(token::BORDER_FOCUSED)),
        (Color::rgb(0x00FF00), Color::rgb(0x00FF00))
    );
}
