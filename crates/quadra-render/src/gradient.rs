//! Gradients (specs/07, "Instances"): colours at stops along a box, mixed in Oklab and baked
//! into a ramp, a row of texels of the colour atlas that the shader samples.

use std::hash::{Hash, Hasher};

use quadra_prim::Color;

/// How many texels a ramp has.
pub const RAMP: u32 = 256;

/// Which way a gradient goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GradientKind {
    /// Along a line at an angle in degrees: 0 from left to right, 90 from top to bottom.
    Linear(f32),
    /// Out from the middle to the corners, an ellipse of the box's shape.
    Radial,
    /// Round the middle, from an angle in degrees: 0 at the top, clockwise.
    Conic(f32),
}

/// Colours at stops from 0 to 1 along a gradient.
#[derive(Clone, Debug, PartialEq)]
pub struct Gradient {
    pub kind: GradientKind,
    pub stops: Vec<(f32, Color)>,
}

/// Stops spread evenly when none says where it is.
fn even(colors: impl IntoIterator<Item = Color>) -> Vec<(f32, Color)> {
    let colors: Vec<Color> = colors.into_iter().collect();
    let last = colors.len().saturating_sub(1).max(1) as f32;
    colors.into_iter().enumerate().map(|(i, color)| (i as f32 / last, color)).collect()
}

impl Gradient {
    /// Colours spread evenly along a line at `angle` degrees.
    pub fn linear(angle: f32, colors: impl IntoIterator<Item = Color>) -> Gradient {
        Gradient { kind: GradientKind::Linear(angle), stops: even(colors) }
    }

    /// Colours spread evenly from the middle out.
    pub fn radial(colors: impl IntoIterator<Item = Color>) -> Gradient {
        Gradient { kind: GradientKind::Radial, stops: even(colors) }
    }

    /// Colours spread evenly round the middle, from `from` degrees.
    pub fn conic(from: f32, colors: impl IntoIterator<Item = Color>) -> Gradient {
        Gradient { kind: GradientKind::Conic(from), stops: even(colors) }
    }

    /// The same way with stops placed where they say: `(where, colour)`, ascending.
    pub fn stops(self, stops: impl IntoIterator<Item = (f32, Color)>) -> Gradient {
        Gradient { stops: stops.into_iter().collect(), ..self }
    }

    /// What the ramp is known by: its stops, whatever the way.
    pub fn key(&self) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for (at, c) in &self.stops {
            [at, &c.r, &c.g, &c.b, &c.a].map(|v| v.to_bits()).hash(&mut h);
        }
        h.finish()
    }

    /// The colour at `t` along the stops: the first before them, the last after.
    pub fn at(&self, t: f32) -> Color {
        let Some(&(first, color)) = self.stops.first() else { return Color::TRANSPARENT };
        if t <= first {
            return color;
        }
        for pair in self.stops.windows(2) {
            let ((a, from), (b, to)) = (pair[0], pair[1]);
            if t <= b {
                return if b > a { from.mix_oklab(to, (t - a) / (b - a)) } else { to };
            }
        }
        self.stops[self.stops.len() - 1].1
    }

    /// The ramp's texels: RGBA, premultiplied, a byte a channel, as the colour atlas holds.
    pub fn ramp(&self) -> Vec<u8> {
        let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        let texel = |i: u32| {
            let c = self.at(i as f32 / (RAMP - 1) as f32);
            [byte(c.r * c.a), byte(c.g * c.a), byte(c.b * c.a), byte(c.a)]
        };
        (0..RAMP).flat_map(texel).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ramp_runs_through_its_stops_and_holds_past_the_ends() {
        let (red, blue) = (Color::rgb(0xFF0000), Color::rgb(0x0000FF));
        let g = Gradient::linear(0.0, [red, blue]).stops([(0.25, red), (0.75, blue)]);
        assert_eq!((g.at(0.0), g.at(0.25), g.at(1.0)), (red, red, blue));
        assert_eq!(g.at(0.5).hex(), "#8c53a2ff");
        let ramp = g.ramp();
        assert_eq!(
            (ramp.len(), &ramp[..4], &ramp[ramp.len() - 4..]),
            (1024, &[255, 0, 0, 255][..], &[0, 0, 255, 255][..])
        );
        // Premultiplied: a stop that is half clear is half as bright.
        let faded = Gradient::radial([red.alpha(0.5)]).ramp();
        assert_eq!(&faded[..4], &[128, 0, 0, 128]);
        assert_eq!(Gradient::conic(0.0, [red, blue, red]).stops[1].0, 0.5);
        assert_ne!(g.key(), Gradient::linear(0.0, [red, blue]).key());
    }
}
