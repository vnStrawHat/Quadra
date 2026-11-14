//! The code editor as a view (specs/13, "Code"; behaviour/18): a scroll view around its
//! widget, and what edits it (typed text, composition, the commands of its keys, the
//! clipboard, the pointer, the caret's blink).

mod find_bar;
mod keys;
mod paint;
mod pointer;
mod widget;

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use quadra_async::{Timers, interval};
use quadra_platform::{Clipboard, Cursor, Mods};
use quadra_reactive::{Scope, signal, use_context};
use quadra_tree::{NodeId, PointerKind, Style, TextValues, Tree};
use quadra_ui::{MountCx, NodeRef, View, column, div, scroll, show};

use find_bar::Finding;

pub use keys::{EDITOR_CONTEXT, EditorCommand, bind_keys};
pub use widget::EditorWidget;

use crate::highlight::{Highlight, Highlighter};
use crate::token::tokenizer_for;

const CARET_BLINK: Duration = Duration::from_millis(530);

pub struct Editor {
    text: String,
    path: String,
    highlight: Option<Box<dyn Highlight>>,
    /// The size of the text and the height of a line, in pixels.
    font: (f32, f32),
}

/// A code editor over a text, in the kit's mono face at 15 px with lines of 25. Give it a
/// height, or let it grow.
pub fn editor(text: impl Into<String>) -> Editor {
    Editor { text: text.into(), path: String::new(), highlight: None, font: (15.0, 25.0) }
}

impl Editor {
    /// The name of the file, which says how it is coloured (`main.rs`, `Dockerfile`).
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Colours the lines with another highlighter than the tokenizer the path names.
    pub fn highlighter(mut self, highlight: impl Highlight + 'static) -> Self {
        self.highlight = Some(Box::new(highlight));
        self
    }

    /// The size of the text and the height of a line, in pixels.
    pub fn font(mut self, size: f32, line: f32) -> Self {
        self.font = (size, line);
        self
    }
}

impl View for Editor {
    /// The scroll view with the widget, and over it the find bar while it is open.
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (find, widget, bar, epoch) =
            (Finding::new(), Rc::new(Cell::new(None)), NodeRef::new(), cx.handle.now());
        let surface = Surface { editor: self, find, widget: widget.clone(), bar: bar.clone(), epoch };
        let handle = cx.handle.clone();
        let shown = move || find_bar::bar(find, handle.clone(), widget.clone(), epoch, bar.clone());
        let over = show(move || find.open.get(), shown).container(div().absolute().top(6.0).right(16.0));
        column().grow(1.0).child(scroll(signal(0.0), surface)).child(over).mount(cx)
    }
}

/// Runs `f` on the editor's widget, as at `at` on the window's clock (when the event came),
/// then has it painted again (laid out again when its count of lines changed) with its
/// caret in view.
fn edit(
    tree: &mut Tree,
    id: NodeId,
    (epoch, at): (Instant, Instant),
    f: impl FnOnce(&mut EditorWidget, usize),
) {
    let now = at.saturating_duration_since(epoch).as_millis() as u64;
    // A page is the lines of the view the editor scrolls in.
    let view = tree.parent(id).map_or(0.0, |port| tree.rect(port).height);
    let Some(w) = tree.widget_mut::<EditorWidget>(id) else { return };
    let (lines, version) = (w.doc.buffer.line_count(), w.doc.version);
    w.doc.now = now;
    f(w, ((view / w.line_px) as usize).max(1));
    w.caret_on = true; // a caret in use does not blink away
    w.follow_caret();
    let (relaid, changed) = (w.doc.buffer.line_count() != lines, w.doc.version != version);
    if changed && w.find.is_some() {
        w.refind(); // the matches of the text as it is now
    }
    let caret = w.caret_rect();
    if relaid {
        tree.mark_layout(id);
    }
    tree.mark_paint(id);
    if changed || relaid {
        tree.update_semantics(id, |_| {});
    }
    if relaid {
        tree.after_layout(move |tree| tree.scroll_rect_into_view(id, caret));
    } else {
        tree.scroll_rect_into_view(id, caret);
    }
}

/// The widget, and what the find bar shares with it.
struct Surface {
    editor: Editor,
    find: Finding,
    widget: Rc<Cell<Option<NodeId>>>,
    bar: NodeRef,
    epoch: Instant,
}

