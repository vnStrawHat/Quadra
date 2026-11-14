//! What of a style can ease, and going between two of them.

use quadra_prim::{Color, Point};

use crate::node::{Pose, Style};
use crate::patch::Patch;
use crate::theme::Theme;

/// What of a style can ease.
#[derive(Clone, Copy, PartialEq)]
pub(super) struct Eased {
    pub background: Color,
    pub border: Color,
    pub pose: Pose,
}

impl Eased {
    pub fn of(style: &Style) -> Eased {
        Eased { background: style.paint.background, border: style.paint.border_color, pose: style.pose }
    }

    pub fn into(self, style: &mut Style) {
        (style.paint.background, style.paint.border_color, style.pose) =
            (self.background, self.border, self.pose);
    }

    pub fn lerp(self, to: Eased, t: f32) -> Eased {
        let mix = |a: f32, b: f32| a + (b - a) * t;
        let (a, b) = (self.pose, to.pose);
        let pose = Pose {
            translate: Point::new(mix(a.translate.x, b.translate.x), mix(a.translate.y, b.translate.y)),
            scale: mix(a.scale, b.scale),
            rotate: mix(a.rotate, b.rotate),
            opacity: mix(a.opacity, b.opacity),
        };
        Eased {
            background: self.background.lerp(to.background, t),
            border: self.border.lerp(to.border, t),
            pose,
        }
    }

    /// With what a step sets.
    pub fn with(mut self, patches: &[Patch], theme: &Theme) -> Eased {
        for patch in patches {
            match *patch {
                Patch::Background(v) => self.background = theme.resolve(v),
                Patch::BorderColor(v) => self.border = theme.resolve(v),
                Patch::Translate(v) => self.pose.translate = v,
                Patch::Scale(v) => self.pose.scale = v,
                Patch::Rotate(v) => self.pose.rotate = v,
                Patch::Opacity(v) => self.pose.opacity = v,
                _ => {} // nothing else eases
            }
        }
        self
    }
}
