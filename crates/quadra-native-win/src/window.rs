//! A Win32 window: how it is made, and what the app can tell it.

use std::cell::{Cell, OnceCell};
use std::ffi::c_void;
use std::rc::Rc;

use quadra_a11y::TreeUpdate;
use quadra_platform::{
    Cursor, Placement, PlatformError, PlatformEvent, PlatformWindow, WindowHandle, WindowId, WindowOptions,
    fit_size,
};
use quadra_prim::{Rect, Size};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow};
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DestroyWindow, GWLP_USERDATA, GetClientRect,
    HCURSOR, IDC_ARROW, LoadCursorW, RegisterClassExW, SW_SHOW, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOZORDER, SetCursor, SetWindowLongPtrW, SetWindowPos, SetWindowTextW, ShowWindow, WINDOW_EX_STYLE,
    WINDOW_STYLE, WNDCLASSEXW, WNDPROC, WS_MAXIMIZEBOX, WS_OVERLAPPEDWINDOW, WS_THICKFRAME,
};
use windows::core::{HSTRING, PCWSTR, Param, w};

use crate::a11y::Access;
use crate::ime;
use crate::input::system_cursor;
use crate::place::primary_display;
use crate::platform::{Delivery, Shared, failed};
use crate::wndproc::window_proc;

/// What the window procedure and the window's owner share.
pub(crate) struct WindowState {
    pub id: WindowId,
    pub shared: Rc<Shared>,
    /// Null once the OS window is gone.
    pub hwnd: Cell<HWND>,
    pub scale: Cell<f32>,
    /// The client size in device pixels.
    pub size: Cell<(i32, i32)>,
    pub frame_requested: Cell<bool>,
    pub tracking_leave: Cell<bool>,
    /// The user is dragging the window or its border: the OS runs its own loop.
    pub in_size_move: Cell<bool>,
    pub high_surrogate: Cell<u16>,
    /// Where the candidates of an input method go: the caret, in device pixels. `None`: the
    /// focus is on nothing that takes text, and the input method is off.
    pub caret: Cell<Option<RECT>>,
    /// The shape the pointer has over the content.
    pub cursor: Cell<HCURSOR>,
    /// The bridge to assistive technology, from when the window exists.
    pub access: OnceCell<Access>,
    /// The app draws the title bar (`frame.rs`); whether the window's borders resize it; and
    /// whether the app's maximize button is held down.
    pub custom_frame: bool,
    pub resizable: bool,
    pub frame_down: Cell<bool>,
}

impl WindowState {
    pub fn logical_size(&self) -> Size {
        let ((w, h), scale) = (self.size.get(), self.scale.get());
        Size::new(w as f32 / scale, h as f32 / scale)
    }

    pub fn emit(&self, event: PlatformEvent) {
        self.shared.deliver(Delivery::Event(self.id, event));
    }

    pub fn frame_now(&self) {
        self.frame_requested.set(false);
        self.shared.deliver(Delivery::Frame(self.id));
    }
}

/// Registers a window class. Registering it again fails, which changes nothing.
///
/// # Safety
/// `proc` must be a sound window procedure for the windows made of this class.
pub(crate) unsafe fn register_class(
    name: PCWSTR,
    proc: unsafe extern "system" fn(
        HWND,
        u32,
        windows::Win32::Foundation::WPARAM,
        windows::Win32::Foundation::LPARAM,
    ) -> windows::Win32::Foundation::LRESULT,
) -> Result<(), PlatformError> {
    // SAFETY: the structure is complete and `name` is a static string.
    unsafe {
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: WNDPROC::Some(proc),
            hInstance: GetModuleHandleW(None).map_err(failed)?.into(),
            hCursor: LoadCursorW(None, IDC_ARROW).map_err(failed)?,
            lpszClassName: name,
            ..Default::default()
        };
        RegisterClassExW(&class);
    }
    Ok(())
}

/// Makes a window of a registered class. `param` reaches its procedure with `WM_NCCREATE`.
///
/// # Safety
/// `param` must be what the class's procedure expects.
pub(crate) unsafe fn create(
    class: PCWSTR,
    title: impl Param<PCWSTR>,
    style: WINDOW_STYLE,
    parent: Option<HWND>,
    param: *const c_void,
) -> Result<HWND, PlatformError> {
    let at = CW_USEDEFAULT;
    // SAFETY: the class is registered and the caller vouches for `param`.
    unsafe {
        let instance = GetModuleHandleW(None).map_err(failed)?;
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            title,
            style,
            at,
            at,
            at,
            at,
            parent,
            None,
            Some(instance.into()),
            Some(param),
        )
        .map_err(failed)
    }
}

