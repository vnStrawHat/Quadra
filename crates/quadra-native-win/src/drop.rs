//! The window's drop target (specs/10, "Win32 behaviour"): files and text dragged from
//! another program become the window's drag events. It is a COM object of Quadra's own,
//! written as its vtable and a count: the framework has no procedural macros to make one.

use std::cell::Cell;
use std::ffi::c_void;
use std::path::PathBuf;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicU32, Ordering};

use quadra_platform::{Dropped, PlatformEvent};
use windows::Win32::Foundation::{E_NOINTERFACE, HWND, POINT, POINTL, S_OK};
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::System::Com::{DVASPECT_CONTENT, FORMATETC, IDataObject, TYMED_HGLOBAL};
use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::System::Ole::{
    CF_HDROP, CF_UNICODETEXT, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_NONE, IDropTarget, IDropTarget_Vtbl,
    OleInitialize, RegisterDragDrop, ReleaseStgMedium, RevokeDragDrop,
};
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
use windows_core::{GUID, HRESULT, IUnknown, IUnknown_Vtbl, Interface};

use crate::window::WindowState;

#[repr(C)]
struct Target {
    /// First, as a COM object's is.
    vtable: &'static IDropTarget_Vtbl,
    count: AtomicU32,
    window: Weak<WindowState>,
    /// What is dragged now can be taken: files or text.
    takes: Cell<bool>,
}

static VTABLE: IDropTarget_Vtbl = IDropTarget_Vtbl {
    base__: IUnknown_Vtbl { QueryInterface: query, AddRef: add_ref, Release: release },
    DragEnter: drag_enter,
    DragOver: drag_over,
    DragLeave: drag_leave,
    Drop: dropped,
};

/// Makes the window a target of drops; nothing when OLE cannot start on this thread.
pub(crate) fn register(hwnd: HWND, window: &Rc<WindowState>) {
    // SAFETY: OLE is started on the UI thread, the one all windows are on; a second start is
    // counted and harmless.
    if unsafe { OleInitialize(None) }.is_err() {
        return;
    }
    let target = Box::new(Target {
        vtable: &VTABLE,
        count: AtomicU32::new(1),
        window: Rc::downgrade(window),
        takes: Cell::new(false),
    });
    // SAFETY: the box is a COM object with one count, which `from_raw` takes over; OLE holds
    // its own while the window is registered.
    unsafe {
        let target = IDropTarget::from_raw(Box::into_raw(target) as *mut c_void);
        let _ = RegisterDragDrop(hwnd, &target);
    }
}

/// Stops the window being a target: before it is destroyed.
pub(crate) fn revoke(hwnd: HWND) {
    // SAFETY: the window is alive; revoking one never registered fails and changes nothing.
    let _ = unsafe { RevokeDragDrop(hwnd) };
}

/// # Safety
/// `this` must point at a `Target`.
unsafe fn target<'a>(this: *mut c_void) -> &'a Target {
    // SAFETY: as the caller vouches.
    unsafe { &*(this as *const Target) }
}

unsafe extern "system" fn query(this: *mut c_void, iid: *const GUID, out: *mut *mut c_void) -> HRESULT {
    // SAFETY: COM gives a valid id and a place for the answer.
    unsafe {
        if *iid == IDropTarget::IID || *iid == IUnknown::IID {
            add_ref(this);
            *out = this;
            return S_OK;
        }
        *out = std::ptr::null_mut();
    }
    E_NOINTERFACE
}

