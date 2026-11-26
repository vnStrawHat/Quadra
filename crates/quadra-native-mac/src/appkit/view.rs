//! A window's content (specs/10): the one view the scene is drawn in, which tells the
//! handler what the pointer and the keys do in it. Its points are counted from its top left.

use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSAppearanceCustomization, NSEvent, NSEventModifierFlags, NSResponder, NSTrackingArea,
    NSTrackingAreaOptions, NSView,
};
use objc2_foundation::{NSArray, NSObject, NSObjectProtocol, NSRect, NSSize, NSString};
use quadra_platform::{HitRegion, MouseButton, PlatformEvent, PointerKind};
use quadra_prim::{Point, Size};

use super::window::State;
use super::{keys, look};

define_class!(
    // SAFETY: `NSView` is made to be subclassed, and the class has no `Drop`.
    #[unsafe(super(NSView, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Rc<State>]
    pub(crate) struct View;

    impl View {
        /// From the top left, as the scene is.
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        /// The keys are its own.
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            true
        }

        /// A press that brings the window forward is a press in it too.
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool {
            true
        }

        #[unsafe(method(setFrameSize:))]
        fn set_frame_size(&self, size: NSSize) {
            // SAFETY: the superclass's own method, with the argument this one was given.
            let _: () = unsafe { msg_send![super(self), setFrameSize: size] };
            self.resized();
        }

        /// The window is on a display of another scale.
        #[unsafe(method(viewDidChangeBackingProperties))]
        fn backing_changed(&self) {
            // SAFETY: as above.
            let _: () = unsafe { msg_send![super(self), viewDidChangeBackingProperties] };
            self.resized();
        }

        /// The system, or the app for this window, changed how apps look.
        #[unsafe(method(viewDidChangeEffectiveAppearance))]
        fn appearance_changed(&self) {
            // SAFETY: as above.
            let _: () = unsafe { msg_send![super(self), viewDidChangeEffectiveAppearance] };
            self.ivars().send(PlatformEvent::Appearance(look::appearance(&self.effectiveAppearance())));
        }

        /// AppKit asks where the pointer has which shape: the one the app set, over all of
        /// the view.
        #[unsafe(method(resetCursorRects))]
        fn reset_cursor_rects(&self) {
            self.addCursorRect_cursor(self.bounds(), &self.ivars().cursor.borrow());
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            if !self.on_caption(event) {
                self.pointer(event, PointerKind::Down, MouseButton::Left);
            }
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) {
            self.pointer(event, PointerKind::Up, MouseButton::Left);
        }

        #[unsafe(method(rightMouseDown:))]
        fn right_down(&self, event: &NSEvent) {
            self.pointer(event, PointerKind::Down, MouseButton::Right);
        }

        #[unsafe(method(rightMouseUp:))]
        fn right_up(&self, event: &NSEvent) {
            self.pointer(event, PointerKind::Up, MouseButton::Right);
        }

        #[unsafe(method(otherMouseDown:))]
        fn other_down(&self, event: &NSEvent) {
            self.pointer(event, PointerKind::Down, other(event));
        }

        #[unsafe(method(otherMouseUp:))]
        fn other_up(&self, event: &NSEvent) {
            self.pointer(event, PointerKind::Up, other(event));
        }

        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, event: &NSEvent) {
            self.pointer(event, PointerKind::Move, MouseButton::None);
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            self.pointer(event, PointerKind::Move, MouseButton::None);
        }

        #[unsafe(method(rightMouseDragged:))]
        fn right_dragged(&self, event: &NSEvent) {
            self.pointer(event, PointerKind::Move, MouseButton::None);
        }

        #[unsafe(method(otherMouseDragged:))]
        fn other_dragged(&self, event: &NSEvent) {
            self.pointer(event, PointerKind::Move, MouseButton::None);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, event: &NSEvent) {
            self.pointer(event, PointerKind::Leave, MouseButton::None);
        }

        /// Lines from a wheel, points from a trackpad; towards the content's start is
        /// positive on both axes, as AppKit counts it too.
        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, event: &NSEvent) {
            let delta = Point::new(event.scrollingDeltaX() as f32, event.scrollingDeltaY() as f32);
            let (precise, at, mods) =
                (event.hasPreciseScrollingDeltas(), self.place(event), keys::mods(event.modifierFlags()));
            self.ivars().send(PlatformEvent::Wheel { delta, precise, at, mods });
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            self.key(event, true);
            // What the key types comes through the system's handling of keys, as
            // `insertText:`. With Command held it types nothing: it is a shortcut.
            if !event.modifierFlags().contains(NSEventModifierFlags::Command) {
                self.interpretKeyEvents(&NSArray::from_slice(&[event]));
            }
        }

        #[unsafe(method(keyUp:))]
        fn key_up(&self, event: &NSEvent) {
            self.key(event, false);
        }

        /// A modifier key went down or up: it is down when its flag is held.
        #[unsafe(method(flagsChanged:))]
        fn flags_changed(&self, event: &NSEvent) {
            let held = keys::flag_of(keys::key(event.keyCode()));
            if let Some(flag) = held {
                self.key(event, event.modifierFlags().contains(flag));
            }
        }

        /// Text a key typed: a string, or one with attributes that an input method made.
        #[unsafe(method(insertText:))]
        fn insert_text(&self, text: &AnyObject) {
            let text = match text.downcast_ref::<NSString>() {
                Some(plain) => plain.to_string(),
                // SAFETY: what is no string here is an attributed one, which has a string.
                None => unsafe {
                    let plain: Retained<NSString> = msg_send![text, string];
                    plain.to_string()
                },
            };
            let typed: String = text.chars().filter(|c| !c.is_control()).collect();
            if !typed.is_empty() {
                self.ivars().send(PlatformEvent::Text(typed));
            }
        }

        /// A key that types nothing (an arrow, Enter): it came as a key already, and is not
        /// answered with the system's beep.
        #[unsafe(method(doCommandBySelector:))]
        fn do_command(&self, _command: Sel) {}
    }

    unsafe impl NSObjectProtocol for View {}
);

