//! Fonctions d'easing : `t ∈ [0, 1] → [0, 1]` (avec dépassement pour back/elastic).

use std::f64::consts::PI;

pub fn linear(t: f64) -> f64 {
    t.clamp(0.0, 1.0)
}

pub fn ease_in_cubic(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * t
}

pub fn ease_out_cubic(t: f64) -> f64 {
    let t = 1.0 - t.clamp(0.0, 1.0);
    1.0 - t * t * t
}

pub fn ease_in_out_sine(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    -((PI * t).cos() - 1.0) / 2.0
}

pub fn ease_out_back(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

pub fn ease_out_elastic(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    if t == 0.0 || t == 1.0 {
        return t;
    }
    let c4 = (2.0 * PI) / 3.0;
    2f64.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
}

pub fn ease_out_bounce(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    let n1 = 7.5625;
    let d1 = 2.75;
    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        let t = t - 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        let t = t - 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / d1;
        n1 * t * t + 0.984375
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Easing {
    #[default]
    Linear,
    InCubic,
    OutCubic,
    InOutSine,
    OutBack,
    OutElastic,
    OutBounce,
}

impl Easing {
    pub fn apply(self, t: f64) -> f64 {
        match self {
            Easing::Linear => linear(t),
            Easing::InCubic => ease_in_cubic(t),
            Easing::OutCubic => ease_out_cubic(t),
            Easing::InOutSine => ease_in_out_sine(t),
            Easing::OutBack => ease_out_back(t),
            Easing::OutElastic => ease_out_elastic(t),
            Easing::OutBounce => ease_out_bounce(t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Easing; 7] = [
        Easing::Linear,
        Easing::InCubic,
        Easing::OutCubic,
        Easing::InOutSine,
        Easing::OutBack,
        Easing::OutElastic,
        Easing::OutBounce,
    ];

    #[test]
    fn endpoints() {
        for e in ALL {
            assert!(e.apply(0.0).abs() < 1e-9, "{e:?}(0)");
            assert!((e.apply(1.0) - 1.0).abs() < 1e-9, "{e:?}(1)");
        }
    }

    #[test]
    fn clamped_outside_range() {
        for e in ALL {
            assert!((e.apply(2.0) - 1.0).abs() < 1e-9);
            assert!(e.apply(-1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn shapes() {
        assert!(ease_out_cubic(0.5) > 0.5, "rapide au début");
        assert!(ease_in_cubic(0.5) < 0.5, "lent au début");
        assert!((ease_in_out_sine(0.5) - 0.5).abs() < 1e-9);
        let overshoot = (1..100)
            .map(|i| ease_out_back(i as f64 / 100.0))
            .fold(0.0, f64::max);
        assert!(overshoot > 1.0, "back dépasse la cible");
        for i in 0..=100 {
            let v = ease_out_bounce(i as f64 / 100.0);
            assert!((0.0..=1.0 + 1e-9).contains(&v));
        }
    }

    #[test]
    fn monotonic_where_expected() {
        for e in [
            Easing::Linear,
            Easing::InCubic,
            Easing::OutCubic,
            Easing::InOutSine,
        ] {
            let mut prev = -1.0;
            for i in 0..=50 {
                let v = e.apply(i as f64 / 50.0);
                assert!(v >= prev);
                prev = v;
            }
        }
    }
}
