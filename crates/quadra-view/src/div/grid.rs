//! A box laid out as a grid (specs/06, "Grid"): its columns, and where a child goes in it.

use quadra_layout::{Display, GridStyle, Track};

use super::Div;

impl Div {
    fn grid_style(&mut self) -> &mut GridStyle {
        self.style.layout.grid.get_or_insert_default()
    }

    /// Lays the box's children out in these columns, row after row.
    pub fn grid(mut self, columns: impl IntoIterator<Item = Track>) -> Self {
        self.style.layout.display = Display::Grid;
        self.grid_style().columns = columns.into_iter().collect();
        self
    }

    /// Lays the box's children out in as many equal columns of at least `least` px as fit:
    /// cards that take the room there is.
    pub fn grid_fill(mut self, least: f32) -> Self {
        self.style.layout.display = Display::Grid;
        self.grid_style().fill_min = least;
        self
    }

    /// In a grid, makes the box as wide as so many columns.
    pub fn col_span(mut self, columns: usize) -> Self {
        self.grid_style().col_span = columns.max(1);
        self
    }
}
