//! The editor's scenes (behaviour/18, "Tests and budgets"), beside the comparison's: a file
//! of 100,000 lines of code, a letter typed and taken out again every frame at its middle
//! (`editor`), or scrolled down 8 px a frame (`editor-scroll`).

use quadra::{AnyView, Mods, NodeRef, Point, column, expect};
use quadra_code::{EditorCommand, EditorWidget, editor};
use quadra_tree::Handle;

use crate::animate;

const LINES: usize = 100_000;

/// A Rust file of `LINES` lines: functions of ten lines with strings, numbers and comments.
fn source() -> String {
    let mut out = String::with_capacity(LINES * 48);
    for f in 0..LINES / 10 {
        out.push_str(&format!("/// The step {f} of the work, and what it gives.\n"));
        out.push_str(&format!("pub fn step_{f}(input: &[u32], scale: f64) -> Option<u64> {{\n"));
        out.push_str("    let mut total = 0u64; // what is summed\n");
        out.push_str("    for (i, value) in input.iter().enumerate() {\n");
        out.push_str(&format!("        total += (*value as u64) * {} + i as u64;\n", f % 97));
        out.push_str("    }\n");
        out.push_str("    /* a comment that runs\n       over two lines */\n");
        out.push_str(&format!("    let name = \"step {f}\"; println!(\"{{name}}: {{total}}\");\n"));
        out.push_str("    (total as f64 * scale).round().try_into().ok() }\n");
    }
    out
}

/// The editor filling the window, with the focus and its cursor at the middle line.
fn mounted(each: impl FnMut(&Handle, u32) + 'static) -> AnyView {
    let (handle, at) = (expect::<Handle>(), NodeRef::new());
    let mut each = each;
    let place = at.clone();
    animate(move |n| {
        if n == 1 {
            let place = place.clone();
            handle.with(move |tree| {
                let Some(around) = place.get() else { return };
                tree.focus_first(around);
                let Some(id) = tree.focused() else { return };
                if let Some(w) = tree.widget_mut::<EditorWidget>(id) {
                    let middle = w.doc.buffer.line_start(LINES / 2);
                    w.doc.set_cursor(middle, middle, false);
                    let caret = w.caret_rect();
                    tree.scroll_rect_into_view(id, caret);
                }
            });
        }
        each(&handle, n);
    });
    let view = column().grow(1.0).node_ref(&at).child(editor(source()).path("work.rs"));
    AnyView::new(crate::full().child(view))
}

pub fn typing() -> AnyView {
    mounted(|handle, n| {
        handle.with(move |tree| {
            if n % 2 == 1 {
                tree.text("x");
            } else {
                tree.dispatch(&EditorCommand::DeleteBack(false));
            }
        })
    })
}

pub fn scrolling() -> AnyView {
    mounted(|handle, _| {
        // 8 px down, in pixels as a touchpad gives them.
        handle.with(|tree| tree.wheel(Point::new(0.0, -8.0), true, Point::new(400.0, 300.0), Mods::NONE))
    })
}
