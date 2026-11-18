//! Links (specs/18, "Input"): an OSC 8 hyperlink or a URL in the text under a cell, looked
//! for in the snapshot the view drew last. Ctrl+click follows one; while Ctrl is held, the one
//! under the pointer is underlined and the pointer is a hand.

use std::cell::Cell;
use std::ops::Range;
use std::rc::Rc;

use oneterm_vt::{RowId, SnapshotContent, SnapshotState};
use quadra_platform::{Cursor, Key};
use quadra_prim::Point;
use quadra_tree::{Handle, NodeId, PointerKind, Tree};
use quadra_view::MountCx;

use crate::session::Session;
use crate::widget::TerminalWidget;

/// A link the user followed: what it opens, and the visible text of an OSC 8 link when that is
/// not its target (a URL in the text is its own text).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TermLink {
    pub url: String,
    pub text: Option<String>,
}

/// The cells a link covers: a row and its columns, for each of its rows.
pub(crate) type Cells = Vec<(RowId, Range<u16>)>;

/// The link at the cell `(row, col)` of the viewport, and its cells.
pub(crate) fn link_at(state: &SnapshotState, row: usize, col: usize) -> Option<(TermLink, Cells)> {
    let rows = state.rows();
    if rows.get(row).is_none_or(|it| col >= it.cells.len()) {
        return None;
    }
    // ponytail: the rows that show only; a URL wrapped off the top or bottom is cut there.
    let first = (0..row).rev().take_while(|&r| rows[r].wrapped).last().unwrap_or(row);
    let last = (row..rows.len()).find(|&r| !rows[r].wrapped).unwrap_or(rows.len() - 1);
    // The logical line's text, and where each character's cell is and starts in it.
    let (mut text, mut at) = (String::new(), Vec::new());
    for (r, line) in rows.iter().enumerate().take(last + 1).skip(first) {
        for (c, cell) in line.cells.iter().enumerate().filter(|it| !it.1.width.is_spacer()) {
            at.push((r, c, text.len()));
            match cell.content {
                SnapshotContent::Scalar(ch) => text.push(ch),
                SnapshotContent::Cluster { start, len } => text.extend(line.cluster(start, len)),
            }
        }
    }
    // The pressed cell, or the wide one a spacer belongs to.
    let k = at.iter().rposition(|&(r, c, _)| r == row && c <= col)?;
    let cell = |i: usize| &rows[at[i].0].cells[at[i].1];
    let byte = |i: usize| at.get(i).map_or(text.len(), |it| it.2);
    let (from, to, link) = match cell(k).hyperlink {
        Some(id) => {
            let from = (0..k).rev().take_while(|&i| cell(i).hyperlink == Some(id)).last().unwrap_or(k);
            let to = (k..at.len()).find(|&i| cell(i).hyperlink != Some(id)).unwrap_or(at.len());
            let url = state.hyperlink(id)?.uri.to_string();
            let shown = text[byte(from)..byte(to)].trim();
            let text = (shown != url).then(|| shown.to_string());
            (from, to, TermLink { url, text })
        }
        None => {
            let span = quadra_text::find_urls(&text).into_iter().find(|it| it.range.contains(&at[k].2))?;
            let from = at.partition_point(|it| it.2 < span.range.start);
            let to = at.partition_point(|it| it.2 < span.range.end);
            (from, to, TermLink { url: span.url, text: None })
        }
    };
    let mut cells: Cells = Vec::new();
    for &(r, c, _) in &at[from..to] {
        let (id, c) = (rows[r].id, c as u16);
        match cells.last_mut() {
            Some((last, cols)) if *last == id => cols.end = c + 1,
            _ => cells.push((id, c..c + 1)),
        }
    }
    Some((link, cells))
}

/// The cell under a point of the view, when it is on the grid.
fn cell_at(tree: &mut Tree, id: NodeId, at: Point) -> Option<(usize, usize)> {
    let (row, col) = crate::pointer::grid_at(tree, id, at)?;
    (row >= 0.0 && col >= 0.0).then_some((row as usize, col as usize))
}

/// The link under a point of the view.
pub(crate) fn under(tree: &mut Tree, id: NodeId, at: Point) -> Option<TermLink> {
    let (row, col) = cell_at(tree, id, at)?;
    link_at(&tree.widget_mut::<TerminalWidget>(id)?.state, row, col).map(|it| it.0)
}

