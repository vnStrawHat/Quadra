//! Instances: every primitive is one fixed-size record, so one pipeline draws everything in
//! paint order (specs/07, "Instances"). Coordinates are device pixels in the layer's space.

use quadra_prim::{Color, Corners, Edges, Rect};

use crate::layer::LayerId;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Quad = 0,
    Shadow = 1,
    /// Glyphs and icons: coverage from the coverage atlas, tinted by the colour.
    Mono = 2,
    /// Images and colour glyphs: RGBA from the colour atlas.
    Poly = 3,
    /// Underlines, wavy underlines, chart columns, stroke segments.
    Line = 4,
    /// Not drawn: marks where a child layer is drawn (specs/07, "Display lists").
    Layer = 5,
}

/// The record the shaders read. Its layout is fixed: 144 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Instance {
    /// x, y, width, height.
    pub bounds: [f32; 4],
    /// The clip rectangle: x, y, width, height. Its corner radius is `params[3]`.
    pub clip: [f32; 4],
    /// Fill or tint, straight alpha.
    pub color: [f32; 4],
    /// Border colour, gradient end, or shadow colour.
    pub color2: [f32; 4],
    /// Corner radii: top-left, top-right, bottom-right, bottom-left.
    pub radii: [f32; 4],
    /// Border widths: top, right, bottom, left.
    pub border: [f32; 4],
    /// Atlas rectangle: u0, v0, u1, v1.
    pub uv: [f32; 4],
    /// Quad: gradient angle, gradient on. Shadow: blur. Line: thickness, mode, rise.
    pub params: [f32; 4],
    pub kind: u32,
    /// The slice of the atlas texture array; for a marker, the child layer.
    pub slice: u32,
    pub flags: u32,
    pub reserved: u32,
}

const _: () = assert!(std::mem::size_of::<Instance>() == 144);

/// Quad flags.
pub const FLAG_GRADIENT: u32 = 1;
pub const FLAG_BORDER: u32 = 2;
/// Sprite flag: text coverage, corrected and blended per channel.
pub const FLAG_GLYPH: u32 = 1;

fn rect4(r: Rect) -> [f32; 4] {
    [r.x, r.y, r.width, r.height]
}

fn corners4(c: Corners) -> [f32; 4] {
    [c.top_left, c.top_right, c.bottom_right, c.bottom_left]
}

impl Instance {
    fn base(kind: Kind, bounds: Rect, clip: &Clip) -> Instance {
        Instance {
            bounds: rect4(bounds),
            clip: rect4(clip.rect),
            params: [0.0, 0.0, 0.0, clip.radius],
            kind: kind as u32,
            ..Instance::default()
        }
    }

    /// The entry that stands where `child` is drawn.
    pub fn marker(child: LayerId) -> Instance {
        Instance { kind: Kind::Layer as u32, slice: child.raw(), ..Instance::default() }
    }

    /// The child layer a marker names.
    pub fn child_layer(&self) -> Option<LayerId> {
        (self.kind == Kind::Layer as u32).then(|| LayerId::from_raw(self.slice))
    }

    pub fn rect(&self) -> Rect {
        Rect::new(self.bounds[0], self.bounds[1], self.bounds[2], self.bounds[3])
    }
}

/// What a primitive is clipped to: a rectangle with one corner radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clip {
    pub rect: Rect,
    pub radius: f32,
}

impl Clip {
    /// No clip: a rectangle far larger than any surface.
    pub const NONE: Clip = Clip { rect: Rect::new(-1.0e6, -1.0e6, 2.0e6, 2.0e6), radius: 0.0 };