pub(crate) fn open(shared: &Rc<Shared>, options: &WindowOptions) -> Result<WinWindow, PlatformError> {
    let state = Rc::new(WindowState {
        id: WindowId(shared.next_id()),
        shared: shared.clone(),
        hwnd: Cell::new(HWND::default()),
        scale: Cell::new(1.0),
        size: Cell::new((0, 0)),
        frame_requested: Cell::new(false),
        tracking_leave: Cell::new(false),
        in_size_move: Cell::new(false),
        high_surrogate: Cell::new(0),
        caret: Cell::new(None),
        cursor: Cell::new(system_cursor(Cursor::Arrow)),
        access: OnceCell::new(),
        custom_frame: options.custom_frame,
        resizable: options.resizable,
        frame_down: Cell::new(false),
    });
    let style = if options.resizable {
        WS_OVERLAPPEDWINDOW
    } else {
        WS_OVERLAPPEDWINDOW & !(WS_THICKFRAME | WS_MAXIMIZEBOX)
    };
    let class = w!("QuadraWindow");
    // SAFETY: `window_proc` expects a pointer to the window's `WindowState`, which `state`
    // keeps alive: it is owned by the `WinWindow` returned, whose drop clears the pointer
    // before the window is destroyed. The other calls take the window just made.
    unsafe {
        register_class(class, window_proc)?;
        let hwnd =
            create(class, &HSTRING::from(options.title.as_str()), style, None, Rc::as_ptr(&state).cast())?;
        let window = WinWindow(state); // from here on, an early return closes the window
        let state = &window.0;
        let _ = state.access.set(Access::new(hwnd));
        // Size the client area to the logical size asked for, at this display's scale.
        let dpi = GetDpiForWindow(hwnd).max(1);
        let scale = dpi as f32 / 96.0;
        state.scale.set(scale);
        let display = primary_display();
        let (dw, dh) = (display.right - display.left, display.bottom - display.top);
        let size = fit_size(options.size, Size::new(dw as f32 / scale, dh as f32 / scale), options.fit);
        let mut frame = RECT {
            left: 0,
            top: 0,
            right: (size.width * scale) as i32,
            bottom: (size.height * scale) as i32,
        };
        AdjustWindowRectExForDpi(&mut frame, style, false, WINDOW_EX_STYLE(0), dpi).map_err(failed)?;
        if options.custom_frame {
            frame.top = 0; // the content covers the caption: no room is added for one
        }
        let (w, h) = (frame.right - frame.left, frame.bottom - frame.top);
        // With a frame of its own the window works its client area out again, our way.
        let mut flags = SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED;
        if !options.centered {
            flags |= SWP_NOMOVE;
        }
        SetWindowPos(hwnd, None, display.left + (dw - w) / 2, display.top + (dh - h) / 2, w, h, flags)
            .map_err(failed)?;
        let placed = options.placement.is_some_and(|p| state.place(p, options.visible));
        let mut client = RECT::default();
        GetClientRect(hwnd, &mut client).map_err(failed)?;
        state.size.set((client.right, client.bottom));
        if options.visible && !placed {
            let _ = ShowWindow(hwnd, SW_SHOW);
        }
        shared.windows.borrow_mut().push(Rc::downgrade(state));
        Ok(window)
    }
}

pub(crate) struct WinWindow(Rc<WindowState>);

impl Drop for WinWindow {
    fn drop(&mut self) {
        let hwnd = self.0.hwnd.replace(HWND::default());
        if !hwnd.is_invalid() {
            // SAFETY: the window is ours and alive. Its pointer to the state is cleared first:
            // nothing sent while it is destroyed reaches a state about to be freed.
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                let _ = DestroyWindow(hwnd);
            }
            self.0.shared.pacing.set(hwnd.0 as isize, None);
        }
    }
}

impl PlatformWindow for WinWindow {
    fn id(&self) -> WindowId {
        self.0.id
    }

    fn handle(&self) -> WindowHandle {
        WindowHandle::Win32 { hwnd: self.0.hwnd.get().0 as isize }
    }

    fn size(&self) -> Size {
        self.0.logical_size()
    }

    fn scale(&self) -> f32 {
        self.0.scale.get()
    }

    fn set_title(&self, title: &str) {
        // SAFETY: the string is valid for the call; a window that is gone refuses it.
        let _ = unsafe { SetWindowTextW(self.0.hwnd.get(), &HSTRING::from(title)) };
    }

    fn request_frame(&self) {
        self.0.frame_requested.set(true);
    }

    fn set_cursor(&self, cursor: Cursor) {
        let cursor = system_cursor(cursor);
        self.0.cursor.set(cursor);
        if self.0.tracking_leave.get() {
            // The pointer is over the content: it changes now, not at its next move.
            // SAFETY: the handle is one of the system's own cursors, or null for none.
            unsafe { SetCursor(Some(cursor)) };
        }
    }

    fn set_text_input(&self, caret: Option<Rect>) {
        let (hwnd, s) = (self.0.hwnd.get(), self.0.scale.get());
        let px = |v: f32| (v * s).round() as i32;
        let caret =
            caret.map(|c| RECT { left: px(c.x), top: px(c.y), right: px(c.right()), bottom: px(c.bottom()) });
        if self.0.caret.replace(caret).is_some() != caret.is_some() {
            ime::enable(hwnd, caret.is_some());
        }
        if let Some(caret) = caret {
            ime::place(hwnd, caret);
        }
    }

    fn update_semantics(&self, update: &TreeUpdate) {
        if let Some(access) = self.0.access.get() {
            access.update(update, self.0.scale.get());
        }
    }

    fn minimize(&self) {
        self.0.minimize();
    }

    fn toggle_maximize(&self) {
        self.0.toggle_maximize();
    }

    fn is_maximized(&self) -> bool {
        self.0.is_maximized()
    }

    fn placement(&self) -> Placement {
        self.0.placement()
    }
}
