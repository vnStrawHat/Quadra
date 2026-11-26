//! A window (specs/10): an `NSWindow` whose content is the one view the scene is drawn in,
//! and whose delegate tells the handler what the window is asked.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSCursor, NSScreen, NSWindow, NSWindowButton, NSWindowDelegate,
    NSWindowStyleMask, NSWindowTitleVisibility,
};
use objc2_foundation::{NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};
use quadra_a11y::TreeUpdate;
use quadra_platform::{
    Cursor, Placement, PlatformEvent, PlatformWindow, WindowControls, WindowHandle, WindowId, WindowOptions,
    at_least, fit_size,
};
use quadra_prim::{Rect, Size};

use super::Shared;
use super::delivery::Delivery;
use super::look;
use super::view::View;

/// What a window, its view and its delegate share.
pub(crate) struct State {
    pub id: WindowId,
    pub shared: Weak<Shared>,
    /// A frame was asked for and not given yet.
    pub wants_frame: Cell<bool>,
    /// The size and the scale the handler was last told: AppKit sets a view's size more
    /// often than it changes.
    pub told: Cell<(Size, f32)>,
    /// The window has the app's own frame: the app says what a point of it is.
    pub custom_frame: bool,
    /// The pointer's shape over the content, and AppKit's cursor of it.
    pub shape: Cell<Cursor>,
    pub cursor: RefCell<Retained<NSCursor>>,
    /// The caret of what takes text, in the view's points; none while nothing does.
    pub caret: Cell<Option<Rect>>,
    /// How many UTF-16 units an input method is composing.
    pub marked: Cell<usize>,
    /// The left button went down with Control held: the press is the right button's.
    pub as_right: Cell<bool>,
    /// Something the view takes is dragged over it from another program.
    pub dragging: Cell<bool>,
}

impl State {
    /// Tells the handler something of the window.
    pub fn send(&self, event: PlatformEvent) {
        if let Some(shared) = self.shared.upgrade() {
            shared.deliver(Delivery::Event(self.id, event));
        }
    }
}

define_class!(
    // SAFETY: `NSObject` asks nothing of a subclass, and the class has no `Drop`.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = Rc<State>]
    struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {}

    unsafe impl NSWindowDelegate for Delegate {
        /// The user asked to close the window: the app decides, and closes it by dropping it.
        #[unsafe(method(windowShouldClose:))]
        fn should_close(&self, _window: &NSWindow) -> bool {
            self.ivars().send(PlatformEvent::CloseRequested);
            false
        }

        #[unsafe(method(windowDidBecomeKey:))]
        fn became_key(&self, _notification: &NSNotification) {
            self.ivars().send(PlatformEvent::Focus(true));
        }

        #[unsafe(method(windowDidResignKey:))]
        fn resigned_key(&self, _notification: &NSNotification) {
            self.ivars().send(PlatformEvent::Focus(false));
        }
    }
);

pub(crate) struct MacWindow {
    state: Rc<State>,
    window: Retained<NSWindow>,
    view: Retained<View>,
    /// A window keeps no hold on its delegate: this does.
    _delegate: Retained<Delegate>,
}

