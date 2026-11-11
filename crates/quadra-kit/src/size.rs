//! The numbers of the design language (specs/behaviour/15, "Type" and "Space, size, shape"):
//! every one a pixel at 100 % scale.

/// A step of the type scale: the size of the text, and the height of its lines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Type {
    pub size: f32,
    pub line: f32,
}

impl Type {
    /// The line height as the multiple of the size a style takes.
    pub fn line_height(self) -> f32 {
        self.line / self.size
    }
}

/// Badges and hints.
pub const TEXT_XS: Type = Type { size: 10.0, line: 16.0 };
/// Meta, the status bar, small buttons.
pub const TEXT_SM: Type = Type { size: 12.0, line: 19.0 };
/// Body, controls, menus, tabs.
pub const TEXT_MD: Type = Type { size: 14.0, line: 23.0 };
/// Large labels.
pub const TEXT_LG: Type = Type { size: 16.0, line: 26.0 };
/// Headlines of dialogs and pages.
pub const TEXT_XL: Type = Type { size: 18.0, line: 26.0 };
/// Display: the first heading of a page.
pub const TEXT_2XL: Type = Type { size: 23.0, line: 29.0 };

/// The heights of controls, at the compact density, which is the reference.
pub const H_XS: f32 = 18.0;
pub const H_SM: f32 = 22.0;
pub const H_MD: f32 = 28.0;
pub const H_LG: f32 = 32.0;

/// Checkboxes and keys.
pub const RADIUS_SM: f32 = 2.0;
/// Buttons, list rows, chips.
pub const RADIUS_MD: f32 = 4.0;
/// Inputs, segmented groups, code blocks.
pub const RADIUS_LG: f32 = 6.0;
/// Every overlay: menus, tooltips, popovers, dialogs, toasts.
pub const RADIUS_XL: f32 = 8.0;

/// Regular text, and what is emphasized or a heading.
pub const WEIGHT_NORMAL: u16 = 400;
pub const WEIGHT_STRONG: u16 = 600;
