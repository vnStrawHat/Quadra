//! Easing: how far along a change is, for how much of its time has passed.

/// A curve from 0 to 1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Ease {
    /// The same speed all the way.
    Linear,
    /// Slow, then fast.
    In,
    /// Fast, then slow: what arrives.
    #[default]
    Out,
    /// Slow at both ends.
    InOut,
}

impl Ease {
    /// How far along the change is when `t` of its time has passed. `t` outside 0 to 1 is
    /// taken as the nearer end.
    pub fn at(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Ease::Linear => t,
            Ease::In => t * t * t,
            Ease::Out => 1.0 - (1.0 - t).powi(3),
            Ease::InOut if t < 0.5 => 4.0 * t * t * t,
            Ease::InOut => 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Ease;

    #[test]
    fn every_curve_goes_from_nought_to_one_and_never_back() {
        for ease in [Ease::Linear, Ease::In, Ease::Out, Ease::InOut] {
            assert_eq!((ease.at(-1.0), ease.at(0.0), ease.at(1.0), ease.at(2.0)), (0.0, 0.0, 1.0, 1.0));
            let steps: Vec<f32> = (0..=20).map(|i| ease.at(i as f32 / 20.0)).collect();
            assert!(steps.windows(2).all(|pair| pair[0] <= pair[1]), "{ease:?}: {steps:?}");
        }
        assert_eq!((Ease::Linear.at(0.5), Ease::InOut.at(0.5)), (0.5, 0.5));
        assert!(Ease::In.at(0.5) < 0.5 && Ease::Out.at(0.5) > 0.5);
    }
}
