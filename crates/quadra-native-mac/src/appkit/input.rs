//! What a view makes of AppKit's events (specs/10): where the pointer is, keys, the text a
//! key types and what an input method composes, and the view's size. The app draws the text
//! being composed itself, at its caret; the system's candidates open beside that caret.

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, msg_send};
use objc2_app_kit::{NSEvent, NSEventModifierFlags};
use objc2_foundation::{NSArray, NSPoint, NSRange, NSRect, NSSize, NSString};
use quadra_platform::{HitRegion, MouseButton, PlatformEvent, PointerKind};
use quadra_prim::{Point, Rect, Size};

use super::keys;
use super::view::View;

/// The text an input method gives: a string, or one with attributes.
fn string_of(text: &AnyObject) -> String {
    match text.downcast_ref::<NSString>() {
        Some(plain) => plain.to_string(),
        // SAFETY: what is no string here is an attributed one, which has a string.
        None => unsafe {
            let plain: Retained<NSString> = msg_send![text, string];
            plain.to_string()
        },
    }
}

impl View {
    /// Where an event is, in the view's own points.
    pub(super) fn place(&self, event: &NSEvent) -> Point {
        let at = self.convertPoint_fromView(event.locationInWindow(), None);
        Point::new(at.x as f32, at.y as f32)
    }

    /// A press on what the app says is its title bar: the window is dragged by it, and
    /// zoomed by a second press. The press is the system's then, not the app's.
    pub(super) fn on_caption(&self, event: &NSEvent) -> bool {
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

    /// The button of a press of the left one: the right one's while Control is held, as a
    /// Mac has it.
    pub(super) fn left_down(&self, event: &NSEvent) -> MouseButton {
        let other = event.modifierFlags().contains(NSEventModifierFlags::Control);
        self.ivars().as_right.set(other);
        if other { MouseButton::Right } else { MouseButton::Left }
    }

    /// The button the release that ends that press is of.
    pub(super) fn left_up(&self) -> MouseButton {
        if self.ivars().as_right.replace(false) { MouseButton::Right } else { MouseButton::Left }
    }

    pub(super) fn pointer(&self, event: &NSEvent, kind: PointerKind, button: MouseButton) {
        let (at, mods) = (self.place(event), keys::mods(event.modifierFlags()));
        self.ivars().send(PlatformEvent::Pointer { kind, button, at, mods });
    }

    pub(super) fn key(&self, event: &NSEvent, down: bool) {
        let code = event.keyCode();
        // A change of the modifiers is no key that repeats, and may not be asked whether it does.
        let repeat = down && keys::flag_of(keys::key(code)).is_none() && event.isARepeat();
        let mods = keys::mods(event.modifierFlags());
        self.ivars().send(PlatformEvent::Key { key: keys::key(code), code: code.into(), mods, down, repeat });
    }

    /// The view's size or its window's scale changed. The loop does not turn while the user
    /// drags the window's edge, so the frame the app asks for is given at once.
    pub(super) fn resized(&self) {
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

    /// A key went down: the key, unless an input method is composing (the key is its own
    /// then: it moves in the candidates, or commits), and what the key types. With Command
    /// held it types nothing: it is a shortcut.
    pub(super) fn pressed(&self, event: &NSEvent) {
        let state = self.ivars();
        let composing = state.marked.get() > 0;
        if !composing {
            self.key(event, true);
        }
        if event.modifierFlags().contains(NSEventModifierFlags::Command) {
            return;
        }
        if composing || state.caret.get().is_some() {
            // Through the input method, which answers with `insertText` or `setMarkedText`.
            self.interpretKeyEvents(&NSArray::from_slice(&[event]));
        } else if let Some(text) = event.characters() {
            // Nothing takes text: no input method composes what nothing would show.
            self.typed(&text.to_string());
        }
    }

    /// Text that was typed, less what is no text: control characters, and the characters
    /// AppKit has for the keys that type nothing (the arrows, the function keys).
    fn typed(&self, text: &str) {
        let keys_of_appkit = '\u{F700}'..='\u{F8FF}';
        let typed: String = text.chars().filter(|c| !c.is_control() && !keys_of_appkit.contains(c)).collect();
        if !typed.is_empty() {
            self.ivars().send(PlatformEvent::Text(typed));
        }
    }

    /// An input method commits text: the composition is over.
    pub(super) fn insert(&self, text: &AnyObject) {
        self.typed(&string_of(text));
        self.unmark();
    }

    /// An input method composes: its text, not committed yet, and what it selects in it, in
    /// UTF-16 units.
    pub(super) fn mark(&self, text: &AnyObject, selected: NSRange) {
        let text = string_of(text);
        let units: Vec<u16> = text.encode_utf16().collect();
        self.ivars().marked.set(units.len());
        let byte = |unit: usize| String::from_utf16_lossy(&units[..unit.min(units.len())]).len();
        let (start, end) = (byte(selected.location), byte(selected.location.saturating_add(selected.length)));
        // ponytail: the clause being converted is what the input method selects; from the
        // text's attributes (the thick underline) if an input method is seen to differ.
        let target = if start < end { start..end } else { 0..0 };
        self.ivars().send(PlatformEvent::Composition { text, caret: end, target });
    }

    /// The composition is over, if there was one.
    pub(super) fn unmark(&self) {
        if self.ivars().marked.replace(0) > 0 {
            self.ivars().send(PlatformEvent::Composition { text: String::new(), caret: 0, target: 0..0 });
        }
    }

    /// Says whether something takes text, and where its caret is in the view's points. An
    /// input method is on only then; what it was composing is dropped when nothing does.
    pub(super) fn text_input(&self, caret: Option<Rect>) {
        if self.ivars().caret.replace(caret) == caret {
            return;
        }
        let Some(context) = self.inputContext() else { return };
        match caret {
            Some(_) => context.invalidateCharacterCoordinates(),
            None => {
                context.discardMarkedText();
                self.unmark();
            }
        }
    }

    /// Where the caret is on the screen, for the system's candidates.
    pub(super) fn caret_on_screen(&self) -> NSRect {
        let caret = self.ivars().caret.get().unwrap_or_default();
        let caret = NSRect::new(
            NSPoint::new(caret.x.into(), caret.y.into()),
            NSSize::new(caret.width.into(), caret.height.into()),
        );
        let caret = self.convertRect_toView(caret, None);
        self.window().map_or(caret, |window| window.convertRectToScreen(caret))
    }
}
