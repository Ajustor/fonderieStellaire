//! Supernova (prestige) : formule des fragments.

use crate::catalog::balance::{FRAGMENT_BONUS, FRAGMENT_DIVISOR};

/// Fragments correspondant à un total de poussière récoltée dans le cycle.
pub fn fragments_for(total_run: f64) -> f64 {
    if !total_run.is_finite() || total_run <= 0.0 {
        return 0.0;
    }
    (total_run / FRAGMENT_DIVISOR).sqrt().floor()
}

/// Fragments gagnables maintenant : `floor(sqrt(total_run / 1e8)) − déjà gagnés ce cycle`.
pub fn fragments_available(total_run: f64, fragments_this_run: f64) -> f64 {
    (fragments_for(total_run) - fragments_this_run).max(0.0)
}

/// Multiplicateur de production apporté par les fragments (+5 % chacun).
pub fn fragment_multiplier(fragments: f64) -> f64 {
    1.0 + FRAGMENT_BONUS * fragments.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragment_formula() {
        assert_eq!(fragments_for(0.0), 0.0);
        assert_eq!(fragments_for(9.9e7), 0.0);
        assert_eq!(fragments_for(1e8), 1.0);
        assert_eq!(fragments_for(4e8), 2.0);
        assert_eq!(fragments_for(8.9e8), 2.0);
        assert_eq!(fragments_for(9e8), 3.0);
        assert_eq!(fragments_for(f64::NAN), 0.0);
    }

    #[test]
    fn available_subtracts_already_earned() {
        assert_eq!(fragments_available(9e8, 1.0), 2.0);
        assert_eq!(fragments_available(1e8, 5.0), 0.0);
    }

    #[test]
    fn multiplier() {
        assert_eq!(fragment_multiplier(0.0), 1.0);
        assert!((fragment_multiplier(12.0) - 1.6).abs() < 1e-12);
    }
}
