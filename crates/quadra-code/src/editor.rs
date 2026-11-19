//! The code editor as a view (specs/13, "Code"; behaviour/18): a scroll view around its
//! widget, and what edits it (typed text, composition, the commands of its keys, the
//! clipboard, the pointer, the caret's blink).

mod edit;
mod find_bar;
mod keys;
mod marks;
mod paint;
mod pointer;
mod rows;
mod time;
mod widget;

use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use quadra_platform::{Clipboard, Cursor, Mods};
use quadra_reactive::{Signal, signal, use_context};
use quadra_tree::{NodeId, PointerKind, Style, TextValues};
use quadra_view::{MountCx, NodeRef, View, column, div, scroll, show};

use edit::{edit, rows_by};
use find_bar::Finding;

pub use edit::EditorHandle;
pub use keys::{EDITOR_CONTEXT, EditorCommand, bind_keys};
pub use marks::{Mark, marks};
pub use widget::{EditorWidget, KeptDoc};

use crate::highlight::{Highlight, Highlighter};
use crate::token::{Tokenizer, tokenizer_for};

pub struct Editor {
    text: String,
    path: String,
    highlight: Option<Box<dyn Highlight>>,
    /// The size of the text and the height of a line, in pixels.
    font: (f32, f32),
    wrap: bool,
    on_link: Option<pointer::OnLink>,
    base: Option<String>,
    changed: Option<Signal<bool>>,
    top: Option<Signal<usize>>,
    kept: Option<KeptDoc>,
    edited: Option<Signal<u64>>,
    reach: Option<EditorHandle>,
    position: Option<Signal<f64>>,
    caret: Option<Signal<usize>>,
}

/// A code editor over a text, in the kit's mono face at 15 px with lines of 25. Give it a
/// height, or let it grow.
pub fn editor(text: impl Into<String>) -> Editor {
    Editor {
        text: text.into(),
        path: String::new(),
        highlight: None,
        font: (15.0, 25.0),
        wrap: false,
        on_link: None,
        base: None,
        changed: None,
        top: None,
        kept: None,
        edited: None,
        reach: None,
        position: None,
        caret: None,
    }
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

    /// What Ctrl and a press on a link in the text do with its URL: open it, or follow it
    /// in the app. Without it links are not underlined or opened.
    pub fn on_link(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.on_link = Some(Rc::new(f));
        self
    }

    /// The text the gutter marks what was added, changed and removed against: the file as
    /// it was saved, or as version control has it.
    pub fn base(mut self, text: impl Into<String>) -> Self {
        self.base = Some(text.into());
        self
    }

    /// Kept true while the text is other than the base, false while it is the same.
    pub fn changed(mut self, changed: Signal<bool>) -> Self {
        self.changed = Some(changed);
        self
    }

    /// Breaks lines at the editor's width, as prose wants: no line scrolls sideways.
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// The line at the top of the view, a signal kept as it scrolls: setting it scrolls that
    /// line to the top, as far as the view goes. For a preview that follows the editor.
    pub fn top_line(mut self, line: Signal<usize>) -> Self {
        self.top = Some(line);
        self
    }

    /// Where the document goes when the view does, and is taken back from when it is built
    /// again over the same slot: its text, cursors and undo.
    pub fn keep(mut self, slot: KeptDoc) -> Self {
        self.kept = Some(slot);
        self
    }

    /// The document's version, set after each change of the text: read the text when it
    /// moves, not on every key.
    pub fn edited(mut self, version: Signal<u64>) -> Self {
        self.edited = Some(version);
        self
    }

    /// Where the view stands as a fractional line: the line at its top and how far down it
    /// (its wrapped rows counted), kept as it scrolls; setting it scrolls there, as far as the
    /// view goes. For panes that scroll together.
    pub fn position(mut self, at: Signal<f64>) -> Self {
        self.position = Some(at);
        self
    }

    /// The line of the primary cursor, kept as it moves.
    pub fn caret_line(mut self, line: Signal<usize>) -> Self {
        self.caret = Some(line);
        self
    }

    /// What reaches the editor from outside it while it is mounted.
    pub fn handle(mut self, reach: EditorHandle) -> Self {
        self.reach = Some(reach);
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
        let scroll_y = signal(0.0);
        if let Some(top) = self.top {
            time::follow_top(cx, widget.clone(), scroll_y, top);
        }
        if let Some(at) = self.position {
            time::follow_position(cx, widget.clone(), scroll_y, at);
        }
        let surface = Surface { editor: self, find, widget: widget.clone(), bar: bar.clone(), epoch };
        let handle = cx.handle.clone();
        let shown = move || find_bar::bar(find, handle.clone(), widget.clone(), epoch, bar.clone());
        let over = show(move || find.open.get(), shown).container(div().absolute().top(6.0).right(16.0));
        column().grow(1.0).child(scroll(scroll_y, surface)).child(over).mount(cx)
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
        let tokenizer = tokenizer_for(&editor.path);
        let tokens = || Box::new(Highlighter::new(tokenizer)) as Box<dyn Highlight>;
        let mut widget = EditorWidget::new(&editor.text, editor.highlight.unwrap_or_else(tokens));
        widget.wrap = editor.wrap;
        widget.prose = matches!(tokenizer, Tokenizer::Plain | Tokenizer::Markdown);
        widget.base = editor.base.as_deref().map(marks::Base::new);
        widget.changed = editor.changed;
        widget.edited = editor.edited;
        widget.caret_line = editor.caret;
        if let Some(doc) = editor.kept.as_ref().and_then(|it| it.take()) {
            widget.doc = doc;
            widget.doc.dirty = 0; // coloured again from the start
        }
        widget.kept = editor.kept;
        widget.remark();
        let editor_link = editor.on_link.clone();
        let (size, line) = editor.font;
        let family = Some(quadra_design::fonts::MONO.into());
        let text =
            TextValues { family, size: Some(size), line_height: Some(line / size), ..TextValues::default() };
        let id = cx.tree.insert_widget(Style { text, ..Style::default() }, Box::new(widget));
        cx.tree.set_focusable(id, true);
        cx.tree.set_cursor(id, Some(Cursor::Text));
        cx.tree.set_key_context(id, EDITOR_CONTEXT);
        slot.set(Some(id));
        if let Some(reach) = &editor.reach {
            reach.mounted(cx.handle.clone(), id, epoch);
        }
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
        cx.tree.on_composition(id, move |composing, caret, clause| {
            let composing = (!composing.is_empty()).then(|| (composing.to_string(), caret));
            let at = writer.now();
            writer.with(move |tree| {
                edit(tree, id, (epoch, at), |w, _| (w.preedit, w.clause) = (composing, clause))
            });
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
                    // Wrapped, up and down go by rows.
                    (EditorCommand::Move(motion, extend), _) if w.wrap && rows_by(motion, page).is_some() => {
                        w.move_rows(rows_by(motion, page).unwrap_or(0), extend)
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
        time::blink(cx, id);
        pointer::pointer(cx, id, epoch, editor_link);
        id
    }
}
