//! The item in the menu bar (specs/16, "On macOS"), on the main thread, as an application's
//! code is. On the CI's Mac alone (`CI` is set there): no test puts an item in a person's
//! menu bar.

#[cfg(target_os = "macos")]
fn main() {
    use quadra_shell::{MacShell, Tray, TrayEvent, TrayImage};

    if std::env::var_os("CI").is_none() {
        println!("tray_mac: not the CI's Mac, nothing is added");
        return;
    }
    let events = quadra_async::inbox::<TrayEvent>();
    let image = TrayImage { width: 16, height: 16, rgba: vec![255; 16 * 16 * 4] };
    let menu = ["Show", "Quit"];
    let mut with_menu =
        MacShell.add(Some(&image), "Quadra", &menu, events.sender()).expect("an item with a menu");
    with_menu.set_tooltip("Quadra, again");
    let plain = MacShell.add(None, "Quadra", &[], events.sender()).expect("an item with the app's picture");
    assert!(MacShell.add(Some(&TrayImage { rgba: vec![0; 5], ..image }), "", &[], events.sender()).is_some());
    with_menu.notify("Quadra", "A test said this.");
    drop((with_menu, plain));
    println!("tray_mac: items added to the menu bar and taken away");
}

#[cfg(not(target_os = "macos"))]
fn main() {}
