//! The fake shell (specs/16, "Traits and fakes"): views find it where they find the OS's, it
//! answers what the test queued and keeps what it was asked. Here a window that is not saved
//! asks before it closes.

use std::rc::Rc;

use quadra_app::App;
use quadra_platform::{HeadlessPlatform, Platform, PlatformEvent, WindowOptions};
use quadra_render::NullRenderer;
use quadra_shell::{
    Answer, Dialogs, FakeShell, FileDialog, Message, MessageKind, Opener, Tray, TrayEvent, dialogs, opener,
};
use quadra_text::FakeBackend;
use quadra_view::{column, window};

#[test]
fn a_window_not_saved_asks_the_user_before_it_closes() {
    let (platform, shell) = (HeadlessPlatform::new(), FakeShell::new());
    let app = App::with(Rc::new(platform.clone()), NullRenderer::new(), Box::new(FakeBackend::new()));
    let app = app.provide::<Rc<dyn Dialogs>>(shell.clone()).provide::<Rc<dyn Opener>>(shell.clone());
    let mut app = app;
    let id = app
        .open_window(WindowOptions::new("doc"), || {
            let (dialogs, this) = (dialogs(), window());
            column().on_close(move || {
                let asked = Message::new(MessageKind::Question, "Save the changes?", "doc.txt");
                match dialogs.message(&asked) {
                    Answer::No => this.close_now(), // closed without saving
                    Answer::Yes => {
                        let to = dialogs.save_file(&FileDialog::new("Save as").start("doc.txt"));
                        if to.is_some() {
                            this.close_now();
                        }
                    }
                    _ => {}
                }
                false
            })
        })
        .unwrap();
    platform.step(&mut app);
    let close = |app: &mut App<NullRenderer>| {
        platform.push(id, PlatformEvent::CloseRequested);
        platform.step(app);
        platform.waker().wake();
        platform.step(app);
    };

    close(&mut app); // nothing queued: Cancel
    shell.answer(Answer::Yes); // then the save dialog is given up
    close(&mut app);
    assert_eq!(app.window_count(), 1);
    shell.answer(Answer::Yes);
    shell.choose(&["D:/doc.txt"]);
    close(&mut app);
    assert_eq!(app.window_count(), 0);
    let message = "message: Save the changes?";
    assert_eq!(shell.asked(), [message, message, "save_file: Save as", message, "save_file: Save as"]);
}

#[test]
fn what_is_opened_is_kept_and_the_dialogs_answer_what_was_queued() {
    let shell = FakeShell::new();
    shell.choose(&["a.png", "b.png"]);
    let images = FileDialog::new("Images").filter("Images", "*.png;*.jpg").multiple();
    assert_eq!(shell.open_files(&images).len(), 2);
    assert!(shell.open_files(&images).is_empty()); // given up
    assert_eq!(shell.message(&Message::new(MessageKind::Error, "Failed", "..")), Answer::Ok);
    let opened = || {
        quadra_reactive::Scope::child().run(|| {
            shell.provide();
            opener()
        })
    };
    assert!(opened().open("https://example.com"));
    assert_eq!(shell.asked().last().unwrap(), "open: https://example.com");
}

#[test]
fn the_tray_tells_the_app_what_the_user_did() {
    let shell = FakeShell::new();
    let events = quadra_async::inbox();
    let mut icon = shell.add(None, "Sync", &["Open", "Quit"], events.sender()).unwrap();
    icon.notify("Synced", "3 files");
    icon.set_tooltip("Sync: idle");
    shell.click_tray(TrayEvent::Item(1));
    let asked = ["tray: Sync [Open, Quit]", "notify: Synced: 3 files", "tooltip: Sync: idle"];
    assert_eq!(shell.asked(), asked);
    let seen = Rc::new(std::cell::RefCell::new(Vec::new()));
    let platform = HeadlessPlatform::new();
    let mut app = App::with(Rc::new(platform.clone()), NullRenderer::new(), Box::new(FakeBackend::new()));
    let got = seen.clone();
    app.open_window(WindowOptions::new("tray"), move || {
        events.on_data(move |burst| got.borrow_mut().append(burst));
        column()
    })
    .unwrap();
    platform.waker().wake();
    platform.step(&mut app);
    assert_eq!(*seen.borrow(), [TrayEvent::Item(1)]);
}
