//! The text view.

use std::cell::Cell;
use std::rc::Rc;

use quadra_platform::{Clipboard, Cursor, Mods, MouseButton};
use quadra_reactive::use_context;
use quadra_tree::{
    Handle, NodeId, Paint, Patch, PointerEvent, PointerKind, Style, TEXT_SELECT_CONTEXT, Text, TextCommand,
    TextValues, Tree,
};

use crate::prop::{IntoProp, Prop};
use crate::view::{MountCx, View};

pub struct TextView {
    content: Prop<String>,
    /// What the text says of itself; the rest is as the boxes around it say.
    values: TextValues,
    color: Option<Paint>,
    selectable: bool,
    ellipsis: bool,
    cut_start: bool,
}

/// Text: a constant, or something reactive whose changes set the node's text.
pub fn text<M>(content: impl IntoProp<String, M>) -> TextView {
    let values = TextValues::default();
    TextView {
        content: content.into_prop(),
        values,
        color: None,
        selectable: false,
        ellipsis: false,
        cut_start: false,
    }
}

impl TextView {
    /// The font size in pixels.
    pub fn size(mut self, px: f32) -> Self {
        self.values.size = Some(px);
        self
    }

    pub fn weight(mut self, weight: u16) -> Self {
        self.values.weight = Some(weight);
        self
    }

    /// The height of a line, as a multiple of the size.
    pub fn line_height(mut self, multiple: f32) -> Self {
        self.values.line_height = Some(multiple);
        self
    }

    /// A colour, or a role of the theme.
    pub fn color(mut self, color: impl Into<Paint>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Lets the user select the text: by dragging over it, a word by a double click, all of
    /// it by a third click or the key for it while the text has the focus; the key for
    /// copying puts what is selected on the clipboard. The selection goes with the focus.
    pub fn selectable(mut self) -> Self {
        self.selectable = true;
        self
    }

    /// One line, cut with an ellipsis where its box ends: a box that is narrower than the
    /// text (a tab's title under a width it may not pass) shows what fits and "…".
    pub fn ellipsis(mut self) -> Self {
        self.ellipsis = true;
        self
    }

    /// One line, cut with an ellipsis at its start: "…" and the end of the words, for words
    /// whose end tells one from another (`user@host:port`, a path).
    pub fn ellipsis_start(mut self) -> Self {
        (self.ellipsis, self.cut_start) = (true, true);
        self
    }
}

/// Changes what is selected in a text, and has it painted again.
fn select(handle: &Handle, id: NodeId, f: impl FnOnce(&mut Text) + 'static) {
    handle.with(move |tree: &mut Tree| {
        if let Some(text) = tree.widget_mut::<Text>(id) {
            f(text);
            tree.mark_paint(id);
        }
    });
}

/// What makes a mounted text selectable: the focus, the pointer and two keys.
fn make_selectable(cx: &mut MountCx, id: NodeId) {
    cx.tree.set_focusable(id, true);
    cx.tree.set_cursor(id, Some(Cursor::Text));
    cx.tree.set_key_context(id, TEXT_SELECT_CONTEXT);
    // A press starts a selection (with Shift it takes the one there is to the press) and
    // holds the pointer, so a drag selects to wherever it goes.
    let dragging = Rc::new(Cell::new(false));
    for kind in [PointerKind::Down, PointerKind::Move, PointerKind::Up, PointerKind::Click] {
        let (handle, dragging) = (cx.handle.clone(), dragging.clone());
        cx.tree.on_pointer(id, kind, move |e: &mut PointerEvent| {
            if e.button != MouseButton::Left && e.kind != PointerKind::Move {
                return;
            }
            let (at, clicks, shift) = (e.local, e.clicks, e.mods.contains(Mods::SHIFT));
            match e.kind {
                PointerKind::Down => {
                    dragging.set(true);
                    e.capture_pointer();
                    select(&handle, id, move |text| text.select_to(text.index_at(at), shift));
                }
                PointerKind::Move if dragging.get() => {
                    select(&handle, id, move |text| text.select_to(text.index_at(at), true))
                }
                PointerKind::Up => dragging.set(false),
                PointerKind::Click if clicks == 2 => {
                    select(&handle, id, move |text| text.select_word(Some(text.index_at(at))))
                }
                PointerKind::Click if clicks > 2 => select(&handle, id, |text| text.select_word(None)),
                _ => {}
            }
        });
    }
    let (clipboard, handle) = (use_context::<Rc<dyn Clipboard>>(), cx.handle.clone());
    cx.tree.on_action::<TextCommand>(id, move |&command| {
        let clipboard = clipboard.clone();
        select(&handle, id, move |text| match (command, clipboard) {
            (TextCommand::SelectAll, _) => text.select_word(None),
            (TextCommand::Copy, Some(clipboard)) if !text.selected().is_empty() => {
                clipboard.write_text(text.selected())
            }
            _ => {}
        });
    });
    let handle = cx.handle.clone();
    cx.tree.on_focus(id, move |gained| {
        if !gained {
            select(&handle, id, |text| text.select_to(0, false));
        }
    });
}

impl View for TextView {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (initial, read) = match self.content {
            Prop::Value(text) => (text, None),
            Prop::Reactive(read) => (String::new(), Some(read)),
        };
        let style = Style { text: self.values, ..Style::default() };
        let mut widget = Text::new(initial);
        (widget.ellipsis, widget.cut_start) = (self.ellipsis.then_some(None), self.cut_start);
        let id = cx.tree.insert_widget(style, Box::new(widget));
        if let Some(color) = self.color {
            cx.tree.patch_style(id, Patch::TextColor(color));
        }
        if let Some(read) = read {
            let writer = cx.handle.clone();
            cx.handle.bind(move || writer.set_text(id, read()));
        }
        if self.selectable {
            make_selectable(cx, id);
        }
        id
    }
}
