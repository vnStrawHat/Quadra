//! Frame pacing: what the renderer tells the loop (specs/10, "Frame pacing").

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use windows::Win32::Foundation::{HANDLE, HWND};

/// The frame-latency handle of each window's swap chain, by window handle: written by the
/// renderer, waited on by the loop (specs/10, "Frame pacing").
#[derive(Clone, Default)]
pub(crate) struct Pacing(Rc<RefCell<HashMap<isize, isize>>>);

impl Pacing {
    pub fn set(&self, hwnd: isize, waitable: Option<HANDLE>) {
        match waitable {
            Some(handle) => self.0.borrow_mut().insert(hwnd, handle.0 as isize),
            None => self.0.borrow_mut().remove(&hwnd),
        };
    }

    pub fn get(&self, hwnd: HWND) -> Option<HANDLE> {
        self.0.borrow().get(&(hwnd.0 as isize)).map(|&h| HANDLE(h as *mut _))
    }
}
