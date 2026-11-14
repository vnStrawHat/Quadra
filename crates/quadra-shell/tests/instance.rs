//! One instance (specs/16): the first to ask is the first; a later start, here of the same
//! process, hands it its folder and arguments, which arrive on the UI thread.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_app::App;
use quadra_platform::{HeadlessPlatform, Platform, WindowOptions};
use quadra_render::NullRenderer;
use quadra_shell::{Instance, Launch, single_instance};
use quadra_text::FakeBackend;
use quadra_ui::column;

#[test]
fn a_later_start_hands_the_first_its_arguments() {
    let id = format!("test-{}", std::process::id());
    let Instance::First(launches) = single_instance(&id, &[]) else { panic!("the first is the first") };
    let args = ["--open".to_string(), "Tệp.txt".to_string()];
    assert!(matches!(single_instance(&id, &args), Instance::Second));

    let platform = HeadlessPlatform::new();
    let mut app = App::with(Rc::new(platform.clone()), NullRenderer::new(), Box::new(FakeBackend::new()));
    let seen = Rc::new(RefCell::new(Vec::<Launch>::new()));
    let into = seen.clone();
    app.open_window(WindowOptions::new("first"), move || {
        launches.on_data(move |burst| into.borrow_mut().append(burst));
        column()
    })
    .unwrap();
    platform.waker().wake();
    platform.step(&mut app);
    let folder = std::env::current_dir().unwrap();
    assert_eq!(*seen.borrow(), [Launch { folder, args: args.to_vec() }]);
}
