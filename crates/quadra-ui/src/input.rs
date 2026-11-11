//! The text input view (specs/04, "Widgets"; behaviour/09): one line of text the user edits,
//! showing a signal and writing it.

use std::cell::Cell;
use std::rc::Rc;

use quadra_platform::{Clipboard, Mods, MouseButton};
use quadra_prim::Color;
use quadra_reactive::{Signal, use_context};
use quadra_text::TextStyle;
use quadra_tree::{NodeId, PointerEvent, PointerKind, Style, TEXT_CONTEXT, TextCommand, TextField, Tree};

use crate::view::{MountCx, View};

pub struct TextInput {
    value: Signal<String>,
    style: TextStyle,
    color: Color,
}

/// A line of text that edits `value`: what is typed is written to it, and what is written to
/// it from elsewhere is shown. It takes the focus on a click or by Tab; the caret, the
/// selection and undo are its own.
pub fn text_input(value: Signal<String>) -> TextInput {
    TextInput { value, style: TextStyle::default(), color: Color::BLACK }
}

impl TextInput {
    /// The font size in pixels.
    pub fn size(mut self, px: f32) -> Self {
        self.style.size = px;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

/// Text for a field of one line: its line breaks become spaces.
fn one_line(text: &str) -> String {
    text.replace("\r\n", " ").replace(['\r', '\n'], " ")
}

/// Changes the field in place. `f` says whether it changed the text: then the value is
/// written and the text shaped again; in any case the field is painted again.
fn edit(tree: &mut Tree, id: NodeId, value: Signal<String>, f: impl FnOnce(&mut TextField) -> bool) {
    let Some(field) = tree.widget_mut::<TextField>(id) else { return };
    let changed = f(field).then(|| field.buffer.text().to_string());
    if let Some(text) = changed {
        tree.mark_layout(id);
        value.set(text);
    }
    tree.mark_paint(id);
}

impl View for TextInput {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let value = self.value;
        let field = TextField::new(&value.get_untracked(), self.style, self.color);
        let id = cx.tree.insert_widget(Style::default(), Box::new(field));
        cx.tree.set_focusable(id, true);
        cx.tree.set_key_context(id, TEXT_CONTEXT);

        // What is written to the value from elsewhere is shown. What the field wrote itself
        // comes back here equal, and changes nothing.
        let writer = cx.handle.clone();
        cx.handle.bind(move || {
            let text = value.get();
            writer.with(move |tree| {
                if tree.widget_mut::<TextField>(id).is_some_and(|f| f.buffer.text() != text) {
                    edit(tree, id, value, |field| {
                        field.buffer.set_text(&text);
                        false
                    });
                    tree.mark_layout(id);
                }
            });
        });

        let writer = cx.handle.clone();
        cx.tree.on_text(id, move |typed| {
            let typed = one_line(typed);
            writer.with(move |tree| edit(tree, id, value, |field| field.buffer.insert(&typed)));
        });
        // The clipboard of the window the input is in, if its context has one.
        let clipboard = use_context::<Rc<dyn Clipboard>>();
        let writer = cx.handle.clone();
        cx.tree.on_action::<TextCommand>(id, move |&command| {
            let clipboard = clipboard.clone();
            writer.with(move |tree| {
                edit(tree, id, value, |field| {
                    let selected = field.buffer.selected_text();
                    match (command, &clipboard) {
                        (TextCommand::Copy | TextCommand::Cut, Some(clipboard)) if !selected.is_empty() => {
                            clipboard.write_text(selected)
                        }
                        (TextCommand::Paste, Some(clipboard)) => {
                            let pasted = clipboard.read_text().unwrap_or_default();
                            return field.buffer.insert(&one_line(&pasted));
                        }
                        _ => {}
                    }
                    field.run(command)
                })
            });
        });
        let writer = cx.handle.clone();
        cx.tree.on_focus(id, move |gained| {
            writer.with(move |tree| {
                edit(tree, id, value, |field| {
                    field.focused = gained;
                    false
                })
            });
        });

        // A press puts the caret (with Shift, the end of the selection); the pointer is held,
        // so a drag selects to wherever it goes. A double click selects a word, a third all.
        let dragging = Rc::new(Cell::new(false));
        let pointer = move |writer: quadra_tree::Handle, dragging: Rc<Cell<bool>>| {
            move |e: &mut PointerEvent| {
                if e.button != MouseButton::Left && e.kind != PointerKind::Move {
                    return;
                }
                let (x, kind, clicks, shift) = (e.local.x, e.kind, e.clicks, e.mods.contains(Mods::SHIFT));
                match kind {
                    PointerKind::Down => {
                        dragging.set(true);
                        e.capture_pointer();
                    }
                    PointerKind::Up => return dragging.set(false),
                    PointerKind::Move if !dragging.get() => return,
                    _ => {}
                }
                writer.with(move |tree| {
                    edit(tree, id, value, |field| {
                        let at = field.index_at(x);
                        match (kind, clicks) {
                            (PointerKind::Click, 2) => field.buffer.select_word(at),
                            (PointerKind::Click, 3..) => field.buffer.select_all(),
                            (PointerKind::Click, _) => {}
                            (PointerKind::Down, _) => field.buffer.set_caret(at, shift),
                            _ => field.buffer.set_caret(at, true),
                        }
                        false
                    })
                });
            }
        };
        for kind in [PointerKind::Down, PointerKind::Move, PointerKind::Up, PointerKind::Click] {
            cx.tree.on_pointer(id, kind, pointer(cx.handle.clone(), dragging.clone()));
        }
        id
    }
}
