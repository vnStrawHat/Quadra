//! The scroll view: content of any size behind a viewport (specs/06, "Scrolling"), and the
//! strip, a row that scrolls sideways.

use quadra_layout::{Direction, Overflow};
use quadra_prim::{Point, Rect};
use quadra_reactive::{Signal, memo};
use quadra_tree::{NodeId, PointerEvent, PointerKind};

use crate::div::div;
use crate::flow::Region;
use crate::scrollbar::{self, Scrolled};
use crate::view::{MountCx, View};

/// Shows `content` behind a viewport, scrolled down by `scroll_y` pixels (kept between zero
/// and the end). The content is a layer cut to the viewport, so scrolling paints nothing: it
/// changes one constant.
///
/// The viewport fills its container (see [`Region::container`]); give that a size or let it
/// grow.
/// What a scroll container does with the wheel: it scrolls as far as its content goes and
/// takes that off the event. What is left is for a scroll container around it; when nothing
/// is left the event stops.
pub(crate) fn wheel_scrolls(e: &mut PointerEvent, scroll_y: Signal<f32>, top: f32, end: f32) {
    let to = (top - e.delta.y).clamp(0.0, end);
    if to != top {
        scroll_y.set(to);
        e.delta.y -= top - to;
    }
    if e.delta.y == 0.0 {
        e.stop();
    }
}

/// What a scroll container does when asked to show a rect of its own box: it scrolls as
/// little as shows it, to the start of what is taller than the viewport, and says how far.
pub(crate) fn reveals(rect: Rect, scroll_y: Signal<f32>, top: f32, end: f32, height: f32) -> f32 {
    let down = if rect.y < 0.0 { rect.y } else { (rect.bottom() - height).clamp(0.0, rect.y) };
    let to = (top + down).clamp(0.0, end);
    scroll_y.set(to);
    to - top
}

pub fn scroll<V: View + 'static>(scroll_y: Signal<f32>, content: V) -> Region {
    let bind = move |viewport: NodeId, cx: &mut MountCx| {
        let scrolls = quadra_layout::Style {
            direction: Direction::Column,
            overflow_y: Overflow::Scroll,
            ..cx.tree.layout_style(viewport).clone()
        };
        cx.tree.set_layout_style(viewport, scrolls);
        let content = content.mount(cx);
        cx.tree.make_layer(content, true);
        cx.tree.cull_layer(content);
        let (outer, inner) = (cx.tree.size_signal(viewport), cx.tree.size_signal(content));
        let end = memo(move || (inner.get().height - outer.get().height).max(0.0));
        let top = memo(move || scroll_y.get().clamp(0.0, end.get()));
        cx.tree.on_pointer(viewport, PointerKind::Wheel, move |e| {
            wheel_scrolls(e, scroll_y, top.get(), end.get())
        });
        cx.tree.on_reveal(viewport, move |rect| {
            reveals(rect, scroll_y, top.get(), end.get(), outer.get_untracked().height)
        });
        let thumb = scrollbar::attach(cx, viewport, Scrolled { scroll_y, top, end, view: outer });
        cx.tree.set_children(viewport, &[content, thumb]);
        let writer = cx.handle.clone();
        cx.handle.bind(move || {
            let y = top.get();
            writer.with(move |tree| tree.set_layer_offset(content, Point::new(0.0, -y)));
        });
    };
    Region::new(div().grow(1.0), bind)
}

/// Shows a row of things behind a viewport, scrolled sideways by `scroll_x` pixels (kept
/// between zero and the end): a strip of tabs. The wheel scrolls it, turned either way, and
/// it has no scrollbar. The content is as wide as the strip at least.
// ponytail: no scrollbar and nothing brought into view; a sideways scroll view with both
// when a wide table or a block of code needs one.
pub fn scroll_strip<V: View + 'static>(scroll_x: Signal<f32>, content: V) -> Region {
    strip(scroll_x, content, true)
}

/// A box that scrolls sideways in a page that scrolls: a block of code, a wide table. It is
/// a strip that leaves the plain wheel to the page, and takes the wheel turned sideways and
/// the wheel with Shift.
pub fn scroll_x<V: View + 'static>(scroll_x: Signal<f32>, content: V) -> Region {
    strip(scroll_x, content, false)
}

fn strip<V: View + 'static>(scroll_x: Signal<f32>, content: V, plain_wheel: bool) -> Region {
    let bind = move |viewport: NodeId, cx: &mut MountCx| {
        let scrolls = quadra_layout::Style {
            direction: Direction::Row,
            overflow_x: Overflow::Scroll,
            ..cx.tree.layout_style(viewport).clone()
        };
        cx.tree.set_layout_style(viewport, scrolls);
        let content = content.mount(cx);
        let fills = quadra_layout::Style { grow: 1.0, shrink: 0.0, ..cx.tree.layout_style(content).clone() };
        cx.tree.set_layout_style(content, fills);
        cx.tree.make_layer(content, true);
        let (outer, inner) = (cx.tree.size_signal(viewport), cx.tree.size_signal(content));
        let end = memo(move || (inner.get().width - outer.get().width).max(0.0));
        let left = memo(move || scroll_x.get().clamp(0.0, end.get()));
        cx.tree.on_pointer(viewport, PointerKind::Wheel, move |e| {
            // The wheel turned sideways, or the plain wheel: one of the two says how far.
            let plain = plain_wheel || e.mods.contains(quadra_platform::Mods::SHIFT);
            let turned = if e.delta.x != 0.0 || !plain { e.delta.x } else { e.delta.y };
            let (from, to) = (left.get(), (left.get() - turned).clamp(0.0, end.get()));
            if to != from {
                scroll_x.set(to);
                e.stop();
            }
        });
        cx.tree.set_children(viewport, &[content]);
        let writer = cx.handle.clone();
        cx.handle.bind(move || {
            let x = left.get();
            writer.with(move |tree| tree.set_layer_offset(content, Point::new(-x, 0.0)));
        });
    };
    Region::new(div().grow(1.0), bind)
}
