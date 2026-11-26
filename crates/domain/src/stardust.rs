//! La ressource principale : la poussière d'étoile.

use std::ops::{Add, AddAssign};

/// Quantité de poussière d'étoile. Toujours finie et positive.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Stardust(f64);

impl Stardust {
    pub const ZERO: Stardust = Stardust(0.0);

    /// Construit une quantité en neutralisant les valeurs invalides (NaN, infini, négatif).
    pub fn new(value: f64) -> Self {
        Self(sanitize(value))
    }

    pub fn value(self) -> f64 {
        self.0
    }

    /// Vrai si l'on peut payer `cost` (avec une petite tolérance sur les erreurs d'arrondi).
    pub fn can_afford(self, cost: f64) -> bool {
        cost.is_finite() && self.0 >= cost - cost.abs() * 1e-9
    }

    /// Dépense `cost` si possible. Renvoie `false` sans rien modifier sinon.
    pub fn spend(&mut self, cost: f64) -> bool {
        if !self.can_afford(cost) {
            return false;
        }
        self.0 = sanitize(self.0 - cost);
        true
    }
}

impl Add<f64> for Stardust {
    type Output = Stardust;
    fn add(self, rhs: f64) -> Stardust {
        Stardust::new(self.0 + sanitize(rhs))
    }
}

impl AddAssign<f64> for Stardust {
    fn add_assign(&mut self, rhs: f64) {
        *self = *self + rhs;
    }
}

fn sanitize(v: f64) -> f64 {
    if v.is_finite() && v > 0.0 {
        v
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_values_become_zero() {
        assert_eq!(Stardust::new(f64::NAN).value(), 0.0);
        assert_eq!(Stardust::new(-5.0).value(), 0.0);
        assert_eq!(Stardust::new(f64::INFINITY).value(), 0.0);
    }

    #[test]
    fn spend_only_when_affordable() {
        let mut s = Stardust::new(10.0);
        assert!(!s.spend(11.0));
        assert_eq!(s.value(), 10.0);
        assert!(s.spend(4.0));
        assert_eq!(s.value(), 6.0);
    }

    #[test]
    fn add_ignores_garbage() {
        let mut s = Stardust::new(1.0);
        s += f64::NAN;
        s += 2.0;
        assert_eq!(s.value(), 3.0);
    }
}
