//! Acte II, fin : le Big Bang (3e niveau de prestige) et le Grand Attracteur.
//!
//! Le Big Bang remet à zéro les Big Crunch contre des particules primordiales.
//! Elles donnent un bonus permanent, et se dépensent (sans perdre ce bonus)
//! pour forger les générateurs primordiaux et bâtir le Grand Attracteur,
//! étape par étape, en temps réel (même jeu fermé). Sa dernière étape clôt
//! l'Acte II.

/// Effet d'une étape achevée du Grand Attracteur.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AttractorEffect {
    /// Production multipliée.
    Production(f64),
    /// Fragments multipliés.
    Fragments(f64),
    /// Hors-ligne : rendement minimal (1.0 = 100 %).
    Offline(f64),
    /// Bonus de matière noire multiplié (×(1 + crunches) devient plus fort).
    DarkMatter(f64),
    /// L'éveil : fin de l'Acte II (et production multipliée).
    Awakening(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttractorStage {
    pub index: usize,
    pub name: &'static str,
    /// Ce que ça apporte, en clair.
    pub description: &'static str,
    /// Coût en particules primordiales.
    pub cost: u32,
    /// Durée de construction, en secondes de temps réel.
    pub build_secs: f64,
    pub effect: AttractorEffect,
}

/// Multiplicateur de production de `particles` particules primordiales.
pub fn particle_production_for(particles: u32) -> f64 {
    crate::catalog::balance::PARTICLE_PRODUCTION.powi(particles as i32)
}

/// Particules gagnées par un Big Bang déclenché avec `crunches` Big Crunch.
pub fn particles_for(crunches: u32, per: u32) -> u32 {
    crunches / per.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particles_grow_with_crunches() {
        assert_eq!(particles_for(29, 10), 2);
        assert_eq!(particles_for(30, 10), 3);
        assert_eq!(particles_for(45, 10), 4);
        assert_eq!(particles_for(5, 0), 5, "diviseur nul : sans panique");
    }
}
