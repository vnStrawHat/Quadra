//! The app's keymap as a signal (specs/09, "The keymap"): a view puts another, and every
//! window follows it.

use std::rc::Rc;

use quadra_app::App;
use quadra_platform::{HeadlessPlatform, PlatformEvent, WindowOptions};
use quadra_reactive::signal;
use quadra_render::NullRenderer;
use quadra_text::FakeBackend;
use quadra_view::text;

fn app(platform: &HeadlessPlatform) -> App<NullRenderer> {
    App::with(Rc::new(platform.clone()), NullRenderer::new(), Box::new(FakeBackend::new()))
}

#[test]
fn a_view_puts_another_keymap_and_the_windows_follow() {
    use quadra_platform::{Key, Mods};
    use quadra_tree::{Action, Keymap};
    use quadra_view::div;

    struct Hit;
    impl Action for Hit {
        const NAME: &'static str = "count::hit";
    }

    let platform = HeadlessPlatform::new();
    let mut first = Keymap::new();
    first.bind("ctrl-equal", Hit).unwrap();
    let mut app = app(&platform).keymap(first);
    let (count, keys) = (signal(0), std::rc::Rc::new(std::cell::Cell::new(None)));
    let kept = keys.clone();
    let view = move || {
        kept.set(Some(quadra_app::app_keymap()));
        div().on_action::<Hit>(move |_| count.update(|n| *n += 1)).child(text("keys"))
    };
    let window = app.open_window(WindowOptions::new("a"), view).unwrap();
    let press = |k| {
        for down in [true, false] {
            platform
                .push(window, PlatformEvent::Key { key: k, code: 0, mods: Mods::CTRL, down, repeat: false });
        }
    };
    press(Key::Equal);
    platform.step(&mut app);
    assert_eq!(count.get_untracked(), 1);
    let mut second = Keymap::new();
    second.bind("ctrl-minus", Hit).unwrap();
    keys.get().unwrap().set_always(std::rc::Rc::new(second));
    press(Key::Equal);
    press(Key::Minus);
    platform.step(&mut app);
    assert_eq!(count.get_untracked(), 2, "the old keys are gone, the new ones work");
}
