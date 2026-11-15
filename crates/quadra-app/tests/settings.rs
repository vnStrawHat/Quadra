//! Settings (specs/15): signals, written half a second after the last change through a file
//! renamed over the old one; a file that is no JSON object moved aside; a migration.

use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use quadra_app::{App, AppDirs, Settings, log};
use quadra_platform::{HeadlessPlatform, Placement, Platform, PlatformEvent, WindowOptions};
use quadra_prim::Size;
use quadra_render::NullRenderer;
use quadra_text::FakeBackend;
use quadra_ui::column;
use serde_json::Value;

/// A folder of the test's own under `.tmp/`, empty.
fn folder(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/tests").join(name);
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn app(platform: &HeadlessPlatform, dirs: AppDirs) -> App<NullRenderer> {
    let app = App::with(Rc::new(platform.clone()), NullRenderer::new(), Box::new(FakeBackend::new()));
    app.dirs(dirs)
}

fn read(path: &PathBuf) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn a_setting_is_a_signal_written_half_a_second_after_the_last_change() {
    let (platform, dirs) = (HeadlessPlatform::new(), AppDirs::under(&folder("settings")));
    let file = dirs.config.join("settings.json");
    let mut app = app(&platform, dirs.clone());
    let settings = app.settings();
    let size = settings.value("editor.font_size", 15.0f64);
    let tabs = settings.value("editor.tabs", vec!["a.rs".to_string()]);
    // A default is not written; two askers of one key share its signal.
    assert!(!file.exists());
    assert_eq!(settings.value("editor.font_size", 0.0f64).get_untracked(), 15.0);

    size.set(16.0);
    platform.advance(Duration::from_millis(300), &mut app);
    tabs.set(vec!["a.rs".into(), "b.rs".into()]);
    platform.advance(Duration::from_millis(300), &mut app);
    assert!(!file.exists()); // still changing
    platform.advance(Duration::from_millis(300), &mut app);
    let written = read(&file);
    assert_eq!(
        (written["editor.font_size"].as_f64(), written["editor.tabs"][1].as_str()),
        (Some(16.0), Some("b.rs"))
    );
    assert!(!dirs.config.join("settings.json.tmp").exists());

    // Back to where it began is a change too; and the next app reads what was written.
    size.set(15.0);
    settings.save_now();
    let again = Settings::open(&file);
    assert_eq!(again.value("editor.font_size", 0.0f64).get_untracked(), 15.0);
    assert_eq!(again.value("editor.tabs", Vec::<String>::new()).get_untracked().len(), 2);
    // A value of another shape is the default.
    assert!(!again.value("editor.tabs", false).get_untracked());
}

#[test]
fn a_broken_file_is_moved_aside_and_an_old_one_is_brought_up_to_date() {
    let dir = folder("broken");
    let file = dir.join("settings.json");
    fs::write(&file, "{ not json").unwrap();
    let settings = Settings::open(&file);
    assert!(!file.exists() && dir.join("settings.json.corrupt").exists());
    assert!(settings.value("theme", "dark".to_string()).get_untracked() == "dark");
    assert!(log::recent(50).iter().any(|line| line.contains("WARN settings") && line.contains("moved to")));

    // Version 1 called the size "fontSize"; version 2 knows it as "editor.font_size".
    fs::write(&file, r#"{ "version": 1, "fontSize": 18 }"#).unwrap();
    let settings = Settings::open(&file);
    settings.migrate(2, |from, map| {
        if from < 2
            && let Some(size) = map.remove("fontSize")
        {
            map.insert("editor.font_size".into(), size);
        }
    });
    assert_eq!(settings.value("editor.font_size", 14u32).get_untracked(), 18);
    assert_eq!(read(&file)["version"], 2);
    // A file of a newer version is read and never written over.
    let newer = Settings::open(&file);
    newer.migrate(1, |_, _| unreachable!());
    newer.value("editor.font_size", 14u32).set(20);
    newer.save_now();
    assert_eq!(read(&file)["editor.font_size"], 18);
}

#[test]
fn a_remembered_window_opens_where_it_was_when_it_closed() {
    let (platform, dirs) = (HeadlessPlatform::new(), AppDirs::under(&folder("window")));
    let options = || WindowOptions::new("main").size(400.0, 300.0).remember("main");
    let mut first = app(&platform, dirs.clone());
    let window = first.open_window(options(), column).unwrap();
    platform.step(&mut first);
    // The user moves it and makes it bigger, then closes it.
    platform.move_window(window, 120, 80);
    platform.resize(window, Size::new(640.0, 480.0), 1.0);
    platform.push(window, PlatformEvent::CloseRequested);
    platform.step(&mut first);
    assert_eq!(first.window_count(), 0);
    first.settings().save_now();
    let kept = read(&dirs.config.join("settings.json"))["window.main"].clone();
    assert_eq!(
        kept,
        serde_json::json!({ "x": 120, "y": 80, "width": 640, "height": 480, "maximized": false })
    );

    // The next run opens it there.
    let platform = HeadlessPlatform::new();
    let mut app = app(&platform, dirs);
    let window = app.open_window(options(), column).unwrap();
    let place = Placement { x: 120, y: 80, width: 640, height: 480, maximized: false };
    assert_eq!(platform.placement(window), Some(place));
}

#[test]
fn a_window_still_open_when_the_app_ends_is_remembered_too() {
    let (platform, dirs) = (HeadlessPlatform::new(), AppDirs::under(&folder("window-open")));
    let mut app = app(&platform, dirs.clone());
    let window =
        app.open_window(WindowOptions::new("main").size(400.0, 300.0).remember("main"), column).unwrap();
    platform.step(&mut app);
    platform.move_window(window, 30, 40);
    // The app ends with the window open (a tray's Quit, the session's end).
    platform.quit();
    app.run().unwrap();
    let kept = read(&dirs.config.join("settings.json"))["window.main"].clone();
    assert_eq!((kept["x"].as_i64(), kept["y"].as_i64()), (Some(30), Some(40)));
}
