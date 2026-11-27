//! The tray on macOS (specs/16, "On macOS"): an item in the menu bar. As the menu bar's
//! items do, one with a menu opens it when it is clicked, and the menu's items come back to
//! the app through an inbox; one with no menu tells its click. Notifications are said by
//! the system's `osascript`.

use std::process::{Command, Stdio};

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSBitmapFormat, NSBitmapImageRep, NSDeviceRGBColorSpace, NSImage, NSMenu, NSMenuItem,
    NSStatusBar, NSStatusItem, NSVariableStatusItemLength,
};
use objc2_foundation::{NSCopying, NSSize, NSString, ns_string};
use quadra_async::Sender;

use crate::{TrayEvent, TrayImage};

/// How large a picture is in the menu bar, in points.
const SIDE: f64 = 18.0;

define_class!(
    // SAFETY: `NSObject` asks nothing of a subclass, and the class has no `Drop`.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = Sender<TrayEvent>]
    struct Target;

    /// What the item's button and its menu's items send their actions to.
    impl Target {
        #[unsafe(method(clicked:))]
        fn clicked(&self, _button: &AnyObject) {
            self.ivars().push(TrayEvent::Click);
        }

        #[unsafe(method(chosen:))]
        fn chosen(&self, item: &NSMenuItem) {
            self.ivars().push(TrayEvent::Item(item.tag().max(0) as usize));
        }
    }
);

/// A picture of straight RGBA as an image of the menu bar's size.
fn picture(of: &TrayImage) -> Option<Retained<NSImage>> {
    let (width, height) = (of.width as isize, of.height as isize);
    if of.rgba.is_empty() || of.rgba.len() != of.width as usize * of.height as usize * 4 {
        return None;
    }
    // SAFETY: no planes are given, so the bitmap allocates its own, of the rows it is told;
    // the name of the colour space is the system's constant.
    let bitmap = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bitmapFormat_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            width,
            height,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            NSBitmapFormat::AlphaNonpremultiplied,
            width * 4,
            32,
        )
    }?;
    let bytes = bitmap.bitmapData();
    if bytes.is_null() {
        return None;
    }
    // SAFETY: the bitmap's own bytes: `width * 4` a row and `height` rows, as it was told.
    unsafe { std::ptr::copy_nonoverlapping(of.rgba.as_ptr(), bytes, of.rgba.len()) };
    let image = NSImage::initWithSize(NSImage::alloc(), NSSize::new(SIDE, SIDE));
    image.addRepresentation(&bitmap);
    Some(image)
}

/// An item in the menu bar; dropping it takes it away.
pub struct MacTray {
    item: Retained<NSStatusItem>,
    /// What the actions are sent to, which the button and the menu's items do not hold.
    _target: Retained<Target>,
}

impl MacTray {
    pub(crate) fn add(
        image: Option<&TrayImage>,
        tooltip: &str,
        menu: &[&str],
        events: Sender<TrayEvent>,
    ) -> Option<MacTray> {
        let mtm = MainThreadMarker::new()?;
        let target = Target::alloc(mtm).set_ivars(events);
        // SAFETY: `NSObject`'s `init`, of an object just allocated.
        let target: Retained<Target> = unsafe { msg_send![super(target), init] };
        let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
        let button = item.button(mtm)?;
        // The app's own picture when it gives none: a copy, as its size is set.
        let picture = image.and_then(picture).or_else(|| {
            let own = NSApplication::sharedApplication(mtm).applicationIconImage()?;
            let own: Retained<NSImage> = own.copy();
            own.setSize(NSSize::new(SIDE, SIDE));
            Some(own)
        });
        button.setImage(picture.as_deref());
        button.setToolTip(Some(&NSString::from_str(tooltip)));
        if menu.is_empty() {
            // SAFETY: the target lives as long as the item: the tray holds both.
            unsafe {
                button.setTarget(Some(&target));
                button.setAction(Some(sel!(clicked:)));
            }
        } else {
            let items = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!(""));
            for (at, title) in menu.iter().enumerate() {
                let (title, chosen) = (NSString::from_str(title), Some(sel!(chosen:)));
                let (entry, no_key) = (NSMenuItem::alloc(mtm), ns_string!(""));
                // SAFETY: the action is one of the target's, which the item is given below.
                let entry =
                    unsafe { NSMenuItem::initWithTitle_action_keyEquivalent(entry, &title, chosen, no_key) };
                entry.setTag(at as isize);
                // SAFETY: as the button's above.
                unsafe { entry.setTarget(Some(&target)) };
                items.addItem(&entry);
            }
            item.setMenu(Some(&items));
        }
        Some(MacTray { item, _target: target })
    }

    // ponytail: said by `osascript`, so the notification carries Script Editor's picture and
    // not the app's; `UNUserNotificationCenter` (the app in a bundle, the user's leave asked
    // for) when an app on macOS has much to say.
    pub(crate) fn notify(&self, title: &str, text: &str) {
        let script =
            ["on run argv", "display notification (item 2 of argv) with title (item 1 of argv)", "end run"];
        let mut say = Command::new("/usr/bin/osascript");
        say.args(script.iter().flat_map(|line| ["-e", line])).args([title, text]);
        say.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        if let Ok(mut said) = say.spawn() {
            // Waited for, so no process is left that ended and was not asked how.
            std::thread::spawn(move || said.wait());
        }
    }

    pub(crate) fn set_tooltip(&mut self, tooltip: &str) {
        let button = MainThreadMarker::new().and_then(|mtm| self.item.button(mtm));
        if let Some(button) = button {
            button.setToolTip(Some(&NSString::from_str(tooltip)));
        }
    }
}

impl Drop for MacTray {
    fn drop(&mut self) {
        NSStatusBar::systemStatusBar().removeStatusItem(&self.item);
    }
}
