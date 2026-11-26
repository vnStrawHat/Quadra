//! Drops from other programs (specs/10, "Drops from the OS"): files or text dragged over
//! the view arrive as the platform's drag events. The system is told a copy is taken
//! wherever the view is: the target is the app's to find.

use std::path::PathBuf;

use objc2::DefinedClass;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{
    NSDragOperation, NSDraggingInfo, NSPasteboard, NSPasteboardTypeFileURL, NSPasteboardTypeString,
};
use objc2_foundation::{NSArray, NSURL};
use quadra_platform::{Dropped, PlatformEvent};
use quadra_prim::Point;

use super::view::View;

type Info = ProtocolObject<dyn NSDraggingInfo>;

/// What a pasteboard carries that a window takes: files when there are any, otherwise text.
pub(crate) fn carried(board: &NSPasteboard) -> Option<Dropped> {
    // SAFETY: constants of AppKit's, set before any code of the app runs.
    let (file, text) = unsafe { (NSPasteboardTypeFileURL, NSPasteboardTypeString) };
    let files: Vec<PathBuf> = board
        .pasteboardItems()
        .iter()
        .flat_map(|items| items.iter())
        .filter_map(|item| item.stringForType(file))
        .filter_map(|url| NSURL::URLWithString(&url)?.to_file_path())
        .collect();
    if !files.is_empty() {
        return Some(Dropped::Files(files));
    }
    board.stringForType(text).map(|text| Dropped::Text(text.to_string()))
}

impl View {
    /// The view takes files and text dragged from other programs.
    pub(super) fn takes_drops(&self) {
        // SAFETY: as in `carried`.
        let kinds = unsafe { [NSPasteboardTypeFileURL, NSPasteboardTypeString] };
        self.registerForDraggedTypes(&NSArray::from_slice(&kinds));
    }

    fn drag_at(&self, info: &Info) -> Point {
        let at = self.convertPoint_fromView(info.draggingLocation(), None);
        Point::new(at.x as f32, at.y as f32)
    }

    /// Something dragged came over the view (`entered`) or moved over it: what the system
    /// is told will be done with it.
    pub(super) fn drag(&self, info: &Info, entered: bool) -> NSDragOperation {
        let (state, at) = (self.ivars(), self.drag_at(info));
        if entered {
            let what = carried(&info.draggingPasteboard());
            state.dragging.set(what.is_some());
            if let Some(what) = what {
                state.send(PlatformEvent::DragEnter { at, what });
            }
        } else if state.dragging.get() {
            state.send(PlatformEvent::DragOver { at });
        }
        if state.dragging.get() { NSDragOperation::Copy } else { NSDragOperation::None }
    }

    pub(super) fn drag_left(&self) {
        if self.ivars().dragging.replace(false) {
            self.ivars().send(PlatformEvent::DragLeave);
        }
    }

    /// It was dropped: whether the view took it.
    pub(super) fn drop_of(&self, info: &Info) -> bool {
        let taken = self.ivars().dragging.replace(false);
        if taken {
            self.ivars().send(PlatformEvent::Drop { at: self.drag_at(info) });
        }
        taken
    }
}

#[cfg(test)]
mod tests {
    use objc2_foundation::NSString;

    use super::*;

    /// On a pasteboard of the test's own: never the clipboard.
    #[test]
    fn a_pasteboard_carries_files_or_text() {
        let board = NSPasteboard::pasteboardWithUniqueName();
        // SAFETY: as in `carried`.
        let (file, text) = unsafe { (NSPasteboardTypeFileURL, NSPasteboardTypeString) };
        assert_eq!(carried(&board), None);
        board.clearContents();
        assert!(board.setString_forType(&NSString::from_str("việt"), text));
        assert_eq!(carried(&board), Some(Dropped::Text("việt".into())));
        board.clearContents();
        assert!(board.setString_forType(&NSString::from_str("file:///tmp/a%20b.txt"), file));
        assert_eq!(carried(&board), Some(Dropped::Files(vec![PathBuf::from("/tmp/a b.txt")])));
        board.clearContents();
    }
}
