//! The key input of the kit (specs/behaviour/15, "Key input"): `quadra_headless::keys` in the
//! frame of a text input. It says the keys, "Press keys…" while it records, and "Not bound"
//! while the value is empty.

use quadra_layout::{Align, Justify};
use quadra_platform::Cursor;
use quadra_reactive::{Signal, signal};
use quadra_tree::NodeId;
use quadra_view::{AnyView, Div, IntoProp, MountCx, Prop, View, by_text, dynamic, row, text};

use crate::display::key_label;
use crate::token;

pub struct KeyInput {
    value: Signal<String>,
    label: Option<Prop<String>>,
    width: f32,
}

/// A field that records a keystroke into `value` (`ctrl-shift-t`; empty: unbound), 160 px
/// wide unless told otherwise.
pub fn key_input(value: Signal<String>) -> KeyInput {
    KeyInput { value, label: None, width: 160.0 }
}

impl KeyInput {
    /// What a reader calls it: the action it binds. A constant or words that change.
    pub fn label<M>(mut self, name: impl IntoProp<String, M>) -> Self {
        self.label = Some(name.into_prop());
        self
    }

    pub fn width(mut self, px: f32) -> Self {
        self.width = px;
        self
    }

    pub(crate) fn build(self) -> Div {
        let (value, recording) = (self.value, signal(false));
        let says = dynamic(move || {
            let words = match (recording.get(), value.get()) {
                (true, _) => text("Press keys\u{2026}").color(token::TEXT_2),
                (false, keys) if keys.is_empty() => text("Not bound").color(token::TEXT_3),
                (false, keys) => text(key_label(&keys)).color(token::TEXT),
            };
            AnyView::new(words)
        });
        let frame = row().align_items(Align::Center).justify(Justify::Center).width(self.width);
        let frame =
            frame.height(by_text(crate::size::H_MD)).padding_x(8.0).radius(token::RADIUS_LG).border(1.0);
        let frame = frame.bg(token::INPUT_BG).border_color(token::LINE_STRONG).cursor(Cursor::Pointer);
        let frame = frame.focus(|s| s.border_color(token::BORDER_FOCUSED));
        let frame = match self.label {
            Some(name) => frame.label(name),
            None => frame,
        };
        quadra_headless::keys::key_input(frame.child(says), value, recording)
    }
}

impl View for KeyInput {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        self.build().mount(cx)
    }
}