/// The button of a press that is neither the left nor the right one.
fn other(event: &NSEvent) -> MouseButton {
    match event.buttonNumber() {
        3 => MouseButton::Back,
        4 => MouseButton::Forward,
        _ => MouseButton::Middle,
    }
}

impl View {
    pub fn new(mtm: MainThreadMarker, state: Rc<State>, frame: NSRect) -> Retained<View> {
        let view = View::alloc(mtm).set_ivars(state);
        // SAFETY: `NSView`'s initializer, of an object whose instance variables are set.
        let view: Retained<View> = unsafe { msg_send![super(view), initWithFrame: frame] };
        // Moves and leaves of the pointer over all of the view, whatever window has the keys.
        let options = NSTrackingAreaOptions::MouseEnteredAndExited
            | NSTrackingAreaOptions::MouseMoved
            | NSTrackingAreaOptions::ActiveAlways
            | NSTrackingAreaOptions::InVisibleRect;
        // SAFETY: an area that follows the view's visible part and tells the view itself.
        let area = unsafe {
            NSTrackingArea::initWithRect_options_owner_userInfo(
                NSTrackingArea::alloc(),
                frame,
                options,
                Some(&view),
                None,
            )
        };
        view.addTrackingArea(&area);
        view
    }

    /// Where an event is, in the view's own points.
    fn place(&self, event: &NSEvent) -> Point {
        let at = self.convertPoint_fromView(event.locationInWindow(), None);
        Point::new(at.x as f32, at.y as f32)
    }

    /// A press on what the app says is its title bar: the window is dragged by it, and
    /// zoomed by a second press. The press is the system's then, not the app's.
    fn on_caption(&self, event: &NSEvent) -> bool {
        let state = self.ivars();
        let (Some(shared), Some(window)) = (state.shared.upgrade(), self.window()) else { return false };
        if !state.custom_frame || shared.hit_test(state.id, self.place(event)) != HitRegion::Caption {
            return false;
        }
        // ponytail: a second press zooms; what the user chose for it in the system's
        // settings (minimize, nothing) when someone asks.
        if event.clickCount() == 2 {
            window.zoom(None)
        } else {
            window.performWindowDragWithEvent(event)
        }
        true
    }

    fn pointer(&self, event: &NSEvent, kind: PointerKind, button: MouseButton) {
        let (at, mods) = (self.place(event), keys::mods(event.modifierFlags()));
        self.ivars().send(PlatformEvent::Pointer { kind, button, at, mods });
    }

    fn key(&self, event: &NSEvent, down: bool) {
        let code = event.keyCode();
        // A change of the modifiers is no key that repeats, and may not be asked whether it does.
        let repeat = down && keys::flag_of(keys::key(code)).is_none() && event.isARepeat();
        let mods = keys::mods(event.modifierFlags());
        self.ivars().send(PlatformEvent::Key { key: keys::key(code), code: code.into(), mods, down, repeat });
    }

    /// The view's size or its window's scale changed. The loop does not turn while the user
    /// drags the window's edge, so the frame the app asks for is given at once.
    fn resized(&self) {
        let (size, state) = (self.bounds().size, self.ivars());
        let scale = self.window().map_or(1.0, |window| window.backingScaleFactor() as f32);
        let size = Size::new(size.width as f32, size.height as f32);
        if state.told.replace((size, scale)) == (size, scale) {
            return;
        }
        state.send(PlatformEvent::Resize { size, scale });
        if let Some(shared) = state.shared.upgrade() {
            shared.frames();
        }
    }
}