impl MacWindow {
    pub fn open(shared: &Rc<Shared>, options: &WindowOptions) -> MacWindow {
        let mtm = shared.mtm;
        let mut style =
            NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Miniaturizable;
        if options.resizable {
            style |= NSWindowStyleMask::Resizable;
        }
        if options.custom_frame {
            style |= NSWindowStyleMask::FullSizeContentView;
        }
        let display = NSScreen::mainScreen(mtm).map_or(Size::ZERO, |screen| {
            let visible = screen.visibleFrame().size;
            Size::new(visible.width as f32, visible.height as f32)
        });
        let size = at_least(fit_size(options.size, display, options.fit), options.min_size);
        let content = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(size.width.into(), size.height.into()));
        // SAFETY: a new window of a content size and a style, kept by this value alone: it is
        // not released when it closes.
        let window = unsafe {
            let window = NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                content,
                style,
                NSBackingStoreType::Buffered,
                false,
            );
            window.setReleasedWhenClosed(false);
            window
        };
        window.setTitle(&NSString::from_str(&options.title));
        window.setContentMinSize(NSSize::new(options.min_size.width.into(), options.min_size.height.into()));
        if options.custom_frame {
            // The content covers where the title bar would be; the system's buttons stay.
            window.setTitlebarAppearsTransparent(true);
            window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
        }
        let state = Rc::new(State {
            id: WindowId(shared.next_id()),
            shared: Rc::downgrade(shared),
            wants_frame: Cell::new(false),
            told: Cell::new((Size::ZERO, 0.0)),
            custom_frame: options.custom_frame,
            shape: Cell::new(Cursor::Arrow),
            cursor: RefCell::new(look::cursor(Cursor::Arrow)),
            caret: Cell::new(None),
            marked: Cell::new(0),
            as_right: Cell::new(false),
            dragging: Cell::new(false),
        });
        let view = View::new(mtm, state.clone(), content);
        window.setContentView(Some(&view));
        window.makeFirstResponder(Some(&view));
        let delegate = Delegate::alloc(mtm).set_ivars(state.clone());
        // SAFETY: `NSObject`'s initializer, of an object whose instance variables are set.
        let delegate: Retained<Delegate> = unsafe { msg_send![super(delegate), init] };
        window.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        match options.placement {
            Some(place) => {
                let frame = NSRect::new(
                    NSPoint::new(place.x.into(), place.y.into()),
                    NSSize::new(place.width.into(), place.height.into()),
                );
                window.setFrame_display(frame, false);
                if place.maximized {
                    window.zoom(None);
                }
            }
            None if options.centered => window.center(),
            None => {}
        }
        if options.visible {
            window.makeKeyAndOrderFront(None);
            NSApplication::sharedApplication(mtm).activate();
        }
        shared.windows.borrow_mut().push(Rc::downgrade(&state));
        MacWindow { state, window, view, _delegate: delegate }
    }
}

impl Drop for MacWindow {
    fn drop(&mut self) {
        self.window.setDelegate(None);
        self.window.close();
        if let Some(shared) = self.state.shared.upgrade() {
            shared.windows.borrow_mut().retain(|it| it.upgrade().is_some_and(|it| it.id != self.state.id));
        }
    }
}

impl PlatformWindow for MacWindow {
    fn id(&self) -> WindowId {
        self.state.id
    }

    fn handle(&self) -> WindowHandle {
        WindowHandle::AppKit { view: Retained::as_ptr(&self.view) as usize }
    }

    fn size(&self) -> Size {
        let size = self.view.bounds().size;
        Size::new(size.width as f32, size.height as f32)
    }

    fn scale(&self) -> f32 {
        self.window.backingScaleFactor() as f32
    }

    fn set_title(&self, title: &str) {
        self.window.setTitle(&NSString::from_str(title));
    }

    fn request_frame(&self) {
        self.state.wants_frame.set(true);
    }

    /// The view's cursor rectangle is made again with the new shape.
    fn set_cursor(&self, cursor: Cursor) {
        if self.state.shape.replace(cursor) != cursor {
            *self.state.cursor.borrow_mut() = look::cursor(cursor);
            self.window.invalidateCursorRectsForView(&self.view);
        }
    }

    fn set_text_input(&self, caret: Option<Rect>) {
        self.view.text_input(caret);
    }

    // Not yet (specs/23, "The platform"): readers.
    fn update_semantics(&self, _update: &TreeUpdate) {}

    fn minimize(&self) {
        self.window.miniaturize(None);
    }

    fn toggle_maximize(&self) {
        self.window.zoom(None);
    }

    fn to_front(&self) {
        if self.window.isMiniaturized() {
            self.window.deminiaturize(None);
        }
        self.window.makeKeyAndOrderFront(None);
        NSApplication::sharedApplication(self.window.mtm()).activate();
    }

    fn is_maximized(&self) -> bool {
        self.window.isZoomed()
    }

    /// The system's three buttons stand over the content's left end: as far as the zoom
    /// button reaches, and as much again as there is before the close button.
    fn controls(&self) -> WindowControls {
        let frame = |which| self.window.standardWindowButton(which).map(|button| button.frame());
        match (frame(NSWindowButton::CloseButton), frame(NSWindowButton::ZoomButton)) {
            (Some(close), Some(zoom)) => {
                let leading = zoom.origin.x + zoom.size.width + close.origin.x;
                WindowControls::System { leading: leading as f32 }
            }
            _ => WindowControls::of_system(),
        }
    }

    /// The window's frame as AppKit keeps it: in points, from the bottom left of the
    /// first display.
    fn placement(&self) -> Placement {
        let frame = self.window.frame();
        Placement {
            x: frame.origin.x as i32,
            y: frame.origin.y as i32,
            width: frame.size.width as i32,
            height: frame.size.height as i32,
            maximized: self.window.isZoomed(),
        }
    }
}
