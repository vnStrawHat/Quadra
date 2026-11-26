//! The menu bar (specs/23, "The menu bar"): what a Mac's user finds in every app's, with the
//! keys the system keeps for it. The app's own menu hides and quits, the window's minimizes
//! and zooms. An app's own menus are not in it yet.

use std::rc::{Rc, Weak};

use objc2::rc::Retained;
use objc2::runtime::{ProtocolObject, Sel};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSApplicationDelegate, NSApplicationTerminateReply, NSEventModifierFlags, NSMenu,
    NSMenuItem,
};
use objc2_foundation::{NSArray, NSObject, NSObjectProtocol, NSProcessInfo, NSString, NSURL};

use super::Shared;

define_class!(
    // SAFETY: `NSObject` asks nothing of a subclass, and the class has no `Drop`.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = Weak<Shared>]
    pub(crate) struct AppDelegate;

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        /// Quit ends no process: every window is asked to close, as its close button asks,
        /// and the app, which decides, ends when it has none left.
        #[unsafe(method(applicationShouldTerminate:))]
        fn should_terminate(&self, _app: &NSApplication) -> NSApplicationTerminateReply {
            if let Some(shared) = self.ivars().upgrade() {
                shared.close_all();
            }
            NSApplicationTerminateReply::TerminateCancel
        }

        /// Files opened from the Finder or by `open`, for who listens (`opens.rs`).
        #[unsafe(method(application:openURLs:))]
        fn open_urls(&self, _app: &NSApplication, urls: &NSArray<NSURL>) {
            crate::opens::opened(urls.iter().filter_map(|url| crate::opens::file_path(&url)).collect());
        }
    }
);

/// An item that sends an action to whatever of AppKit's answers it first, with its key.
fn item(mtm: MainThreadMarker, title: &str, action: Sel, key: &str) -> Retained<NSMenuItem> {
    let (title, key) = (NSString::from_str(title), NSString::from_str(key));
    // SAFETY: the actions given here are AppKit's own, of its application and its windows.
    unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(NSMenuItem::alloc(mtm), &title, Some(action), &key)
    }
}

/// A menu of the bar, under a title.
fn menu(
    mtm: MainThreadMarker,
    bar: &NSMenu,
    title: &str,
    items: &[Retained<NSMenuItem>],
) -> Retained<NSMenu> {
    let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(title));
    for item in items {
        menu.addItem(item);
    }
    let holder = NSMenuItem::new(mtm);
    holder.setSubmenu(Some(&menu));
    bar.addItem(&holder);
    menu
}

/// Puts the menu bar up and has Quit ask the app. The application holds its delegate
/// weakly: who calls keeps what is returned.
// ponytail: the titles in English; in the app's language with the app's own menus.
pub(crate) fn install(mtm: MainThreadMarker, shared: &Rc<Shared>) -> Retained<AppDelegate> {
    let app = NSApplication::sharedApplication(mtm);
    let delegate = AppDelegate::alloc(mtm).set_ivars(Rc::downgrade(shared));
    // SAFETY: `NSObject`'s initializer, of an object whose instance variables are set.
    let delegate: Retained<AppDelegate> = unsafe { msg_send![super(delegate), init] };
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));

    let name = NSProcessInfo::processInfo().processName().to_string();
    let bar = NSMenu::new(mtm);
    let others = item(mtm, "Hide Others", sel!(hideOtherApplications:), "h");
    others.setKeyEquivalentModifierMask(NSEventModifierFlags::Option | NSEventModifierFlags::Command);
    // The first menu is the app's own, whatever its title: the system names it.
    menu(
        mtm,
        &bar,
        &name,
        &[
            item(mtm, &format!("Hide {name}"), sel!(hide:), "h"),
            others,
            item(mtm, "Show All", sel!(unhideAllApplications:), ""),
            NSMenuItem::separatorItem(mtm),
            item(mtm, &format!("Quit {name}"), sel!(terminate:), "q"),
        ],
    );
    let windows =
        [item(mtm, "Minimize", sel!(performMiniaturize:), "m"), item(mtm, "Zoom", sel!(performZoom:), "")];
    // The system lists the app's windows under this one.
    app.setWindowsMenu(Some(&menu(mtm, &bar, "Window", &windows)));
    app.setMainMenu(Some(&bar));
    delegate
}
