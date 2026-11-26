//! A window's content (specs/10): the one view the scene is drawn in, which tells the
//! handler what the pointer and the keys do in it. Its points are counted from its top left.

use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject, Sel};
use objc2::{AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSAppearanceCustomization, NSDragOperation, NSDraggingInfo, NSEvent, NSResponder, NSTextInputClient,
    NSTrackingArea, NSTrackingAreaOptions, NSView,
};
use objc2_foundation::{
    NSArray, NSAttributedString, NSNotFound, NSObject, NSObjectProtocol, NSPoint, NSRange, NSRect, NSSize,
    NSString,
};
use quadra_platform::{MouseButton, PlatformEvent, PointerKind};
use quadra_prim::Point;

use super::window::State;
use super::{keys, look};

/// No range, as AppKit says it.
const NOWHERE: NSRange = NSRange { location: NSNotFound as usize, length: 0 };

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
                self.pointer(event, PointerKind::Down, self.left_down(event));
            }
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) {
            self.pointer(event, PointerKind::Up, self.left_up());
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
            self.pressed(event);
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

        // What another program drags over the view (`drops.rs`).
        #[unsafe(method(draggingEntered:))]
        fn dragging_entered(&self, info: &ProtocolObject<dyn NSDraggingInfo>) -> NSDragOperation {
            self.drag(info, true)
        }

        #[unsafe(method(draggingUpdated:))]
        fn dragging_updated(&self, info: &ProtocolObject<dyn NSDraggingInfo>) -> NSDragOperation {
            self.drag(info, false)
        }

        #[unsafe(method(draggingExited:))]
        fn dragging_exited(&self, _info: Option<&ProtocolObject<dyn NSDraggingInfo>>) {
            self.drag_left();
        }

        #[unsafe(method(performDragOperation:))]
        fn perform_drag(&self, info: &ProtocolObject<dyn NSDraggingInfo>) -> bool {
            self.drop_of(info)
        }
    }

    unsafe impl NSObjectProtocol for View {}

    // What an input method asks of where text goes, and tells it (`input.rs`). The view
    // keeps no text: the composition is all an input method can ask it for.
    unsafe impl NSTextInputClient for View {
        #[unsafe(method(insertText:replacementRange:))]
        fn insert_text(&self, text: &AnyObject, _replaced: NSRange) {
            self.insert(text);
        }

        /// A key that types nothing (an arrow, Enter): it came as a key already, and is not
        /// answered with the system's beep.
        #[unsafe(method(doCommandBySelector:))]
        fn do_command(&self, _command: Sel) {}

        #[unsafe(method(setMarkedText:selectedRange:replacementRange:))]
        fn set_marked_text(&self, text: &AnyObject, selected: NSRange, _replaced: NSRange) {
            self.mark(text, selected);
        }

        #[unsafe(method(unmarkText))]
        fn unmark_text(&self) {
            self.unmark();
        }

        #[unsafe(method(selectedRange))]
        fn selected_range(&self) -> NSRange {
            NOWHERE
        }

        #[unsafe(method(markedRange))]
        fn marked_range(&self) -> NSRange {
            match self.ivars().marked.get() {
                0 => NOWHERE,
                length => NSRange { location: 0, length },
            }
        }

        #[unsafe(method(hasMarkedText))]
        fn has_marked_text(&self) -> bool {
            self.ivars().marked.get() > 0
        }

        #[unsafe(method_id(attributedSubstringForProposedRange:actualRange:))]
        fn substring(&self, _range: NSRange, _actual: *mut NSRange) -> Option<Retained<NSAttributedString>> {
            None
        }

        #[unsafe(method_id(validAttributesForMarkedText))]
        fn valid_attributes(&self) -> Retained<NSArray<NSString>> {
            NSArray::new()
        }

        /// Where the candidates open: beside the caret.
        #[unsafe(method(firstRectForCharacterRange:actualRange:))]
        fn first_rect(&self, _range: NSRange, _actual: *mut NSRange) -> NSRect {
            self.caret_on_screen()
        }

        #[unsafe(method(characterIndexForPoint:))]
        fn character_index(&self, _point: NSPoint) -> usize {
            NOWHERE.location
        }
    }
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
        view.takes_drops();
        view
    }
}
