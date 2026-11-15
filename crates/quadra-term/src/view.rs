//! The terminal as a view (specs/18, "The view", "Input"): one widget that fills its box,
//! sizes the grid after layout, and sends keys, text, paste and focus to the program.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use oneterm_vt::input::{KeyEvent, KeyMods, KeySpec, NamedKey};
use quadra_async::{Timers, interval};
use quadra_platform::{Clipboard, Cursor};
use quadra_reactive::{Scope, use_context};
use quadra_tree::{NodeId, Style, TextValues, Tree};
use quadra_view::{MountCx, View};

use crate::keys::{TERMINAL_CONTEXT, TermCommand, key_event};
use crate::session::Session;
use crate::widget::TerminalWidget;

const CARET_BLINK: Duration = Duration::from_millis(530);

pub struct Terminal {
    session: Session,
    font: (f32, f32),
}

/// The view of a session, in the kit's mono face at 13 px with lines of 20. It takes the
/// room its parent gives it.
pub fn terminal(session: &Session) -> Terminal {
    Terminal { session: session.clone(), font: (13.0, 20.0) }
}

impl Terminal {
    /// The size of the text and the height of a line, in pixels.
    pub fn font(mut self, size: f32, line: f32) -> Self {
        self.font = (size, line);
        self
    }
}

/// Writes bytes to the program and, when the view was scrolled back, shows the bottom again.
pub(crate) fn send(tree: &mut Tree, id: NodeId, session: &Session, bytes: &[u8]) {
    let back = {
        let mut engine = session.engine();
        let back = engine.term.screen().scroll_offset() > 0;
        if back {
            engine.term.scroll_viewport(i32::MAX);
        }
        back
    };
    if back {
        crate::widget::repaint(tree, id);
    }
    session.write(bytes);
}

impl View for Terminal {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let Terminal { session, font: (size, line) } = self;
        let family = Some(quadra_design::fonts::MONO.into());
        let text =
            TextValues { family, size: Some(size), line_height: Some(line / size), ..TextValues::default() };
        let mut style = Style { text, ..Style::default() };
        (style.layout.grow, style.layout.shrink) = (1.0, 1.0);
        style.layout.basis = quadra_layout::Length::Px(0.0);
        (style.layout.min_width, style.layout.min_height) =
            (quadra_layout::Length::Px(0.0), quadra_layout::Length::Px(0.0));
        let id = cx.tree.insert_widget(style, Box::new(TerminalWidget::new(session.clone())));
        cx.tree.set_focusable(id, true);
        cx.tree.set_cursor(id, Some(Cursor::Text));
        cx.tree.set_key_context(id, TERMINAL_CONTEXT);
        session.0.view.set(Some(id));
        *session.0.handle.borrow_mut() = Some(cx.handle.clone());

        // The grid follows the box, once layout has sized it.
        let (box_size, writer, s) = (cx.tree.size_signal(id), cx.handle.clone(), session.clone());
        cx.handle.bind(move || {
            let size = box_size.get();
            if size.width <= 0.0 || size.height <= 0.0 {
                return; // not laid out yet
            }
            let s = s.clone();
            writer.with(move |tree| {
                let Some(w) = tree.widget_mut::<TerminalWidget>(id) else { return };
                let (rows, cols) = w.grid_for(size);
                s.resize(rows, cols);
                crate::widget::repaint(tree, id);
            });
        });

        let (writer, s) = (cx.handle.clone(), session.clone());
        cx.tree.on_key(id, true, move |e| {
            let Some(event) = key_event(e.key, e.mods, e.repeat) else { return };
            let Some(bytes) = s.engine().term.encode_key_event(&event) else { return };
            e.stop();
            let s = s.clone();
            writer.with(move |tree| send(tree, id, &s, &bytes));
        });
        let (writer, s) = (cx.handle.clone(), session.clone());
        cx.tree.on_text(id, move |typed| {
            let event = KeyEvent::new(KeySpec::Character(typed.to_string()), KeyMods::default());
            let bytes = s.engine().term.encode_key_event(&event).unwrap_or_else(|| typed.as_bytes().to_vec());
            let s = s.clone();
            writer.with(move |tree| send(tree, id, &s, &bytes));
        });
        let clipboard = use_context::<Rc<dyn Clipboard>>();
        let (writer, s) = (cx.handle.clone(), session.clone());
        cx.tree.on_action::<TermCommand>(id, move |&command| {
            let bytes = match (command, &clipboard) {
                (TermCommand::Tab(back), _) => {
                    let mods = KeyMods { shift: back, ..KeyMods::default() };
                    s.engine().term.encode_key(&KeySpec::Named(NamedKey::Tab), mods).unwrap_or_default()
                }
                (TermCommand::Copy, Some(clipboard)) => {
                    if let Some(text) = s.engine().term.selection_text() {
                        clipboard.write_text(&text);
                    }
                    return;
                }
                (TermCommand::Paste, Some(clipboard)) => {
                    let text = clipboard.read_text().unwrap_or_default();
                    s.engine().term.encode_paste(&text)
                }
                _ => return,
            };
            let s = s.clone();
            writer.with(move |tree| send(tree, id, &s, &bytes));
        });
        focus(cx, id, &session);
        crate::pointer::pointer(cx, id, &session);
        id
    }
}

/// The focus: told to the program when it asked, and the cursor's blink while it is there.
fn focus(cx: &mut MountCx, id: NodeId, session: &Session) {
    let (owner, blinks) = (Scope::current(), use_context::<Rc<Timers>>().is_some());
    let blinking: Cell<Option<Scope>> = Cell::new(None);
    let (writer, s) = (cx.handle.clone(), session.clone());
    cx.tree.on_focus(id, move |gained| {
        if let Some(ended) = blinking.take() {
            ended.dispose();
        }
        if gained && blinks {
            let scope = owner.run(Scope::child);
            let writer = writer.clone();
            scope.run(|| {
                interval(CARET_BLINK, move || {
                    writer.with(move |tree| {
                        if let Some(w) = tree.widget_mut::<TerminalWidget>(id)
                            && w.cursor_style.blinking
                        {
                            w.caret_on = !w.caret_on;
                            crate::widget::repaint(tree, id);
                        }
                    })
                })
            });
            blinking.set(Some(scope));
        }
        let told = s.engine().term.encode_focus(gained).map(<[u8]>::to_vec);
        if let Some(bytes) = told {
            s.write(&bytes);
        }
        writer.with(move |tree| {
            if let Some(w) = tree.widget_mut::<TerminalWidget>(id) {
                (w.focused, w.caret_on) = (gained, true);
                crate::widget::repaint(tree, id);
            }
        });
    });
}
