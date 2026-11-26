//! Supernova (prestige) : formule des fragments.

use crate::catalog::balance::{FRAGMENT_BONUS, FRAGMENT_DIVISOR, FRAGMENT_KNEE, FRAGMENT_SCALING};

/// Seuil de la formule des fragments après `supernovas` Supernovas :
/// chaque Supernova rend la suivante 25 % plus exigeante.
pub fn fragment_divisor(supernovas: u32) -> f64 {
    FRAGMENT_DIVISOR * (1.0 + FRAGMENT_SCALING * supernovas as f64)
}

/// Fragments correspondant à `total_run` récoltés : `floor(√(total_run / seuil))`.
pub fn fragments_for(total_run: f64, supernovas: u32) -> f64 {
    if !total_run.is_finite() || total_run <= 0.0 {
        return 0.0;
    }
    (total_run / fragment_divisor(supernovas)).sqrt().floor()
}

/// Fragments gagnables maintenant, moins ceux déjà gagnés ce cycle.
pub fn fragments_available(total_run: f64, supernovas: u32, fragments_this_run: f64) -> f64 {
    (fragments_for(total_run, supernovas) - fragments_this_run).max(0.0)
}

/// Multiplicateur de production des fragments : +10 % chacun jusqu'à 5, puis
/// croissance en racine carrée (courbe continue). Évite l'emballement : plus de
/// fragments rapporte toujours plus, mais de moins en moins vite.
pub fn fragment_multiplier(fragments: f64) -> f64 {
    let f = fragments.max(0.0);
    if f <= FRAGMENT_KNEE {
        1.0 + FRAGMENT_BONUS * f
    } else {
        1.0 + FRAGMENT_BONUS * FRAGMENT_KNEE * (f / FRAGMENT_KNEE).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragment_formula() {
        // floor(sqrt(total_run / 1e7)) avant la première Supernova.
        assert_eq!(fragments_for(0.0, 0), 0.0);
        assert_eq!(fragments_for(9.9e6, 0), 0.0);
        assert_eq!(fragments_for(1e7, 0), 1.0);
        assert_eq!(fragments_for(9e7, 0), 3.0);
        assert_eq!(fragments_for(1e9, 0), 10.0);
        assert_eq!(fragments_for(f64::NAN, 0), 0.0);
    }

    #[test]
    fn each_supernova_raises_the_bar() {
        assert_eq!(fragment_divisor(0), 1e7);
        assert_eq!(fragment_divisor(4), 2e7);
        assert_eq!(fragments_for(1e9, 0), 10.0);
        assert_eq!(fragments_for(1e9, 4), 7.0); // sqrt(50) = 7.07
    }

    #[test]
    fn available_subtracts_already_earned() {
        assert_eq!(fragments_available(9e7, 0, 1.0), 2.0);
        assert_eq!(fragments_available(1e7, 0, 5.0), 0.0);
    }

    #[test]
    fn multiplier_is_linear_then_sqrt() {
        assert_eq!(fragment_multiplier(0.0), 1.0);
        assert!((fragment_multiplier(5.0) - 1.5).abs() < 1e-12);
        assert!((fragment_multiplier(20.0) - 2.0).abs() < 1e-12); // 1 + 0.5 × √4
        assert!((fragment_multiplier(80.0) - 3.0).abs() < 1e-12); // 1 + 0.5 × √16
                                                                  // Continue et croissante.
        assert!(fragment_multiplier(5.01) > fragment_multiplier(5.0));
        assert!(fragment_multiplier(1e6) > fragment_multiplier(1e5));
    }
}