unsafe extern "system" fn add_ref(this: *mut c_void) -> u32 {
    // SAFETY: COM calls it on the object.
    unsafe { target(this) }.count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn release(this: *mut c_void) -> u32 {
    // SAFETY: COM calls it on the object; the last count frees the box it was made as.
    unsafe {
        let left = target(this).count.fetch_sub(1, Ordering::AcqRel) - 1;
        if left == 0 {
            drop(Box::from_raw(this as *mut Target));
        }
        left
    }
}

/// Where a point of the screen is in the window, in logical pixels, and the window.
fn place(t: &Target, at: POINTL) -> Option<(Rc<WindowState>, quadra_prim::Point)> {
    let window = t.window.upgrade()?;
    let mut point = POINT { x: at.x, y: at.y };
    // SAFETY: the window is alive while its state is.
    let _ = unsafe { ScreenToClient(window.hwnd.get(), &mut point) };
    let at = window.point(point.x, point.y);
    Some((window, at))
}

unsafe extern "system" fn drag_enter(
    this: *mut c_void,
    data: *mut c_void,
    _: MODIFIERKEYS_FLAGS,
    at: POINTL,
    effect: *mut DROPEFFECT,
) -> HRESULT {
    // SAFETY: OLE calls it on the object, with what is dragged and a place for the effect.
    let (t, data) = unsafe { (target(this), IDataObject::from_raw_borrowed(&data)) };
    let what = data.and_then(read);
    t.takes.set(what.is_some());
    if let (Some(what), Some((window, at))) = (what, place(t, at)) {
        window.emit(PlatformEvent::DragEnter { at, what });
    }
    // SAFETY: as above.
    unsafe { *effect = if t.takes.get() { DROPEFFECT_COPY } else { DROPEFFECT_NONE } };
    S_OK
}

unsafe extern "system" fn drag_over(
    this: *mut c_void,
    _: MODIFIERKEYS_FLAGS,
    at: POINTL,
    effect: *mut DROPEFFECT,
) -> HRESULT {
    // SAFETY: OLE calls it on the object, with a place for the effect.
    let t = unsafe { target(this) };
    if t.takes.get()
        && let Some((window, at)) = place(t, at)
    {
        window.emit(PlatformEvent::DragOver { at });
    }
    // SAFETY: as above.
    unsafe { *effect = if t.takes.get() { DROPEFFECT_COPY } else { DROPEFFECT_NONE } };
    S_OK
}

unsafe extern "system" fn drag_leave(this: *mut c_void) -> HRESULT {
    // SAFETY: OLE calls it on the object.
    let t = unsafe { target(this) };
    if t.takes.replace(false)
        && let Some(window) = t.window.upgrade()
    {
        window.emit(PlatformEvent::DragLeave);
    }
    S_OK
}

unsafe extern "system" fn dropped(
    this: *mut c_void,
    _: *mut c_void,
    _: MODIFIERKEYS_FLAGS,
    at: POINTL,
    effect: *mut DROPEFFECT,
) -> HRESULT {
    // SAFETY: OLE calls it on the object, with a place for the effect.
    let t = unsafe { target(this) };
    let took = t.takes.replace(false);
    if took && let Some((window, at)) = place(t, at) {
        window.emit(PlatformEvent::Drop { at });
    }
    // SAFETY: as above.
    unsafe { *effect = if took { DROPEFFECT_COPY } else { DROPEFFECT_NONE } };
    S_OK
}

/// The bytes of what is dragged in a format, as a block of memory.
fn bytes(data: &IDataObject, format: u16) -> Option<Vec<u8>> {
    let asked = FORMATETC {
        cfFormat: format,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    };
    // SAFETY: the format is well formed; the block is read while locked and given back.
    unsafe {
        let mut medium = data.GetData(&asked).ok()?;
        let block = medium.u.hGlobal;
        let at = GlobalLock(block) as *const u8;
        let out = (!at.is_null()).then(|| std::slice::from_raw_parts(at, GlobalSize(block)).to_vec());
        let _ = GlobalUnlock(block);
        ReleaseStgMedium(&mut medium);
        out
    }
}

/// UTF-16 units of bytes, up to the first zero.
fn units(bytes: &[u8]) -> Vec<u16> {
    let all = bytes.chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]]));
    all.take_while(|&unit| unit != 0).collect()
}

/// Files when there are any, otherwise text.
fn read(data: &IDataObject) -> Option<Dropped> {
    if let Some(list) = bytes(data, CF_HDROP.0) {
        // A list of files: where its names start, and whether they are wide, then the names,
        // each ended by a zero, and a zero more.
        let word = |at: usize| list.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        if let (Some(start), Some(1)) = (word(0), word(16)) {
            let mut names = Vec::new();
            let mut at = start as usize;
            while at < list.len() {
                let name = units(&list[at..]);
                if name.is_empty() {
                    break;
                }
                at += (name.len() + 1) * 2;
                names.push(PathBuf::from(String::from_utf16_lossy(&name)));
            }
            if !names.is_empty() {
                return Some(Dropped::Files(names));
            }
        }
    }
    let text = bytes(data, CF_UNICODETEXT.0)?;
    Some(Dropped::Text(String::from_utf16_lossy(&units(&text))))
}
