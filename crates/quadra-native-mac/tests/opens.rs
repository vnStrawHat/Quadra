//! Files the system asks the app to open (specs/23, "The app bundle"): what the application
//! is told by AppKit reaches who listens, as paths. A `main` of its own, as AppKit runs on
//! the main thread alone.

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
    use std::path::PathBuf;
    use std::sync::mpsc::channel;

    use objc2::{MainThreadMarker, msg_send};
    use objc2_app_kit::NSApplication;
    use objc2_foundation::{NSArray, NSString, NSURL};
    use quadra_native_mac::{MacPlatform, on_open_files};

    let _platform = MacPlatform::new().unwrap();
    let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
    let delegate = app.delegate().expect("the platform's delegate");
    let files = [NSURL::fileURLWithPath(&NSString::from_str("/tmp/tiếng việt.md"))];
    // SAFETY: the message AppKit sends its application's delegate, with what it takes.
    let _: () = unsafe {
        msg_send![&*delegate, application: &*app, openURLs: &*NSArray::from_retained_slice(&files)]
    };
    let (to, seen) = channel();
    on_open_files(move |files| to.send(files).unwrap());
    assert_eq!(seen.try_recv(), Ok(vec![PathBuf::from("/tmp/tiếng việt.md")]));
    // As the harness says it, for who counts what passed.
    println!("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out");
}