    /// This clip narrowed by a rectangle inside it. A radius of zero keeps the outer one.
    pub fn then(self, rect: Rect, radius: f32) -> Clip {
        Clip { rect: self.rect.intersect(rect), radius: if radius > 0.0 { radius } else { self.radius } }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Quad {
    pub bounds: Rect,
    pub background: Color,
    pub gradient_to: Option<Color>,
    /// Degrees; 0 is left to right.
    pub gradient_angle: f32,
    pub border_widths: Edges,
    pub border_color: Color,
    pub radii: Corners,
}

impl Quad {
    /// A border that can be seen: it has a width and a colour.
    fn has_border(&self) -> bool {
        self.border_widths != Edges::default() && !self.border_color.is_transparent()
    }

    /// The one instance of this quad. The shader keeps one second colour, so a gradient and a
    /// border cannot share an instance: [`push_quad`] splits them.
    pub fn instance(&self, clip: &Clip) -> Instance {
        let mut i = Instance::base(Kind::Quad, self.bounds, clip);
        i.color = self.background.to_array();
        i.radii = corners4(self.radii);
        let b = self.border_widths;
        i.border = [b.top, b.right, b.bottom, b.left];
        if let Some(to) = self.gradient_to {
            i.color2 = to.to_array();
            i.params[0] = self.gradient_angle;
            i.params[1] = 1.0;
            i.flags = FLAG_GRADIENT;
        }
        if self.has_border() {
            i.color2 = self.border_color.to_array();
            i.flags |= FLAG_BORDER;
        }
        i
    }
}

/// Appends a quad: nothing when it cannot be seen, two instances for a gradient with a border.
pub fn push_quad(out: &mut Vec<Instance>, q: &Quad, clip: &Clip) {
    if q.bounds.is_empty() || (q.background.is_transparent() && q.gradient_to.is_none() && !q.has_border()) {
        return;
    }
    if q.gradient_to.is_some() && q.has_border() {
        out.push(Quad { border_widths: Edges::default(), ..*q }.instance(clip));
        out.push(Quad { background: Color::TRANSPARENT, gradient_to: None, ..*q }.instance(clip));
        return;
    }
    out.push(q.instance(clip));
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Shadow {
    /// The box casting the shadow, already offset.
    pub bounds: Rect,
    pub radii: Corners,
    pub blur: f32,
    pub color: Color,
}

impl Shadow {
    pub fn instance(&self, clip: &Clip) -> Instance {
        // The quad grows by the blur extent so the falloff fits inside it.
        let pad = self.blur * 1.5;
        let b = self.bounds;
        let grown = Rect::new(b.x - pad, b.y - pad, b.width + pad * 2.0, b.height + pad * 2.0);
        let mut i = Instance::base(Kind::Shadow, grown, clip);
        i.color2 = self.color.to_array();
        i.radii = corners4(self.radii);
        i.params[0] = self.blur;
        i.border = [pad; 4]; // inset of the casting box inside the bounds
        i
    }
}

/// A rectangle of an atlas: a glyph, an icon, a small image.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sprite {
    pub bounds: Rect,
    /// u0, v0, u1, v1 in the atlas slice.
    pub uv: [f32; 4],
    pub slice: u32,
    /// Tint for coverage; for colour only the alpha is used.
    pub color: Color,
    pub radii: Corners,
    /// Text coverage: corrected and blended per channel.
    pub glyph: bool,
}

impl Sprite {
    /// `kind` is [`Kind::Mono`] or [`Kind::Poly`], by the atlas the tile is in.
    pub fn instance(&self, kind: Kind, clip: &Clip) -> Instance {
        let mut i = Instance::base(kind, self.bounds, clip);
        i.color = self.color.to_array();
        i.uv = self.uv;
        i.slice = self.slice;
        i.radii = corners4(self.radii);
        i.flags = if self.glyph { FLAG_GLYPH } else { 0 };
        i
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineMode {
    Straight,
    Wavy,
    /// A chart line crossing a 1 px column, antialiased exactly: the thickness is its vertical
    /// extent and `rise` how far it travels down the column; the bounds are padded by 1 px.
    Column {
        rise: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    pub bounds: Rect,
    pub color: Color,
    pub thickness: f32,
    pub mode: LineMode,
}

impl Line {
    pub fn instance(&self, clip: &Clip) -> Instance {
        let mut i = Instance::base(Kind::Line, self.bounds, clip);
        i.color = self.color.to_array();
        i.params[0] = self.thickness;
        (i.params[1], i.params[2]) = match self.mode {
            LineMode::Straight => (0.0, 0.0),
            LineMode::Wavy => (1.0, 0.0),
            LineMode::Column { rise } => (2.0, rise),
        };
        i
    }
}
