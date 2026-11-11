//! The scroll view: content of any size behind a viewport (specs/06, "Scrolling").

use quadra_layout::{Direction, Overflow};
use quadra_prim::Point;
use quadra_reactive::{Signal, memo};
use quadra_tree::NodeId;

use crate::div::div;
use crate::flow::Region;
use crate::view::{MountCx, View};

/// Shows `content` behind a viewport, scrolled down by `scroll_y` pixels (kept between zero
/// and the end). The content is a layer cut to the viewport, so scrolling paints nothing: it
/// changes one constant.
///
/// The viewport fills its container (see [`Region::container`]); give that a size or let it
/// grow.
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
        cx.tree.set_children(viewport, &[content]);
        let (outer, inner) = (cx.tree.size_signal(viewport), cx.tree.size_signal(content));
        let top = memo(move || scroll_y.get().clamp(0.0, (inner.get().height - outer.get().height).max(0.0)));
        let writer = cx.handle.clone();
        cx.handle.bind(move || {
            let y = top.get();
            writer.with(move |tree| tree.set_layer_offset(content, Point::new(0.0, -y)));
        });
    };
    Region::new(div().grow(1.0), bind)
}