impl View for Surface {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let Surface { editor, find, widget: slot, bar, epoch } = self;
        let tokens = || Box::new(Highlighter::new(tokenizer_for(&editor.path))) as Box<dyn Highlight>;
        let widget = EditorWidget::new(&editor.text, editor.highlight.unwrap_or_else(tokens));
        let (size, line) = editor.font;
        let family = Some(quadra_kit::fonts::MONO.into());
        let text =
            TextValues { family, size: Some(size), line_height: Some(line / size), ..TextValues::default() };
        let id = cx.tree.insert_widget(Style { text, ..Style::default() }, Box::new(widget));
        cx.tree.set_focusable(id, true);
        cx.tree.set_cursor(id, Some(Cursor::Text));
        cx.tree.set_key_context(id, EDITOR_CONTEXT);
        slot.set(Some(id));
        find_bar::follow(cx, id, find);

        let writer = cx.handle.clone();
        cx.tree.on_text(id, move |typed| {
            let typed = typed.replace("\r\n", "\n").replace('\r', "\n");
            let at = writer.now();
            writer.with(move |tree| {
                edit(tree, id, (epoch, at), |w, _| {
                    w.preedit = None; // what was composed is committed, or dropped
                    w.doc.insert_text(&typed, true);
                })
            });
        });
        let writer = cx.handle.clone();
        cx.tree.on_composition(id, move |composing, caret| {
            let composing = (!composing.is_empty()).then(|| (composing.to_string(), caret));
            let at = writer.now();
            writer.with(move |tree| edit(tree, id, (epoch, at), |w, _| w.preedit = composing));
        });
        // The clipboard of the window, if its context has one.
        let clipboard = use_context::<Rc<dyn Clipboard>>();
        let writer = cx.handle.clone();
        cx.tree.on_action::<EditorCommand>(id, move |&command| {
            let (clipboard, at, bar) = (clipboard.clone(), writer.now(), bar.clone());
            writer.with(move |tree| {
                edit(tree, id, (epoch, at), |w, page| match (command, &clipboard) {
                    _ if find_bar::command(w, find, command) => {}
                    (EditorCommand::Copy, Some(clipboard)) => clipboard.write_text(&w.doc.copy_text()),
                    (EditorCommand::Cut, Some(clipboard)) => clipboard.write_text(&w.doc.cut()),
                    (EditorCommand::Paste, Some(clipboard)) => {
                        w.doc.paste(&clipboard.read_text().unwrap_or_default())
                    }
                    _ => command.run(&mut w.doc, page),
                });
                // The bar has the focus once it is there.
                if matches!(command, EditorCommand::Find | EditorCommand::Replace) {
                    tree.after_layout(move |tree| {
                        if let Some(bar) = bar.get() {
                            tree.focus_first(bar);
                        }
                    });
                }
            });
        });
        // Sideways: the wheel turned so, or with Shift.
        let writer = cx.handle.clone();
        cx.tree.on_pointer(id, PointerKind::Wheel, move |e| {
            let dx = if e.mods.contains(Mods::SHIFT) { -e.delta.y } else { -e.delta.x };
            if dx != 0.0 {
                e.stop();
                writer.with(move |tree| {
                    if tree.widget_mut::<EditorWidget>(id).is_some_and(|w| w.scroll_by(dx)) {
                        tree.mark_paint(id);
                    }
                });
            }
        });
        blink(cx, id);
        pointer::pointer(cx, id, epoch);
        id
    }
}

/// While it has the focus the caret blinks, on a timer that lives as long as the focus.
fn blink(cx: &mut MountCx, id: NodeId) {
    let (owner, blinks) = (Scope::current(), use_context::<Rc<Timers>>().is_some());
    let session: Cell<Option<Scope>> = Cell::new(None);
    let writer = cx.handle.clone();
    cx.tree.on_focus(id, move |gained| {
        if let Some(ended) = session.take() {
            ended.dispose();
        }
        if gained && blinks {
            let scope = owner.run(Scope::child);
            let writer = writer.clone();
            scope.run(|| {
                interval(CARET_BLINK, move || {
                    writer.with(move |tree| {
                        if let Some(w) = tree.widget_mut::<EditorWidget>(id) {
                            w.caret_on = !w.caret_on;
                            tree.mark_paint(id);
                        }
                    })
                })
            });
            session.set(Some(scope));
        }
        writer.with(move |tree| {
            if let Some(w) = tree.widget_mut::<EditorWidget>(id) {
                (w.focused, w.caret_on) = (gained, true);
                tree.mark_paint(id);
            }
        });
    });
}
