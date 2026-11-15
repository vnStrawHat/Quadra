//! The editor's find bar (behaviour/18, "Find"): at the top right of the editor, the query
//! and its switches, where the chosen match is of how many, the way to the next and the one
//! before; and, opened with Ctrl-H, what replaces them.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use quadra_design::{Size, Variant, button, icon_button, icons, input, size, token};
use quadra_layout::Align;
use quadra_platform::{Key, Mods};
use quadra_reactive::{Signal, memo, signal};
use quadra_tree::{Handle, NodeId};
use quadra_view::{Div, MountCx, NodeRef, column, row, show, text};

use super::{EditorCommand, EditorWidget, edit};
use crate::find::Query;

/// What the bar shows and sets.
#[derive(Clone, Copy)]
pub(super) struct Finding {
    pub open: Signal<bool>,
    pub replacing: Signal<bool>,
    pub query: Signal<String>,
    pub with: Signal<String>,
    pub case: Signal<bool>,
    pub word: Signal<bool>,
    pub regex: Signal<bool>,
    /// The place of the chosen match and the count; `None`: the pattern is wrong.
    pub found: Signal<Option<(usize, usize)>>,
}

impl Finding {
    pub fn new() -> Finding {
        Finding {
            open: signal(false),
            replacing: signal(false),
            query: signal(String::new()),
            with: signal(String::new()),
            case: signal(false),
            word: signal(false),
            regex: signal(false),
            found: signal(Some((0, 0))),
        }
    }
}

/// The editor's side: what the bar says is looked for, found again as it changes.
pub(super) fn follow(cx: &mut MountCx, id: NodeId, f: Finding) {
    if let Some(w) = cx.tree.widget_mut::<EditorWidget>(id) {
        w.found = Some(f.found);
    }
    let writer = cx.handle.clone();
    cx.handle.bind(move || {
        let text = f.query.get();
        let (case, word, regex) = (f.case.get(), f.word.get(), f.regex.get());
        let query = f.open.get().then_some(Query { text, case, word, regex });
        writer.with(move |tree| {
            if let Some(w) = tree.widget_mut::<EditorWidget>(id) {
                w.find = query;
                w.refind();
                tree.mark_paint(id);
            }
        });
    });
}

/// What the commands of finding do to the editor; false for the others. Opening takes a
/// selection of one line as the query.
pub(super) fn command(w: &mut EditorWidget, f: Finding, command: EditorCommand) -> bool {
    match command {
        EditorCommand::Find | EditorCommand::Replace => {
            let selected = w.doc.selected_texts().pop().unwrap_or_default();
            if !selected.is_empty() && !selected.contains('\n') {
                f.query.set(selected);
            }
            f.replacing.set(command == EditorCommand::Replace);
            f.open.set(true);
            true
        }
        EditorCommand::FindNext(back) => {
            w.go_to_match(back);
            true
        }
        // Escape closes the bar if it is open, and gives one cursor only if it is not.
        EditorCommand::Collapse if f.open.get_untracked() => {
            f.open.set(false);
            true
        }
        _ => false,
    }
}

/// What a button of the bar does to the editor.
#[derive(Clone, Copy)]
enum Act {
    Next(bool),
    Replace,
    All,
    Close,
}

/// The bar; who shows it stands it at the top right of the editor.
pub(super) fn bar(
    f: Finding,
    handle: Handle,
    widget: Rc<Cell<Option<NodeId>>>,
    epoch: Instant,
    at: NodeRef,
) -> Div {
    let act = move |what: Act| {
        let (handle, widget) = (handle.clone(), widget.clone());
        move || {
            let Some(id) = widget.get() else { return };
            let (time, with) = (handle.now(), f.with.get_untracked());
            if let Act::Close = what {
                f.open.set(false);
            }
            handle.with(move |tree| {
                if let Act::Close = what {
                    return tree.focus(id);
                }
                edit(tree, id, (epoch, time), |w, _| match what {
                    Act::Next(back) => w.go_to_match(back),
                    Act::Replace => {
                        if let Some(m) = w.current.map(|i| w.matches[i].clone()) {
                            w.doc.replace(m, &with);
                            w.refind();
                            w.go_to_match(false);
                        }
                    }
                    Act::All => {
                        let all = std::mem::take(&mut w.matches);
                        w.doc.replace_all(&all, &with);
                    }
                    Act::Close => {}
                });
            });
        }
    };
    let switch = |label: &'static str, on: Signal<bool>| {
        let flip = move || on.set(!on.get_untracked());
        button(label).variant(Variant::Ghost).size(Size::Sm).toggled(on).on_click(flip)
    };
    let said = memo(move || match f.found.get() {
        None => "Bad pattern".to_string(),
        Some((_, 0)) => "No results".to_string(),
        Some((i, n)) => format!("{i} of {n}"),
    });
    let count = text(move || said.get()).size(size::TEXT_SM.size).color(token::TEXT_3);
    let next = act(Act::Next(false));
    let query = input(f.query).width(180.0).placeholder("Find").on_submit(move |_| next());
    let first = row().align_items(Align::Center).gap(4.0).child(query);
    let first = first.child(switch("Aa", f.case)).child(switch("W", f.word)).child(switch(".*", f.regex));
    let first = first
        .child(count)
        .child(icon_button(icons::ARROW_UP, "Previous match").on_click(act(Act::Next(true))));
    let first = first.child(icon_button(icons::ARROW_DOWN, "Next match").on_click(act(Act::Next(false))));
    let first = first.child(icon_button(icons::X, "Close").on_click(act(Act::Close)));
    let (one, all) = (act(Act::Replace), act(Act::All));
    let second = move || {
        let with = input(f.with).width(180.0).placeholder("Replace");
        let row = row().align_items(Align::Center).gap(4.0).child(with);
        row.child(button("Replace").size(Size::Sm).on_click(one.clone()))
            .child(button("All").size(Size::Sm).on_click(all.clone()))
    };
    let (close, step) = (act(Act::Close), (act(Act::Next(false)), act(Act::Next(true))));
    let keys = move |e: &mut quadra_tree::KeyEvent| match e.key {
        Key::Escape => {
            close();
            e.stop();
        }
        // Enter in the query is the next match; with Shift, the one before.
        Key::F3 | Key::Enter if e.key == Key::F3 || e.mods.contains(Mods::SHIFT) => {
            if e.mods.contains(Mods::SHIFT) {
                (step.1)()
            } else {
                (step.0)()
            }
            e.stop();
        }
        _ => {}
    };
    let plane = column().gap(4.0).padding(4.0).bg(token::OVERLAY_BG);
    let plane = plane.border(1.0).border_color(token::LINE_STRONG).radius(token::RADIUS_MD).elevation(1);
    plane.node_ref(&at).on_key_down(keys).child(first).child(show(move || f.replacing.get(), second))
}
