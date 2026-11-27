//! Calcul de la production, des multiplicateurs et de la valeur d'un clic.
//!
//! ```text
//! prod_gen(i)  = owned[i] × base_prod[i] × upgrade_mult(i) × 2^(paliers atteints)
//! prod_totale  = Σ prod_gen(i) × global_mult
//! global_mult  = Π(upgrades globaux) × fragment_multiplier(fragments) × (1 + 0.01 × succès) × boost_comète
//! ```

use crate::catalog::balance::{ACHIEVEMENT_BONUS, CLICK_BASE};
use crate::catalog::Catalog;
use crate::challenge::ChallengeReward;
use crate::generator::{milestones_reached, GeneratorId};
use crate::perk::PerkEffect;
use crate::prestige::fragment_multiplier;
use crate::state::GameState;
use crate::upgrade::Effect;

fn bought_effects<'a>(
    catalog: &'a Catalog,
    state: &'a GameState,
) -> impl Iterator<Item = Effect> + 'a {
    catalog
        .upgrades
        .iter()
        .filter(|u| state.upgrades.get(u.id.index()).copied().unwrap_or(false))
        .map(|u| u.effect)
}

/// Multiplicateur propre à un générateur : améliorations × 2^paliers.
pub fn generator_multiplier(catalog: &Catalog, state: &GameState, id: GeneratorId) -> f64 {
    let upgrades: f64 = bought_effects(catalog, state)
        .filter_map(|e| match e {
            Effect::GeneratorMultiplier { generator, factor } if generator == id => Some(factor),
            _ => None,
        })
        .product();
    let owned = state.owned_of(id);
    let challenges: f64 = state
        .challenge_rewards(catalog)
        .filter_map(|r| match r {
            ChallengeReward::EarlyGenerators { count, factor } if id.index() < count => {
                Some(factor)
            }
            _ => None,
        })
        .product();
    upgrades * challenges * 2f64.powi(milestones_reached(owned) as i32)
}

/// Production d'un générateur avant multiplicateur global.
pub fn generator_base_production(catalog: &Catalog, state: &GameState, id: GeneratorId) -> f64 {
    let Some(def) = catalog.generator(id) else {
        return 0.0;
    };
    state.owned_of(id) as f64 * def.base_prod * generator_multiplier(catalog, state, id)
}

/// Multiplicateur global (améliorations, fragments, succès, tempête).
pub fn global_multiplier(catalog: &Catalog, state: &GameState) -> f64 {
    permanent_multiplier(catalog, state) * state.comet.boost()
}

/// Source d'un multiplicateur global de production (détail affiché au joueur).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProductionSource {
    Upgrades,
    Fragments,
    DarkMatter,
    Particles,
    Attractor,
    Nebulae,
    Relics,
    Boosts,
    Megastructures,
    Awakening,
    Convergence,
    Lab,
    Universe,
    Constellations,
    Challenges,
    Achievements,
}

impl ProductionSource {
    pub fn label(self) -> &'static str {
        match self {
            ProductionSource::Upgrades => "Améliorations globales",
            ProductionSource::Fragments => "Fragments de Supernova",
            ProductionSource::DarkMatter => "Matière noire (Big Crunch)",
            ProductionSource::Particles => "Particules (Big Bang)",
            ProductionSource::Attractor => "Grand Attracteur",
            ProductionSource::Nebulae => "Nébuleuses explorées",
            ProductionSource::Relics => "Reliques",
            ProductionSource::Boosts => "Boosts cosmiques",
            ProductionSource::Megastructures => "Mégastructures",
            ProductionSource::Awakening => "Éveil du Cosmos",
            ProductionSource::Convergence => "Convergence",
            ProductionSource::Lab => "Laboratoire",
            ProductionSource::Universe => "Univers en cours",
            ProductionSource::Constellations => "Constellations",
            ProductionSource::Challenges => "Défis réussis",
            ProductionSource::Achievements => "Succès",
        }
    }
}

/// Nombre de sources de [`production_breakdown`].
pub const SOURCES: usize = 16;

/// Détail du multiplicateur global hors tempête : chaque source et son
/// facteur (1 : sans effet). Leur produit est [`permanent_multiplier`].
pub fn production_breakdown(
    catalog: &Catalog,
    state: &GameState,
) -> [(ProductionSource, f64); SOURCES] {
    use ProductionSource as P;
    let upgrades: f64 = bought_effects(catalog, state)
        .filter_map(|e| match e {
            Effect::GlobalMultiplier(f) => Some(f),
            _ => None,
        })
        .product();
    let challenges: f64 = state
        .challenge_rewards(catalog)
        .filter_map(|r| match r {
            ChallengeReward::Production(f) => Some(f),
            _ => None,
        })
        .product();
    [
        (P::Upgrades, upgrades),
        (P::Fragments, fragment_multiplier(state.fragments)),
        (P::DarkMatter, state.crunch_production(catalog)),
        (P::Particles, state.particle_production()),
        (P::Attractor, state.attractor_production(catalog)),
        (
            P::Nebulae,
            state.map_bonus(crate::starmap::SystemKind::Nebula, 0.10),
        ),
        (P::Relics, state.relic_production(catalog)),
        (
            P::Boosts,
            state.boost_factor(crate::cosmic::BoostKind::Production),
        ),
        (P::Megastructures, state.grid_effects().production),
        (
            P::Awakening,
            if state.awakened {
                crate::megastructure::AWAKENED_PRODUCTION
            } else {
                1.0
            },
        ),
        (
            P::Convergence,
            if state.converged {
                crate::cosmic::CONVERGENCE_PRODUCTION
            } else {
                1.0
            },
        ),
        (P::Lab, state.lab_production(catalog)),
        (P::Universe, state.universe_production(catalog)),
        (P::Constellations, state.constellation_production(catalog)),
        (P::Challenges, challenges),
        (
            P::Achievements,
            1.0 + ACHIEVEMENT_BONUS * state.achievements_unlocked() as f64,
        ),
    ]
}

