//! Answering one request: the cache, sizes fixed by the style, leaves, and the dispatch to a
//! container algorithm.

use quadra_prim::Size;

use crate::cache::Request;
use crate::style::{Display, Length, Style, finite};
use crate::tree::{LayoutId, LayoutTree};

/// The content size of leaves (text, images, custom widgets). The tree crate implements it.
pub trait Measure {
    /// `known` is the content size already fixed per axis; `avail` the space left for the
    /// content on each axis (infinite: no limit).
    fn measure(&mut self, node: LayoutId, known: Known, avail: Size) -> Size;
}

/// A size of which each axis may be fixed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Known {
    pub width: Option<f32>,
    pub height: Option<f32>,
}

/// The measurer of a tree without leaves: every leaf has no content.
pub struct NoMeasure;

impl Measure for NoMeasure {
    fn measure(&mut self, _: LayoutId, _: Known, _: Size) -> Size {
        Size::ZERO
    }
}

pub(crate) fn clamp_axis(v: f32, lo: Option<f32>, hi: Option<f32>) -> f32 {
    let v = hi.map_or(v, |hi| v.min(hi));
    lo.map_or(v, |lo| v.max(lo))
}

/// A known size without a frame (padding and border).
pub(crate) fn inner_known(v: Option<f32>, frame: f32) -> Option<f32> {
    v.map(|v| (v - frame).max(0.0))
}

/// An available size without a frame; no limit stays no limit.
pub(crate) fn inner_avail(v: f32, frame: f32) -> f32 {
    (v - frame).max(0.0)
}

impl LayoutTree {
    /// The border-box size of a node for a request. With `perform` the children are also
    /// placed; a measure never moves anything.
    pub(crate) fn layout_node(
        &mut self,
        m: &mut dyn Measure,
        id: LayoutId,
        req: Request,
        perform: bool,
    ) -> Size {
        let answers = &mut self.nodes[id.index()].answers;
        let hit = if perform { answers.performed(&req) } else { answers.cached(&req) };
        if let Some(out) = hit {
            return out;
        }
        self.stats.asked += 1;
        let node = &self.nodes[id.index()];
        let style = node.style.clone();
        let (s, leaf) = (&*style, node.leaf);
        // A size in pixels is definite whoever asks.
        let mut known = Known { width: req.kw, height: req.kh };
        if known.width.is_none()
            && let Length::Px(w) = s.width
        {
            let limit = finite(req.aw);
            known.width = Some(clamp_axis(w, s.min_width.resolve(limit), s.max_width.resolve(limit)));
        }
        if known.height.is_none()
            && let Length::Px(h) = s.height
        {
            let limit = finite(req.ah);
            known.height = Some(clamp_axis(h, s.min_height.resolve(limit), s.max_height.resolve(limit)));
        }
        let out = if leaf {
            self.measure_leaf(m, id, s, known, req)
        } else {
            match s.display {
                Display::Flex => self.layout_flex(m, id, s, known, req, perform),
                Display::Grid => self.layout_grid(m, id, s, known, req, perform),
                Display::None => Size::ZERO,
            }
        };
        self.nodes[id.index()].answers.store(req, out, perform);
        out
    }

    fn measure_leaf(
        &mut self,
        m: &mut dyn Measure,
        id: LayoutId,
        s: &Style,
        known: Known,
        req: Request,
    ) -> Size {
        let f = s.frame();
        self.stats.measured += 1;
        let inner = Known {
            width: inner_known(known.width, f.horizontal()),
            height: inner_known(known.height, f.vertical()),
        };
        let avail = Size::new(inner_avail(req.aw, f.horizontal()), inner_avail(req.ah, f.vertical()));
        let content = m.measure(id, inner, avail);
        // The node's own minimum and maximum apply to what the parent did not fix.
        let (aw, ah) = (finite(req.aw), finite(req.ah));
        let w = known.width.unwrap_or_else(|| {
            clamp_axis(content.width + f.horizontal(), s.min_width.resolve(aw), s.max_width.resolve(aw))
        });
        let h = known.height.unwrap_or_else(|| {
            clamp_axis(content.height + f.vertical(), s.min_height.resolve(ah), s.max_height.resolve(ah))
        });
        Size::new(w, h)
    }
}
