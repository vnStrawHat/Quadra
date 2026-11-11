//! Points, sizes, rectangles, edges and corners.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

/// A rectangle by its top-left corner and size. It is half-open: it holds its left and top
/// edges and not its right and bottom ones, so rectangles that only touch do not intersect.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// One value per side: padding, margins, border widths.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// One value per corner: radii.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Corners {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl Point {
    pub const ZERO: Point = Point { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Point {
        Point { x, y }
    }
}

impl Size {
    pub const ZERO: Size = Size { width: 0.0, height: 0.0 };

    pub const fn new(width: f32, height: f32) -> Size {
        Size { width, height }
    }
}

impl Edges {
    pub const fn all(v: f32) -> Edges {
        Edges { top: v, right: v, bottom: v, left: v }
    }

    /// `x` on the left and right, `y` on the top and bottom.
    pub const fn xy(x: f32, y: f32) -> Edges {
        Edges { top: y, right: x, bottom: y, left: x }
    }

    pub fn horizontal(self) -> f32 {
        self.left + self.right
    }

    pub fn vertical(self) -> f32 {
        self.top + self.bottom
    }

    pub fn scale(self, s: f32) -> Edges {
        Edges { top: self.top * s, right: self.right * s, bottom: self.bottom * s, left: self.left * s }
    }
}

impl Corners {
    pub const fn all(v: f32) -> Corners {
        Corners { top_left: v, top_right: v, bottom_right: v, bottom_left: v }
    }

    pub fn scale(self, s: f32) -> Corners {
        Corners {
            top_left: self.top_left * s,
            top_right: self.top_right * s,
            bottom_right: self.bottom_right * s,
            bottom_left: self.bottom_left * s,
        }
    }

    pub fn is_zero(self) -> bool {
        self == Corners::default()
    }
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect { x, y, width, height }
    }

    pub fn right(self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(self) -> f32 {
        self.y + self.height
    }

    pub fn origin(self) -> Point {
        Point::new(self.x, self.y)
    }

    pub fn size(self) -> Size {
        Size::new(self.width, self.height)
    }

    pub fn center(self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    pub fn contains(self, p: Point) -> bool {
        p.x >= self.x && p.y >= self.y && p.x < self.right() && p.y < self.bottom()
    }

    pub fn intersects(self, other: Rect) -> bool {
        self.x < other.right() && other.x < self.right() && self.y < other.bottom() && other.y < self.bottom()
    }

    /// The overlap of the two; an empty rectangle when there is none.
    pub fn intersect(self, other: Rect) -> Rect {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        if right <= x || bottom <= y {
            return Rect::new(x, y, 0.0, 0.0);
        }
        Rect::new(x, y, right - x, bottom - y)
    }

    /// The smallest rectangle holding both. An empty rectangle adds nothing.
    pub fn union(self, other: Rect) -> Rect {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        Rect::new(x, y, self.right().max(other.right()) - x, self.bottom().max(other.bottom()) - y)
    }

    pub fn translate(self, dx: f32, dy: f32) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.width, self.height)
    }

    pub fn scale(self, s: f32) -> Rect {
        Rect::new(self.x * s, self.y * s, self.width * s, self.height * s)
    }

    /// Shrinks by the edges (padding, borders); the size never goes below zero.
    pub fn inset(self, e: Edges) -> Rect {
        Rect::new(
            self.x + e.left,
            self.y + e.top,
            (self.width - e.horizontal()).max(0.0),
            (self.height - e.vertical()).max(0.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_ops() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, 5.0, 10.0, 10.0);
        assert!(a.intersects(b));
        assert_eq!(a.intersect(b), Rect::new(5.0, 5.0, 5.0, 5.0));
        assert_eq!(a.union(b), Rect::new(0.0, 0.0, 15.0, 15.0));
        assert_eq!(Rect::new(0.0, 0.0, 0.0, 5.0).union(b), b);
        let e = Edges { top: 1.0, right: 2.0, bottom: 3.0, left: 4.0 };
        assert_eq!(a.inset(e), Rect::new(4.0, 1.0, 4.0, 6.0));
        assert_eq!(a.inset(Edges::all(8.0)).size(), Size::ZERO);
    }

    #[test]
    fn rects_are_half_open() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(a.contains(Point::ZERO) && !a.contains(Point::new(10.0, 10.0)));
        assert!(!a.intersects(Rect::new(10.0, 0.0, 5.0, 5.0)));
        assert!(a.intersect(Rect::new(10.0, 0.0, 5.0, 5.0)).is_empty());
    }

    #[test]
    fn edges_and_corners() {
        assert_eq!(Edges::xy(2.0, 3.0), Edges { top: 3.0, right: 2.0, bottom: 3.0, left: 2.0 });
        assert_eq!(Edges::xy(2.0, 3.0).scale(2.0).horizontal(), 8.0);
        assert!(Corners::default().is_zero() && !Corners::all(4.0).scale(0.5).is_zero());
    }
}
