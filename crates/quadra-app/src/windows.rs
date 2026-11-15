//! Windows (specs/15, "Start and end"): opening one, from the app or from a view, and closing
//! one with the windows that belong to it.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_platform::{Cursor, WindowHandle, WindowId, WindowOptions};
use quadra_reactive::{Scope, provide};
use quadra_render::{Renderer, SurfaceTarget};
use quadra_tree::Tree;
use quadra_view::{AnyView, View, mount_with};

use crate::{App, AppWindow, Result, device_size};

/// A window a view asked for, opened after the entry point it asked in.
struct Asked {
    options: WindowOptions,
    parent: WindowId,
    view: Box<dyn FnOnce() -> AnyView>,
}

/// What a view opens windows with: in the context of every window's views.
#[derive(Clone)]
pub struct Windows {
    asked: Rc<RefCell<Vec<Asked>>>,
    parent: WindowId,
}

impl Windows {
    /// Opens a window showing what `view` builds, once the handler that asks has returned. It
    /// belongs to the window it was asked from and closes with it. One kept under the same name
    /// (`WindowOptions::remember`) that is open already closes first: asked again, the window
    /// comes to the front.
    pub fn open<V: View + 'static>(&self, options: WindowOptions, view: impl FnOnce() -> V + 'static) {
        let view = Box::new(move || AnyView::new(view()));
        self.asked.borrow_mut().push(Asked { options, parent: self.parent, view });
    }
}

/// The windows views asked for, waiting for the app.
#[derive(Clone, Default)]
pub(crate) struct Waiting(Rc<RefCell<Vec<Asked>>>);

impl<R: Renderer> App<R> {
    /// Opens a window showing a view. `view` runs once.
    pub fn open_window<V: View>(
        &mut self,
        options: WindowOptions,
        view: impl FnOnce() -> V,
    ) -> Result<WindowId> {
        self.open_under(options, None, view)
    }

    fn open_under<V: View>(
        &mut self,
        options: WindowOptions,
        parent: Option<WindowId>,
        view: impl FnOnce() -> V,
    ) -> Result<WindowId> {
        let options = self.recall(options);
        let window = self.platform.open_window(&options)?;
        let target = match window.handle() {
            WindowHandle::Win32 { hwnd } => SurfaceTarget::Win32 { hwnd },
            WindowHandle::Headless => SurfaceTarget::Offscreen,
        };
        let surface = self.renderer.create_surface(target, device_size(&*window))?;
        let scope = self.root.run(Scope::child);
        let mut tree = Tree::new(self.fonts.clone());
        let clock = self.platform.clone();
        tree.set_clock(move || clock.now());
        self.follow(&mut tree, &scope);
        let id = window.id();
        let windows = Windows { asked: self.waiting.0.clone(), parent: id };
        scope.run(|| {
            provide(windows);
            mount_with(&mut tree, view)
        });
        window.request_frame();
        let (caret, cursor, remember) = (None, Cursor::Arrow, options.remember);
        self.windows.push(AppWindow { scope, tree, surface, caret, cursor, window, remember, parent });
        Ok(id)
    }

    /// The windows views asked for since the last entry point.
    pub(crate) fn open_asked(&mut self) {
        let asked: Vec<Asked> = self.waiting.0.borrow_mut().drain(..).collect();
        for Asked { options, parent, view } in asked {
            let kept = options.remember.as_ref();
            let open = self.windows.iter().find(|w| kept.is_some() && w.remember.as_ref() == kept);
            if let Some(id) = open.map(|w| w.window.id()) {
                self.close(id);
            }
            // A window asked from one that has closed since is not opened.
            if !self.windows.iter().any(|w| w.window.id() == parent) {
                continue;
            }
            if let Err(e) = self.open_under(options, Some(parent), view) {
                (self.on_error)(&e);
            }
        }
    }

    /// Closes a window, and the windows that belong to it.
    pub(crate) fn close(&mut self, window: WindowId) {
        let Some(at) = self.windows.iter().position(|w| w.window.id() == window) else { return };
        let closed = self.windows.remove(at);
        let children: Vec<WindowId> =
            self.windows.iter().filter(|w| w.parent == Some(window)).map(|w| w.window.id()).collect();
        for child in children {
            self.close(child);
        }
        self.remember(closed.remember.as_deref(), &*closed.window);
        closed.scope.dispose();
        self.renderer.forget_scene(closed.tree.scene().id());
        self.renderer.drop_surface(closed.surface);
        if self.windows.is_empty() && self.quit_on_last {
            self.platform.quit();
        }
    }
}
