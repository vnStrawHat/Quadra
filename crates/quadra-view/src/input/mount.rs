//! Mounting a text input: the field, and everything that edits it (typed text, composition,
//! the editing actions, the clipboard, the pointer, the caret's blink).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use quadra_async::{Timers, interval};
use quadra_platform::{Clipboard, Cursor, Key, Mods, MouseButton};
use quadra_reactive::{Scope, use_context};
use quadra_tree::{
    NodeId, Patch, PointerEvent, PointerKind, SemanticAction, Style, TEXT_AREA_CONTEXT, TEXT_CONTEXT,
    TextCommand, TextField,
};

use super::{CARET_BLINK, TextInput, edit, lines_of};
use crate::prop::Prop;
use crate::view::{MountCx, View};

impl View for TextInput {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let value = self.value;
        let mut field = TextField::new(&value.get_untracked());
        let (masked, multiline) = (self.masked, self.multiline);
        (field.masked, field.multiline, field.placeholder) = (masked, multiline, self.placeholder);
        field.fit = self.fit;
        let style = Style { text: self.values, ..Style::default() };
        let id = cx.tree.insert_widget(style, Box::new(field));
        if let Some(color) = self.color {
            cx.tree.patch_style(id, Patch::TextColor(color));
        }
        if let Some(submit) = self.submit {
            // After what was typed before the key: edits wait for the update pass, in order.
            // Shift-Enter goes on to what is around the input (a find bar goes back with it).
            let (submit, writer) = (Rc::new(RefCell::new(submit)), cx.handle.clone());
            cx.tree.on_key(id, true, move |e| {
                if e.key == Key::Enter && !e.mods.contains(Mods::SHIFT) {
                    let submit = submit.clone();
                    writer.with(move |tree| {
                        let text = tree.widget_mut::<TextField>(id).map(|f| f.buffer.text().to_string());
                        if let Some(text) = text {
                            (submit.borrow_mut())(&text);
                        }
                    });
                    e.stop();
                }
            });
        }
        cx.tree.set_focusable(id, true);
        if let Some(test_id) = self.test_id {
            cx.tree.update_semantics(id, |says| says.test_id = Some(test_id.to_string()));
        }
        match self.label {
            Some(Prop::Value(name)) => cx.tree.update_semantics(id, |says| says.name = name),
            Some(Prop::Reactive(read)) => {
                let writer = cx.handle.clone();
                cx.handle.bind(move || {
                    let name = read();
                    writer.with(move |tree| tree.update_semantics(id, |says| says.name = name));
                });
            }
            None => {}
        }
        cx.tree.set_cursor(id, Some(Cursor::Text));
        cx.tree.set_key_context(id, if multiline { TEXT_AREA_CONTEXT } else { TEXT_CONTEXT });

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
        if let Some(shown) = self.revealed.filter(|_| masked) {
            let writer = cx.handle.clone();
            cx.handle.bind(move || {
                let hidden = !shown.get();
                writer.with(move |tree| {
                    if let Some(field) = tree.widget_mut::<TextField>(id) {
                        field.masked = hidden;
                    }
                    tree.mark_layout(id);
                    tree.mark_paint(id);
                });
            });
        }

        let writer = cx.handle.clone();
        cx.tree.on_text(id, move |typed| {
            let typed = lines_of(typed, multiline);
            writer.with(move |tree| {
                edit(tree, id, value, |field| {
                    field.preedit = None; // what was being composed is committed, or dropped
                    field.buffer.insert(&typed)
                })
            });
        });
        // An input method composes at the caret: shown in the field, not yet in the value.
        let writer = cx.handle.clone();
        cx.tree.on_composition(id, move |composing, caret, clause| {
            let composing = (!composing.is_empty()).then(|| (lines_of(composing, multiline), caret));
            writer.with(move |tree| {
                edit(tree, id, value, |field| {
                    field.preedit = composing;
                    field.clause = clause;
                    false
                });
                tree.mark_layout(id);
            });
        });
        // The clipboard of the window the input is in, if its context has one.
        let clipboard = use_context::<Rc<dyn Clipboard>>();
        let writer = cx.handle.clone();
        cx.tree.on_action::<TextCommand>(id, move |&command| {
            if masked && matches!(command, TextCommand::Copy | TextCommand::Cut) {
                return; // a password leaves the field only as the value
            }
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
                            return field.buffer.insert(&lines_of(&pasted, multiline));
                        }
                        _ => {}
                    }
                    field.run(command)
                })
            });
        });
        // Assistive technology, or a test, sets the whole value, one step of undo, or the
        // selection, in characters.
        let writer = cx.handle.clone();
        cx.tree.on_semantic(id, move |action| {
            match action.clone() {
                SemanticAction::SetValue(text) => {
                    let text = lines_of(&text, multiline);
                    writer.with(move |tree| {
                        edit(tree, id, value, |field| {
                            field.buffer.select_all();
                            field.buffer.insert(&text) || field.buffer.delete(quadra_text::Motion::Right)
                        })
                    });
                }
                SemanticAction::Select { anchor, focus } => writer.with(move |tree| {
                    edit(tree, id, value, |field| {
                        let text = field.buffer.text();
                        let byte = |at: usize| text.char_indices().nth(at).map_or(text.len(), |(i, _)| i);
                        let (anchor, focus) = (byte(anchor), byte(focus));
                        field.buffer.set_caret(anchor, false);
                        field.buffer.set_caret(focus, true);
                        false
                    })
                }),
                _ => return false,
            }
            true
        });

        // While it has the focus the caret blinks, on a timer that lives as long as the focus
        // (in a scope of its own). Without timers in the context it simply stays shown.
        let (owner, blinks) = (Scope::current(), use_context::<Rc<Timers>>().is_some());
        let session: Cell<Option<Scope>> = Cell::new(None);
        let writer = cx.handle.clone();
        cx.tree.on_focus(id, move |gained| {
            if let Some(ended) = session.take() {
                ended.dispose();
            }
            // A field whose view is gone may still be given the focus back as popups close.
            if gained && blinks && owner.is_alive() {
                let scope = owner.run(Scope::child);
                let writer = writer.clone();
                scope.run(|| {
                    interval(CARET_BLINK, move || {
                        writer.with(move |tree| {
                            if let Some(field) = tree.widget_mut::<TextField>(id) {
                                field.blink();
                                tree.mark_paint(id);
                            }
                        })
                    })
                });
                session.set(Some(scope));
            }
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
                let (at, kind, clicks, shift) = (e.local, e.kind, e.clicks, e.mods.contains(Mods::SHIFT));
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
                        let at = field.index_at(at);
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
