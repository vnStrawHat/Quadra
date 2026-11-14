//! The app's language (specs/15, "Language"): strings looked up in the locale, then the
//! default one, then as their keys; plurals by language; formats of Vietnamese and English;
//! a view that says a string says it again when the language changes; the time where the
//! user is.

use std::cell::Cell;
use std::rc::Rc;

use quadra_app::time::{Date, Time, local_now, unix_now};
use quadra_app::{App, I18n, Locale, language};
use quadra_platform::{HeadlessPlatform, Platform, WindowOptions};
use quadra_render::NullRenderer;
use quadra_text::FakeBackend;
use quadra_ui::{column, text};

fn strings() -> Rc<I18n> {
    let i18n = I18n::new(Locale::en());
    let en = r#"{ "files.count": { "one": "{n} file", "other": "{n} files" }, "hello": "Hello, {name}", "only.en": "Only here" }"#;
    i18n.add("en", en).unwrap();
    i18n.add("vi", r#"{ "files.count": "{n} tệp", "hello": "Xin chào, {name}" }"#).unwrap();
    i18n
}

#[test]
fn strings_fall_back_to_the_default_language_then_to_their_keys() {
    let i18n = strings();
    assert_eq!(i18n.text("files.count", &[("n", 1.into())]), "1 file");
    assert_eq!(i18n.text("files.count", &[("n", 1200.into())]), "1,200 files");
    assert_eq!(i18n.text("hello", &[("name", "Lan".into())]), "Hello, Lan");
    i18n.locale().set(Locale::new("vi-VN"));
    // Vietnamese has one form: no plurals.
    assert_eq!(i18n.text("files.count", &[("n", 1200.into())]), "1.200 tệp");
    assert_eq!(i18n.text("only.en", &[]), "Only here");
    assert_eq!(i18n.text("not.anywhere", &[]), "not.anywhere");
    assert!(i18n.add("fr", "[1, 2]").is_err());
}

#[test]
fn numbers_sizes_dates_and_times_are_in_the_form_of_the_language() {
    let (vi, en) = (Locale::vi(), Locale::new("en-GB"));
    assert_eq!(
        (vi.number(1234567.891, 2), en.number(1234567.891, 2)),
        ("1.234.567,89".into(), "1,234,567.89".into())
    );
    assert_eq!((vi.number(-0.4, 0), en.number(-12.5, 1)), ("0".into(), "-12.5".into()));
    assert_eq!((vi.size(1536 * 1024), en.size(1536 * 1024)), ("1,5 MB".into(), "1.5 MB".into()));
    assert_eq!(
        (en.size(512), en.size(2048), en.size(150 << 30)),
        ("512 B".into(), "2 KB".into(), "150 GB".into())
    );
    let day = Date::new(2026, 11, 4);
    assert_eq!((vi.date(day), en.date(day)), ("04/11/2026".into(), "11/4/2026".into()));
    assert_eq!(
        (vi.date_long(day), en.date_long(day)),
        ("4 tháng 11 năm 2026".into(), "November 4, 2026".into())
    );
    let (afternoon, midnight) = (Time { hour: 15, minute: 5, second: 0 }, Time::default());
    assert_eq!(
        (vi.time(afternoon), en.time(afternoon), en.time(midnight)),
        ("15:05".into(), "3:05 PM".into(), "12:00 AM".into())
    );
    assert_eq!(
        (vi.relative(-180), en.relative(-180), en.relative(3600)),
        ("3 phút trước".into(), "3 minutes ago".into(), "in 1 hour".into())
    );
    assert_eq!((vi.relative(20), en.relative(-86_400 * 2)), ("vừa xong".into(), "2 days ago".into()));
    assert_eq!(
        (vi.month(11), en.day(6), vi.day(6), vi.first_day(), en.first_day()),
        ("Tháng 11", "Su", "CN", 0, 6)
    );
    assert_eq!(Locale::new("fr").number(1000.0, 0), "1,000"); // a language of no form here is English
}

#[test]
fn a_view_says_its_strings_again_in_another_language_and_knows_the_local_time() {
    let platform = HeadlessPlatform::new();
    platform.set_utc_offset(7 * 3600);
    let app = App::with(Rc::new(platform.clone()), NullRenderer::new(), Box::new(FakeBackend::new()));
    let i18n = strings();
    let mut app = app.i18n(i18n.clone());
    let hour = Rc::new(Cell::new(None));
    let seen = hour.clone();
    let window = app
        .open_window(WindowOptions::new("i18n").size(300.0, 100.0), move || {
            seen.set(Some(local_now()));
            let lang = language();
            column().child(text(move || lang.t_with("hello", &[("name", "Lan".into())])))
        })
        .unwrap();
    platform.step(&mut app);
    let said = |app: &App<NullRenderer>| {
        let tree = app.tree(window).unwrap();
        tree.semantic_tree().nodes.iter().map(|n| n.semantics.name.clone()).collect::<Vec<_>>().join("|")
    };
    assert!(said(&app).contains("Hello, Lan"), "{}", said(&app));
    i18n.locale().set(Locale::vi());
    platform.waker().wake(); // an entry point: the app looks at what changed
    platform.step(&mut app);
    assert!(said(&app).contains("Xin chào, Lan"), "{}", said(&app));

    // Seven hours ahead of UTC, as the platform said.
    let local = hour.get().unwrap();
    let utc = quadra_app::time::DateTime::from_unix(unix_now(), 0);
    let ahead = (local.to_unix(0) - utc.to_unix(0) + 30) / 3600;
    assert_eq!(ahead, 7);
}
