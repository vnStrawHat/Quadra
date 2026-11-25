//! The tooltip (specs/design, "Tooltip"): a few words about what the pointer rests on,
//! shown after half a second and gone when the pointer leaves or presses. It never takes
//! the focus, and it adds no node around the view it is about: the layout does not know.
//! An arrow on the bubble's edge points at the view's middle, on the side it shows on.

use quadra_headless::hint;
use quadra_layout::{Display, Length};
use quadra_prim::Point;
use quadra_reactive::{Signal, signal};
use quadra_tree::{Along, Anchor, NodeId, Role, Side, Tree};
use quadra_vector::Svg;
use std::rc::Rc;

use quadra_view::{AnyView, Div, IntoProp, MountCx, Prop, View, column, div, drawing, dynamic, row, text};

use crate::popup::{hang, surface};
use crate::token;

fn tip_line(words: &str) -> Div {
    row()
        .gap(token::SPACE_XL)
        .padding_x(token::SPACE_MD)
        .padding_y(token::SPACE_XS)
        .child(text(words.to_string()))
}

fn tip_bubble(line: Div) -> Div {
    surface(1).role(Role::Tooltip).max_width(288.0).child(line)
}

/// The bubble of a tooltip with a few words: for a tooltip opened by hand, over a part of a
/// view (`quadra_headless::popup::hang_with`).
pub fn tip(words: &str) -> Div {
    tip_bubble(tip_line(words))
}

pub struct Tooltip<V> {
    tip: Prop<String>,
    keys: Option<String>,
    meta: Option<Rc<dyn Fn() -> String>>,
    side: Side,
    /// A view in the bubble instead of the words.
    content: Option<Rc<dyn Fn() -> AnyView>>,
    of: V,
}

/// Gives a view a tooltip: `tip` shows above it once the pointer has rested on it. It may
/// change (a closure or a signal): the bubble and what a reader is told follow.
pub fn tooltip<V: View + 'static, M>(tip: impl IntoProp<String, M>, of: V) -> Tooltip<V> {
    Tooltip { tip: tip.into_prop(), keys: None, meta: None, side: Side::Above, content: None, of }
}

impl<V> Tooltip<V> {
    /// The shortcut that does the same, muted, after the text: "Ctrl-S".
    pub fn keys(mut self, keys: impl Into<String>) -> Self {
        self.keys = Some(keys.into());
        self
    }
}

impl<V> Tooltip<V> {
    /// The side of the view the tooltip shows on: at the right of what stands in a column
    /// at the edge of the window.
    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    /// A view in the bubble instead of the words, built each time it shows; the words are
    /// then what a reader is told.
    pub fn content<W: View + 'static>(mut self, build: impl Fn() -> W + 'static) -> Self {
        self.content = Some(Rc::new(move || AnyView::new(build())));
        self
    }

    /// Lines under the text that say more, smaller and muted: one for each line of `words`,
    /// which may change (a closure or a signal).
    pub fn meta<M>(mut self, words: impl IntoProp<String, M>) -> Self {
        self.meta = Some(match words.into_prop() {
            Prop::Value(said) => Rc::new(move || said.clone()),
            Prop::Reactive(said) => Rc::from(said),
        });
        self
    }
}

impl<V: View + 'static> View for Tooltip<V> {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let of = self.of.mount(cx);
        let shown = quadra_headless::tooltip(cx, of, hint::TOOLTIP_REST);
        // To a reader the words are the description of what they are about, there or not.
        let tip: Rc<dyn Fn() -> String> = match self.tip {
            Prop::Value(said) => {
                let told = said.clone();
                cx.tree.update_semantics(of, move |says| says.description = Some(told));
                Rc::new(move || said.clone())
            }
            Prop::Reactive(said) => {
                let (said, writer): (Rc<dyn Fn() -> String>, _) = (Rc::from(said), cx.handle.clone());
                let told = said.clone();
                cx.handle.bind(move || {
                    let words = told();
                    writer.with(move |tree| {
                        tree.update_semantics(of, move |says| says.description = Some(words))
                    });
                });
                said
            }
        };

        // The bubble is an overlay opened by hand, above the view and centred on it.
        let (words, content) = ((tip, self.keys, self.meta), self.content);
        let anchor = Anchor::Beside { node: of, side: self.side, along: Along::Center, gap: 6.0 };
        hang(cx, of, shown, anchor, move || {
            let said = words.0.clone();
            let line = row()
                .gap(token::SPACE_XL)
                .padding_x(token::SPACE_MD)
                .padding_y(token::SPACE_XS)
                .child(text(move || said()));
            let line = match &words.1 {
                Some(keys) => line.child(text(keys.clone()).color(token::TEXT_2)),
                None => line,
            };
            let placed = signal(None);
            let bubble = match &content {
                Some(build) => surface(1).role(Role::Tooltip).padding(token::SPACE_LG).child(build()),
                None => tip_bubble(line),
            };
            let bubble = bubble.child(arrow(placed));
            let bubble = bubble.on_mount(move |tree, id| {
                tree.after_layout(move |tree| placed.set(arrow_place(tree, id, of)));
            });
            match &words.2 {
                Some(meta) => {
                    let meta = meta.clone();
                    let more = dynamic(move || {
                        let line = |said: &str| text(said.to_string()).size(12.0).color(token::TEXT_2);
                        meta().lines().fold(column(), |lines, said| lines.child(line(said)))
                    });
                    bubble.child(div().padding_x(token::SPACE_MD).padding_bottom(token::SPACE_XS).child(more))
                }
                None => bubble,
            }
        });
        of
    }
}

