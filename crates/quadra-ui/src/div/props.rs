//! The style values of a box: each a setter of the box, taking a constant or something
//! reactive, and a setter of what a state changes, taking a constant.

use quadra_layout::{Align, Direction, Display, Justify, Length, Overflow, Position};
use quadra_tree::{Paint, Patch};

use super::Div;
use crate::prop::IntoProp;

/// What a state changes of a box's style: the setters of the box, taking constants (specs/05,
/// "Style and states").
#[derive(Default)]
pub struct Refine(pub(super) Vec<Patch>);

/// Declares the style values: each is a setter of the box, taking a constant or something
/// reactive, and a setter of a refinement, taking a constant.
macro_rules! props {
    ($($(#[$doc:meta])* $name:ident: $ty:ty => $patch:ident;)*) => {
        impl Div {$(
            $(#[$doc])*
            pub fn $name<M>(mut self, value: impl IntoProp<$ty, M>) -> Self {
                self.prop(value.into_prop(), Patch::$patch);
                self
            }
        )*}

        impl Refine {$(
            $(#[$doc])*
            pub fn $name(mut self, value: impl Into<$ty>) -> Self {
                self.0.push(Patch::$patch(value.into()));
                self
            }
        )*}
    };
}

props! {
    width: Length => Width;
    height: Length => Height;
    min_width: Length => MinWidth;
    min_height: Length => MinHeight;
    max_width: Length => MaxWidth;
    max_height: Length => MaxHeight;
    /// How much of the free space the box takes.
    grow: f32 => Grow;
    shrink: f32 => Shrink;
    /// The space between children, on both axes.
    gap: f32 => Gap;
    /// The space between columns, and between rows: of a grid, of lines that wrap.
    gap_x: f32 => GapX;
    gap_y: f32 => GapY;
    /// The same padding on every side.
    padding: f32 => Padding;
    /// Left and right padding.
    padding_x: f32 => PaddingX;
    /// Top and bottom padding.
    padding_y: f32 => PaddingY;
    padding_top: f32 => PaddingTop;
    padding_right: f32 => PaddingRight;
    padding_bottom: f32 => PaddingBottom;
    padding_left: f32 => PaddingLeft;
    margin: f32 => Margin;
    /// The width of the border on every side.
    border: f32 => Border;
    /// The width of the border on one side: a line under a bar, a seam beside a pane.
    border_top: f32 => BorderTop;
    border_right: f32 => BorderRight;
    border_bottom: f32 => BorderBottom;
    border_left: f32 => BorderLeft;
    direction: Direction => Direction;
    /// `Position::Absolute` takes the box out of the flow: it is placed by `left`, `top`,
    /// `right` and `bottom` against the box around it.
    position: Position => Position;
    left: Length => Left;
    top: Length => Top;
    right: Length => Right;
    bottom: Length => Bottom;
    /// `Display::None` takes the box and what it holds out of the layout and the picture,
    /// mounted as it is.
    display: Display => Display;
    justify: Justify => Justify;
    align_items: Align => AlignItems;
    wrap: bool => Wrap;
    overflow_x: Overflow => OverflowX;
    overflow_y: Overflow => OverflowY;
    /// The background colour.
    bg: Paint => Background;
    border_color: Paint => BorderColor;
    /// The same radius on every corner.
    radius: f32 => Radius;
    /// A line of one pixel along the box's edge, taking no room: a mark of the focus.
    outline: Paint => Outline;
    /// The shadow of a level above the window: 1 for a menu, 2 for a dialog.
    elevation: u8 => Elevation;
    /// The colour of the text in the box and in what it holds: a colour, or a role.
    text_color: Paint => TextColor;
    /// The size of that text, in pixels.
    text_size: f32 => TextSize;
    italic: bool => Italic;
    /// The height of its lines, as a multiple of the size.
    line_height: f32 => LineHeight;
    /// What stands in the fields of the box where text is to be typed.
    placeholder_color: Paint => PlaceholderColor;
    /// The fill behind selected text in the box.
    selection_color: Paint => SelectionColor;
    /// The carets of the fields in the box.
    caret_color: Paint => CaretColor;
}

impl Div {
    /// Out of the flow: placed by `left`, `top`, `right` and `bottom` against the box around.
    pub fn absolute(self) -> Self {
        self.position(Position::Absolute)
    }
}

impl Refine {
    /// The weight of the text while the state is set.
    pub fn font_weight(mut self, weight: u16) -> Self {
        self.0.push(Patch::FontWeight(weight));
        self
    }
}
