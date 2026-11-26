//! Comètes : apparition, passage, capture et récompenses.

/// Délai avant la toute première comète.
pub const FIRST_DELAY: f64 = 45.0;
/// Intervalle minimal entre deux comètes.
pub const MIN_INTERVAL: f64 = 90.0;
/// Intervalle maximal entre deux comètes.
pub const MAX_INTERVAL: f64 = 240.0;
/// Durée de la traversée du ciel.
pub const CROSSING_TIME: f64 = 15.0;
/// Multiplicateur de la Tempête solaire.
pub const STORM_MULTIPLIER: f64 = 7.0;
/// Durée de la Tempête solaire.
pub const STORM_DURATION: f64 = 30.0;

/// Récompense d'une comète capturée.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CometReward {
    /// Production ×7 pendant 30 s.
    SolarStorm { multiplier: f64, duration: f64 },
    /// Gain immédiat.
    StarShower { amount: f64 },
}

impl CometReward {
    /// Tire la récompense (50/50) à partir d'un tirage uniforme dans [0, 1).
    pub fn from_roll(roll: f64, stardust: f64, production: f64) -> Self {
        if roll < 0.5 {
            CometReward::SolarStorm {
                multiplier: STORM_MULTIPLIER,
                duration: STORM_DURATION,
            }
        } else {
            CometReward::StarShower {
                amount: star_shower_amount(stardust, production),
            }
        }
    }
}

/// Gain de la Pluie d'étoiles : `min(stardust × 0.15, prod × 900) + 13`.
pub fn star_shower_amount(stardust: f64, production: f64) -> f64 {
    (stardust * 0.15).min(production * 900.0) + 13.0
}

/// Intervalle avant la prochaine comète pour un tirage dans [0, 1).
pub fn interval_from_roll(roll: f64) -> f64 {
    MIN_INTERVAL + roll.clamp(0.0, 1.0) * (MAX_INTERVAL - MIN_INTERVAL)
}

/// Ce qui s'est passé pendant un pas de temps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CometTick {
    pub spawned: bool,
    pub missed: bool,
    pub storm_ended: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CometState {
    /// Secondes avant la prochaine apparition (quand aucune comète n'est active).
    pub next_in: f64,
    /// Temps écoulé depuis l'apparition de la comète active.
    pub active: Option<f64>,
    /// Temps restant de Tempête solaire.
    pub storm_remaining: f64,
    /// Multiplie l'intervalle entre deux comètes (Atelier : « Aimant à comètes »).
    pub interval_factor: f64,
}

impl Default for CometState {
    fn default() -> Self {
        CometState {
            next_in: FIRST_DELAY,
            active: None,
            storm_remaining: 0.0,
            interval_factor: 1.0,
        }
    }
}

impl CometState {
    /// Multiplicateur de production apporté par une tempête en cours.
    pub fn boost(&self) -> f64 {
        if self.storm_remaining > 0.0 {
            STORM_MULTIPLIER
        } else {
            1.0
        }
    }

    /// Avancement de la comète active dans sa traversée (0 → 1).
    pub fn progress(&self) -> Option<f64> {
        self.active.map(|e| (e / CROSSING_TIME).clamp(0.0, 1.0))
    }

    /// Fait avancer le temps. `roll` sert à tirer l'intervalle suivant si besoin.
    pub fn advance(&mut self, dt: f64, roll: f64) -> CometTick {
        let mut tick = CometTick::default();
        if self.storm_remaining > 0.0 {
            self.storm_remaining -= dt;
            if self.storm_remaining <= 0.0 {
                self.storm_remaining = 0.0;
                tick.storm_ended = true;
            }
        }
        match self.active {
            Some(elapsed) => {
                let e = elapsed + dt;
                if e >= CROSSING_TIME {
                    self.active = None;
                    self.next_in = interval_from_roll(roll) * self.interval_factor;
                    tick.missed = true;
                } else {
                    self.active = Some(e);
                }
            }
            None => {
                self.next_in -= dt;
                if self.next_in <= 0.0 {
                    self.active = Some(0.0);
                    tick.spawned = true;
                }
            }
        }
        tick
    }

    /// Termine le passage de la comète (capturée) et planifie la suivante.
    pub fn take(&mut self, roll_next: f64) -> bool {
        if self.active.take().is_some() {
            self.next_in = interval_from_roll(roll_next) * self.interval_factor;
            true
        } else {
            false
        }
    }

    pub fn start_storm(&mut self, duration: f64) {
        self.storm_remaining = self.storm_remaining.max(0.0) + duration;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_comet_after_45s_then_crosses_in_15s() {
        let mut c = CometState::default();
        assert!(!c.advance(44.0, 0.5).spawned);
        assert!(c.advance(1.5, 0.5).spawned);
        assert_eq!(c.progress(), Some(0.0));
        assert!(!c.advance(14.0, 0.5).missed);
        let t = c.advance(1.0, 0.0);
        assert!(t.missed);
        assert_eq!(c.active, None);
        assert_eq!(c.next_in, MIN_INTERVAL);
    }

    #[test]
    fn interval_bounds() {
        assert_eq!(interval_from_roll(0.0), 90.0);
        assert_eq!(interval_from_roll(1.0), 240.0);
        assert_eq!(interval_from_roll(7.0), 240.0);
    }

    #[test]
    fn rewards_50_50() {
        assert!(matches!(
            CometReward::from_roll(0.1, 0.0, 0.0),
            CometReward::SolarStorm { .. }
        ));
        match CometReward::from_roll(0.9, 1000.0, 1.0) {
            CometReward::StarShower { amount } => assert_eq!(amount, 150.0 + 13.0),
            other => panic!("{other:?}"),
        }
        assert_eq!(star_shower_amount(100.0, 1e9), 15.0 + 13.0);
    }

    #[test]
    fn storm_counts_down() {
        let mut c = CometState::default();
        c.start_storm(30.0);
        assert_eq!(c.boost(), 7.0);
        assert!(!c.advance(29.0, 0.0).storm_ended);
        assert!(c.advance(2.0, 0.0).storm_ended);
        assert_eq!(c.boost(), 1.0);
    }

    #[test]
    fn take_only_when_active() {
        let mut c = CometState::default();
        assert!(!c.take(0.5));
        c.advance(50.0, 0.0);
        assert!(c.take(1.0));
        assert_eq!(c.next_in, 240.0);
    }
}
