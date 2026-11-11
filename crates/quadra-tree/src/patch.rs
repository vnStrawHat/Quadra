//! One value of a style (specs/13, "Style values"): plain data, a few words. A reactive
//! property writes one, a state variant is a list of them, and a role of the theme is one
//! that stands for another colour under another theme.

use quadra_layout::{Align, Direction, Display, Justify, Length, Overflow, Position};
use quadra_prim::{Corners, Edges};

use crate::node::Style;
use crate::theme::{Paint, Theme};

/// One value of a style: what a reactive property writes, and what a state variant is a list
/// of. Each is named after the setter of the box view that makes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Patch {
    Width(Length),
    Height(Length),
    MinWidth(Length),
    MinHeight(Length),
    MaxWidth(Length),
    MaxHeight(Length),
    Grow(f32),
    Shrink(f32),
    /// The space between children, on both axes.
    Gap(f32),
    /// The same on every side.
    Padding(f32),
    PaddingX(f32),
    PaddingY(f32),
    PaddingTop(f32),
    PaddingRight(f32),
    PaddingBottom(f32),
    PaddingLeft(f32),
    Margin(f32),
    /// The width of the border on every side.
    Border(f32),
    /// The width of the border on one side.
    BorderTop(f32),
    BorderRight(f32),
    BorderBottom(f32),
    BorderLeft(f32),
    Direction(Direction),
    /// In the flow, or out of it and placed by its insets against the box around it.
    Position(Position),
    /// How far an edge of a box out of the flow is from that edge of the box around it.
    Left(Length),
    Top(Length),
    Right(Length),
    Bottom(Length),
    /// Laid out as a row or column, as a grid, or not at all.
    Display(Display),
    Justify(Justify),
    AlignItems(Align),
    Wrap(bool),
    OverflowX(Overflow),
    OverflowY(Overflow),
    Background(Paint),
    BorderColor(Paint),
    /// The same radius on every corner.
    Radius(f32),
    /// A line of one pixel along the edge, taking no room.
    Outline(Paint),
    /// The shadow of a level above the window: 0 none, 1 a menu's, 2 a dialog's.
    Elevation(u8),
    /// The colour of the text in the node and under it.
    TextColor(Paint),
    /// Its size, in pixels.
    TextSize(f32),
    FontWeight(u16),
    Italic(bool),
    /// The height of a line, as a multiple of the size.
    LineHeight(f32),
    /// What stands where text is to be typed, the fill behind selected text, and the caret.
    PlaceholderColor(Paint),
    SelectionColor(Paint),
    CaretColor(Paint),
}

impl Patch {
    /// Sets the value in a style, a role as the colour the theme gives it.
    pub fn apply(self, style: &mut Style, theme: &Theme) {
        let (l, p, t) = (&mut style.layout, &mut style.paint, &mut style.text);
        match self {
            Patch::Background(v) => p.background = theme.resolve(v),
            Patch::BorderColor(v) => p.border_color = theme.resolve(v),
            Patch::Radius(v) => p.radii = Corners::all(v),
            Patch::Outline(v) => p.outline = theme.resolve(v),
            Patch::Elevation(v) => p.elevation = v,
            Patch::TextColor(v) => t.color = Some(theme.resolve(v)),
            Patch::TextSize(v) => t.size = Some(v),
            Patch::FontWeight(v) => t.weight = Some(v),
            Patch::Italic(v) => t.italic = Some(v),
            Patch::LineHeight(v) => t.line_height = Some(v),
            Patch::PlaceholderColor(v) => t.placeholder = Some(theme.resolve(v)),
            Patch::SelectionColor(v) => t.selection = Some(theme.resolve(v)),
            Patch::CaretColor(v) => t.caret = Some(theme.resolve(v)),
            layout => match layout {
                Patch::Width(v) => l.width = v,
                Patch::Height(v) => l.height = v,
                Patch::MinWidth(v) => l.min_width = v,
                Patch::MinHeight(v) => l.min_height = v,
                Patch::MaxWidth(v) => l.max_width = v,
                Patch::MaxHeight(v) => l.max_height = v,
                Patch::Grow(v) => l.grow = v,
                Patch::Shrink(v) => l.shrink = v,
                Patch::Gap(v) => (l.gap_row, l.gap_col) = (v, v),
                Patch::Padding(v) => l.padding = Edges::all(v),
                Patch::PaddingX(v) => (l.padding.left, l.padding.right) = (v, v),
                Patch::PaddingY(v) => (l.padding.top, l.padding.bottom) = (v, v),
                Patch::PaddingTop(v) => l.padding.top = v,
                Patch::PaddingRight(v) => l.padding.right = v,
                Patch::PaddingBottom(v) => l.padding.bottom = v,
                Patch::PaddingLeft(v) => l.padding.left = v,
                Patch::Margin(v) => l.margin = Edges::all(v),
                Patch::Border(v) => l.border = Edges::all(v),
                Patch::BorderTop(v) => l.border.top = v,
                Patch::BorderRight(v) => l.border.right = v,
                Patch::BorderBottom(v) => l.border.bottom = v,
                Patch::BorderLeft(v) => l.border.left = v,
                Patch::Direction(v) => l.direction = v,
                Patch::Display(v) => l.display = v,
                Patch::Position(v) => l.position = v,
                Patch::Left(v) => l.inset.left = v,
                Patch::Top(v) => l.inset.top = v,
                Patch::Right(v) => l.inset.right = v,
                Patch::Bottom(v) => l.inset.bottom = v,
                Patch::Justify(v) => l.justify = v,
                Patch::AlignItems(v) => l.align_items = v,
                Patch::Wrap(v) => l.wrap = v,
                Patch::OverflowX(v) => l.overflow_x = v,
                Patch::OverflowY(v) => l.overflow_y = v,
                _ => {}
            },
        }
    }

    /// Which part of a style the value is of.
    pub(crate) fn part(self) -> Part {
        use Patch::*;
        match self {
            Background(_) | BorderColor(_) | Radius(_) | Outline(_) | Elevation(_) => Part::Paint,
            TextColor(_) | TextSize(_) | FontWeight(_) | Italic(_) | LineHeight(_) => Part::Text,
            PlaceholderColor(_) | SelectionColor(_) | CaretColor(_) => Part::Text,
            _ => Part::Layout,
        }
    }

    /// Whether the value is a role of the theme: it stands for another colour under
    /// another theme.
    pub fn themed(self) -> bool {
        use Patch::*;
        let (Background(paint) | BorderColor(paint) | TextColor(paint) | Outline(paint)) = self else {
            let (PlaceholderColor(paint) | SelectionColor(paint) | CaretColor(paint)) = self else {
                return false;
            };
            return matches!(paint, Paint::Token(..));
        };
        matches!(paint, Paint::Token(..))
    }
}

/// The parts of a style: what a change of each one sets off.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Part {
    Layout,
    Paint,
    Text,
}