/// Underlines the link under `at` while `ctrl` is held, the pointer a hand over it. Looked for
/// again only when the cell or Ctrl changed; a change paints the view, the snapshot kept.
pub(crate) fn hover(tree: &mut Tree, id: NodeId, at: Option<Point>, ctrl: bool) {
    let cell = at.filter(|_| ctrl).and_then(|at| cell_at(tree, id, at));
    let Some(w) = tree.widget_mut::<TerminalWidget>(id) else { return };
    if w.hovered_at == Some(cell) {
        return;
    }
    w.hovered_at = Some(cell);
    let cells = cell.and_then(|(r, c)| link_at(&w.state, r, c)).map(|it| it.1).unwrap_or_default();
    if cells == w.hover {
        return;
    }
    let shape = if cells.is_empty() { w.shape } else { Cursor::Pointer };
    w.hover = cells;
    tree.set_cursor(id, Some(shape));
    tree.mark_paint(id);
}

/// The pointer shape a program names (OSC 22, CSS's names) as the system's cursor; the text
/// cursor for a name it has none for. ponytail: `grab` the hand and `grabbing` Move, as the
/// system has no closed hand; a cursor of our own when that matters.
pub(crate) fn shape(tree: &mut Tree, id: NodeId, name: &str) {
    let shape = cursor_of(name).unwrap_or(Cursor::Text);
    let Some(w) = tree.widget_mut::<TerminalWidget>(id) else { return };
    w.shape = shape;
    if w.hover.is_empty() {
        tree.set_cursor(id, Some(shape));
    }
}

/// The CSS pointer names there is a cursor for: what the engine answers a program's query by.
pub(crate) fn drawn_shapes() -> Vec<std::borrow::Cow<'static, str>> {
    let has = |name: &&&str| cursor_of(name).is_some();
    oneterm_vt::PointerShapes::CSS.iter().filter(has).map(|&name| name.into()).collect()
}

/// The system's cursor for a pointer name of a program, CSS or X11.
fn cursor_of(name: &str) -> Option<Cursor> {
    Some(match name {
        "default" | "left_ptr" | "arrow" => Cursor::Arrow,
        "pointer" | "hand" | "pointing_hand" | "grab" | "openhand" => Cursor::Pointer,
        "move" | "grabbing" | "closedhand" | "all-scroll" | "fleur" => Cursor::Move,
        "crosshair" | "cell" | "cross" => Cursor::Crosshair,
        "not-allowed" | "no-drop" | "forbidden" => Cursor::NotAllowed,
        "ew-resize" | "col-resize" | "e-resize" | "w-resize" => Cursor::ResizeHorizontal,
        "ns-resize" | "row-resize" | "n-resize" | "s-resize" => Cursor::ResizeVertical,
        "nwse-resize" | "nw-resize" | "se-resize" => Cursor::ResizeNwse,
        "nesw-resize" | "ne-resize" | "sw-resize" => Cursor::ResizeNesw,
        "wait" | "watch" => Cursor::Wait,
        "progress" => Cursor::Progress,
        "help" => Cursor::Help,
        "text" | "xterm" | "ibeam" => Cursor::Text,
        _ => return None,
    })
}

/// Where the pointer was last over the view, and whether Ctrl was held then.
#[derive(Default)]
pub(crate) struct Hovering {
    at: Cell<Option<Point>>,
    ctrl: Cell<bool>,
}

impl Hovering {
    /// The pointer moved (`None`: it left), or Ctrl changed: the link under it is looked for
    /// while Ctrl is held, and once more as it is let go. Nothing else costs anything.
    pub(crate) fn moved(&self, writer: &Handle, id: NodeId, at: Option<Point>, ctrl: bool) {
        self.at.set(at);
        if self.ctrl.replace(ctrl) || ctrl {
            writer.with(move |tree| hover(tree, id, at, ctrl));
        }
    }
}

/// Ctrl pressed and released while the view has the focus, and the pointer leaving it.
pub(crate) fn follow_ctrl(cx: &mut MountCx, id: NodeId) -> Rc<Hovering> {
    let hovering = Rc::new(Hovering::default());
    for down in [true, false] {
        let (writer, h) = (cx.handle.clone(), hovering.clone());
        cx.tree.on_key(id, down, move |e| {
            if e.key == Key::Ctrl {
                h.moved(&writer, id, h.at.get(), down);
            }
        });
    }
    let (writer, h) = (cx.handle.clone(), hovering.clone());
    cx.tree.on_pointer(id, PointerKind::Leave, move |_| h.moved(&writer, id, None, false));
    hovering
}

impl Session {
    /// The link under the last press of the other button; none after a press of another. For
    /// the app's menu.
    pub fn link_pressed(&self) -> Option<TermLink> {
        self.0.pressed.borrow().clone()
    }
}