/// The arrow's box, 12 px square: its triangle is 6 deep and 12 across, its base on the
/// bubble's border.
const ARROW: f32 = 12.0;

/// The side the bubble stands on against the view (a flip seen) and where the arrow's box is
/// from the bubble's corner: on the edge toward the view, at its middle, off the corners.
fn arrow_place(tree: &Tree, bubble: NodeId, of: NodeId) -> Option<(Side, Point)> {
    if !tree.is_alive(bubble) || !tree.is_alive(of) {
        return None;
    }
    let (b, v) = (tree.window_rect(bubble), tree.window_rect(of));
    let side = if b.bottom() <= v.y + 0.5 {
        Side::Above
    } else if b.y >= v.bottom() - 0.5 {
        Side::Below
    } else if b.right() <= v.x + 0.5 {
        Side::Left
    } else {
        Side::Right
    };
    let along = |middle: f32, start: f32, len: f32| {
        (middle - start - ARROW / 2.0).round().clamp(4.0, (len - ARROW - 4.0).max(4.0))
    };
    let (x, y) = (along(v.x + v.width / 2.0, b.x, b.width), along(v.y + v.height / 2.0, b.y, b.height));
    // The base on the border: the triangle's box half over the bubble's edge.
    let at = match side {
        Side::Above => Point::new(x, b.height - 1.0),
        Side::Below => Point::new(x, 1.0 - ARROW),
        Side::Left => Point::new(b.width - 1.0, y),
        Side::Right => Point::new(1.0 - ARROW, y),
    };
    Some((side, at))
}

/// The two triangles of the arrow for a bubble on `side` of its view: the line's, and over it
/// the plane's, its base 1.4 px further in, so the border runs on along the two slopes and stops under
/// the base.
fn arrow_svgs(side: Side) -> [Rc<Svg>; 2] {
    let (a, h, e) = (ARROW, ARROW / 2.0, 1.4);
    // Pointing down, as for a bubble above, with its base this far up; turned for the others.
    let down = |up: f32| [(0.0, -up), (a, -up), (h, h - up)];
    let turned = |up: f32| -> [(f32, f32); 3] {
        down(up).map(|(x, y)| match side {
            Side::Above => (x, y),
            Side::Below => (x, a - y),
            Side::Left => (y, x),
            Side::Right => (a - y, x),
        })
    };
    let svg = |points: [(f32, f32); 3]| {
        let [p, q, r] = points;
        let d = format!("M{} {}L{} {}L{} {}Z", p.0, p.1, q.0, q.1, r.0, r.1);
        let source = format!(r#"<svg viewBox="0 0 {a} {a}"><path d="{d}" fill="currentColor"/></svg>"#);
        Rc::new(Svg::parse(&source))
    };
    [svg(turned(0.0)), svg(turned(e))]
}

/// The arrow, out of the bubble's flow, where `placed` says once the bubble is laid out. Its
/// box reaches over the view's edge and is not there for the pointer: the view keeps it.
fn arrow(placed: Signal<Option<(Side, Point)>>) -> Div {
    // Against the bubble's padding box: one pixel in from its corner, past its border.
    let at = move |y: bool| {
        move || {
            let p = placed.get().map_or(Point::new(0.0, 0.0), |(_, p)| p);
            Length::Px(if y { p.y } else { p.x } - 1.0)
        }
    };
    let mut boxed = div().absolute().inert(true).width(ARROW).height(ARROW).left(at(false)).top(at(true));
    for side in [Side::Above, Side::Below, Side::Left, Side::Right] {
        let [outer, inner] = arrow_svgs(side);
        let on = move || match placed.get() {
            Some((at, _)) if at == side => Display::Flex,
            _ => Display::None,
        };
        let layer = |svg, colour: quadra_tree::Token| {
            let drawn = drawing(svg).size(ARROW).color(colour);
            div().absolute().left(0.0).top(0.0).display(on).child(drawn)
        };
        boxed = boxed.child(layer(outer, token::LINE)).child(layer(inner, token::OVERLAY_BG));
    }
    boxed
}
