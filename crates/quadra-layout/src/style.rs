//! Layout style: the values of specs/behaviour/05, "Style".

use quadra_prim::Edges;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Length {
    #[default]
    Auto,
    Px(f32),
    /// A percentage of the parent's size on that axis.
    Pct(f32),
    /// All of the parent's size on that axis.
    Fill,
}

impl Length {
    pub fn is_auto(self) -> bool {
        self == Length::Auto
    }

    /// In pixels against the parent's size, or `None` when it depends on content (auto) or on
    /// a parent size that is not known.
    pub fn resolve(self, parent: Option<f32>) -> Option<f32> {
        match self {
            Length::Px(v) => Some(v),
            Length::Pct(p) => parent.map(|size| p / 100.0 * size),
            Length::Fill => parent,
            Length::Auto => None,
        }
    }
}

/// An available size as a known size: `None` when there is no limit.
pub(crate) fn finite(v: f32) -> Option<f32> {
    v.is_finite().then_some(v)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Display {
    #[default]
    Flex,
    /// Takes no space and is not laid out.
    None,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Position {
    #[default]
    Relative,
    /// Out of flow, placed by its insets against the parent's padding box.
    Absolute,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Direction {
    #[default]
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Justify {
    #[default]
    Start,
    End,
    Center,
    Between,
    Around,
    Evenly,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Stretch,
    Start,
    End,
    Center,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Overflow {
    #[default]
    Visible,
    /// Clips; the size is still the content's.
    Hidden,
    /// Clips and scrolls: content may be larger than the box.
    Scroll,
}

/// Which margins are `auto`: they take the free space on their side.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AutoMargins {
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
}

/// Offsets of an absolute child from the edges of its parent's padding box.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    pub top: Length,
    pub right: Length,
    pub bottom: Length,
    pub left: Length,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub display: Display,
    pub position: Position,
    pub direction: Direction,
    pub wrap: bool,
    pub justify: Justify,
    pub align_items: Align,
    /// `None`: the container's `align_items`.
    pub align_self: Option<Align>,
    pub gap_row: f32,
    pub gap_col: f32,
    pub grow: f32,
    pub shrink: f32,
    pub basis: Length,
    pub width: Length,
    pub height: Length,
    pub min_width: Length,
    pub min_height: Length,
    pub max_width: Length,
    pub max_height: Length,
    /// Width divided by height.
    pub aspect_ratio: Option<f32>,
    pub padding: Edges,
    pub margin: Edges,
    pub margin_auto: AutoMargins,
    pub border: Edges,
    pub inset: Insets,
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
}

impl Default for Style {
    fn default() -> Style {
        Style {
            display: Display::Flex,
            position: Position::Relative,
            direction: Direction::Row,
            wrap: false,
            justify: Justify::Start,
            align_items: Align::Stretch,
            align_self: None,
            gap_row: 0.0,
            gap_col: 0.0,
            grow: 0.0,
            shrink: 1.0,
            basis: Length::Auto,
            width: Length::Auto,
            height: Length::Auto,
            min_width: Length::Auto,
            min_height: Length::Auto,
            max_width: Length::Auto,
            max_height: Length::Auto,
            aspect_ratio: None,
            padding: Edges::default(),
            margin: Edges::default(),
            margin_auto: AutoMargins::default(),
            border: Edges::default(),
            inset: Insets::default(),
            overflow_x: Overflow::Visible,
            overflow_y: Overflow::Visible,
        }
    }
}

impl Style {
    pub fn is_row(&self) -> bool {
        matches!(self.direction, Direction::Row | Direction::RowReverse)
    }

    pub fn is_reverse(&self) -> bool {
        matches!(self.direction, Direction::RowReverse | Direction::ColumnReverse)
    }

    /// Padding plus border: the space between the border box and the content box.
    pub fn frame(&self) -> Edges {
        Edges {
            top: self.padding.top + self.border.top,
            right: self.padding.right + self.border.right,
            bottom: self.padding.bottom + self.border.bottom,
            left: self.padding.left + self.border.left,
        }
    }

    /// Whether the values a parent reads directly from its child are equal. When they are, a
    /// style change can only reach the parent through the child's answers.
    pub(crate) fn same_as_item(&self, o: &Style) -> bool {
        self.display == o.display
            && self.position == o.position
            && self.align_self == o.align_self
            && self.grow == o.grow
            && self.shrink == o.shrink
            && self.basis == o.basis
            && (self.width, self.height) == (o.width, o.height)
            && (self.min_width, self.min_height) == (o.min_width, o.min_height)
            && (self.max_width, self.max_height) == (o.max_width, o.max_height)
            && self.aspect_ratio == o.aspect_ratio
            && self.margin == o.margin
            && self.margin_auto == o.margin_auto
            && self.inset == o.inset
    }
}