/// Multiplicateur global hors tempête solaire.
pub fn permanent_multiplier(catalog: &Catalog, state: &GameState) -> f64 {
    production_breakdown(catalog, state)
        .iter()
        .map(|(_, f)| f)
        .product()
}

/// Production effective d'un générateur (multiplicateur global inclus).
pub fn generator_production(catalog: &Catalog, state: &GameState, id: GeneratorId) -> f64 {
    generator_base_production(catalog, state, id) * global_multiplier(catalog, state)
}

/// Production totale par seconde.
pub fn total_production(catalog: &Catalog, state: &GameState) -> f64 {
    let sum: f64 = catalog
        .generators
        .iter()
        .map(|g| generator_base_production(catalog, state, g.id))
        .sum();
    sum * global_multiplier(catalog, state)
}

/// Production par seconde hors tempête (utilisée pour les gains hors-ligne).
pub fn steady_production(catalog: &Catalog, state: &GameState) -> f64 {
    let sum: f64 = catalog
        .generators
        .iter()
        .map(|g| generator_base_production(catalog, state, g.id))
        .sum();
    sum * permanent_multiplier(catalog, state)
}

/// Fraction de la prod/s ajoutée à chaque clic.
pub fn click_percent(catalog: &Catalog, state: &GameState) -> f64 {
    let upgrades: f64 = bought_effects(catalog, state)
        .filter_map(|e| match e {
            Effect::ClickPercent(p) => Some(p),
            _ => None,
        })
        .sum();
    let perks: f64 = state
        .perk_effects(catalog)
        .filter_map(|e| match e {
            PerkEffect::ClickBoost(p) => Some(p),
            _ => None,
        })
        .sum();
    upgrades
        + perks
        + state.constellation_click(catalog)
        + state.lab_click(catalog)
        + state.relic_sum(catalog, |e| match e {
            crate::relic::RelicEffect::Click(x) => Some(x),
            _ => None,
        })
}

/// Vrai si la récolte « touche maintenue » est débloquée.
pub fn hold_harvest_unlocked(catalog: &Catalog, state: &GameState) -> bool {
    bought_effects(catalog, state).any(|e| e == Effect::HoldHarvest)
}

/// Gain d'un clic : `1 + click_pct × production_par_seconde`.
pub fn click_value(catalog: &Catalog, state: &GameState) -> f64 {
    CLICK_BASE + click_percent(catalog, state) * total_production(catalog, state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_breakdown_is_the_multiplier() {
        let c = Catalog::standard();
        let mut s = GameState::new(&c);
        s.crunches = 2;
        s.fragments = 40.0;
        let parts = production_breakdown(&c, &s);
        let product: f64 = parts.iter().map(|(_, f)| f).product();
        assert!((product - permanent_multiplier(&c, &s)).abs() < 1e-9 * product);
        let dark = parts
            .iter()
            .find(|(p, _)| *p == ProductionSource::DarkMatter)
            .unwrap();
        assert_eq!(dark.1, 3.0, "2 Big Crunch : ×3");
    }

    fn setup() -> (Catalog, GameState) {
        let c = Catalog::standard();
        let s = GameState::new(&c);
        (c, s)
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
    }

    #[test]
    fn base_production() {
        let (c, mut s) = setup();
        assert_eq!(total_production(&c, &s), 0.0);
        s.owned[0] = 10;
        s.owned[1] = 2;
        assert!(close(total_production(&c, &s), 10.0 * 0.1 + 2.0 * 1.0));
    }

    #[test]
    fn milestones_double() {
        let (c, mut s) = setup();
        s.owned[0] = 25;
        assert!(close(total_production(&c, &s), 25.0 * 0.1 * 2.0));
        s.owned[0] = 100;
        assert!(close(total_production(&c, &s), 100.0 * 0.1 * 8.0));
    }

    #[test]
    fn upgrades_fragments_achievements_and_storm() {
        let (c, mut s) = setup();
        s.owned[1] = 10;
        let t0 = c.upgrade_by_key("gen1_t0").unwrap();
        let g0 = c.upgrade_by_key("global_0").unwrap();
        s.upgrades[t0.index()] = true;
        s.upgrades[g0.index()] = true;
        s.fragments = 2.0;
        s.achievements[0] = true;
        let expected = 10.0 * 1.0 * 2.0 * 1.5 * 1.02 * 1.01;
        assert!(close(total_production(&c, &s), expected));
        assert!(close(steady_production(&c, &s), expected));
        s.comet.start_storm(30.0);
        assert!(close(total_production(&c, &s), expected * 7.0));
        assert!(close(steady_production(&c, &s), expected));
    }

    #[test]
    fn click_value_uses_percent_of_production() {
        let (c, mut s) = setup();
        assert_eq!(click_value(&c, &s), 1.0);
        s.owned[1] = 100;
        let k = c.upgrade_by_key("click_0").unwrap();
        s.upgrades[k.index()] = true;
        let prod = total_production(&c, &s);
        assert!(close(click_value(&c, &s), 1.0 + 0.01 * prod));
    }
}
