//! `GameState` : l'agrégat racine et toutes les règles qui le modifient.

use crate::achievement::{AchievementContext, AchievementId};
use crate::catalog::balance::{
    HOLD_HARVEST_INTERVAL, OFFLINE_CAP_SECS, OFFLINE_RATE, REVEAL_RATIO,
};
use crate::catalog::Catalog;
use crate::comet::{CometReward, CometState};
use crate::economy;
use crate::events::{DomainEvent, PurchaseTarget};
use crate::generator::{milestones_crossed, BuyMode, GeneratorId};
use crate::prestige::fragments_available;
use crate::stardust::Stardust;
use crate::upgrade::UpgradeId;

/// Intervalle entre deux vérifications de succès.
const ACHIEVEMENT_CHECK_INTERVAL: f64 = 1.0;

#[derive(Clone, Debug, PartialEq)]
pub struct GameState {
    pub stardust: Stardust,
    /// Gagné depuis la dernière Supernova.
    pub total_run: f64,
    /// Gagné depuis toujours.
    pub total_all: f64,
    /// Exemplaires possédés, indexés par générateur.
    pub owned: Vec<u64>,
    /// Améliorations achetées, indexées par amélioration.
    pub upgrades: Vec<bool>,
    /// Succès débloqués, indexés par succès.
    pub achievements: Vec<bool>,
    pub fragments: f64,
    pub fragments_this_run: f64,
    pub supernovas: u32,
    pub clicks: u64,
    pub meteors_caught: u32,
    pub play_time: f64,
    pub comet: CometState,
    revealed: Vec<bool>,
    achievement_timer: f64,
    /// `play_time` de la dernière récolte « touche maintenue » acceptée.
    last_hold_harvest: f64,
}

impl GameState {
    pub fn new(catalog: &Catalog) -> Self {
        let mut s = GameState {
            stardust: Stardust::ZERO,
            total_run: 0.0,
            total_all: 0.0,
            owned: Vec::new(),
            upgrades: Vec::new(),
            achievements: Vec::new(),
            fragments: 0.0,
            fragments_this_run: 0.0,
            supernovas: 0,
            clicks: 0,
            meteors_caught: 0,
            play_time: 0.0,
            comet: CometState::default(),
            revealed: Vec::new(),
            achievement_timer: 0.0,
            last_hold_harvest: f64::NEG_INFINITY,
        };
        s.normalize(catalog);
        s
    }

    /// Remet l'état en cohérence avec le catalogue (tailles des vecteurs, valeurs
    /// invalides) et recalcule silencieusement les générateurs déjà visibles.
    /// À appeler après une reconstruction depuis une sauvegarde.
    pub fn normalize(&mut self, catalog: &Catalog) {
        self.owned.resize(catalog.generators.len(), 0);
        self.upgrades.resize(catalog.upgrades.len(), false);
        self.achievements.resize(catalog.achievements.len(), false);
        for v in [
            &mut self.total_run,
            &mut self.total_all,
            &mut self.fragments,
            &mut self.fragments_this_run,
            &mut self.play_time,
        ] {
            if !v.is_finite() || *v < 0.0 {
                *v = 0.0;
            }
        }
        self.stardust = Stardust::new(self.stardust.value());
        self.revealed = catalog
            .generators
            .iter()
            .map(|g| self.is_visible(catalog, g.id))
            .collect();
    }

    pub fn owned_of(&self, id: GeneratorId) -> u64 {
        self.owned.get(id.index()).copied().unwrap_or(0)
    }

    pub fn has_upgrade(&self, id: UpgradeId) -> bool {
        self.upgrades.get(id.index()).copied().unwrap_or(false)
    }

    pub fn has_achievement(&self, id: AchievementId) -> bool {
        self.achievements.get(id.index()).copied().unwrap_or(false)
    }

    pub fn achievements_unlocked(&self) -> usize {
        self.achievements.iter().filter(|&&a| a).count()
    }

    /// Visible si possédé, si le précédent est possédé, ou si `total_run ≥ coût × 0.5`.
    /// Le premier générateur est toujours visible.
    pub fn is_visible(&self, catalog: &Catalog, id: GeneratorId) -> bool {
        let Some(def) = catalog.generator(id) else {
            return false;
        };
        let i = id.index();
        i == 0
            || self.owned_of(id) > 0
            || (i > 0 && self.owned_of(GeneratorId(i - 1)) > 0)
            || self.total_run >= def.base_cost * REVEAL_RATIO
    }

    /// Une amélioration est proposée si elle est débloquée et pas encore achetée.
    pub fn is_upgrade_available(&self, catalog: &Catalog, id: UpgradeId) -> bool {
        match catalog.upgrade(id) {
            Some(def) => {
                !self.has_upgrade(id) && def.requirement.is_met(&self.owned, self.total_run)
            }
            None => false,
        }
    }

    pub fn fragments_available(&self) -> f64 {
        fragments_available(self.total_run, self.fragments_this_run)
    }

    fn earn(&mut self, amount: f64) {
        if !amount.is_finite() || amount <= 0.0 {
            return;
        }
        self.stardust += amount;
        self.total_run += amount;
        self.total_all += amount;
    }

    /// Récolte manuelle.
    pub fn harvest(&mut self, catalog: &Catalog, events: &mut Vec<DomainEvent>) {
        let amount = economy::click_value(catalog, self);
        self.earn(amount);
        self.clicks += 1;
        events.push(DomainEvent::Harvested { amount });
        self.check_reveals(catalog, events);
    }

    /// Récolte déclenchée en maintenant la touche : nécessite l'amélioration
    /// « Récolte continue », et la cadence est plafonnée à une récolte tous les
    /// `HOLD_HARVEST_INTERVAL` secondes de jeu (indépendamment de la vitesse de
    /// répétition du clavier). Les répétitions trop rapprochées sont ignorées.
    pub fn harvest_held(&mut self, catalog: &Catalog, events: &mut Vec<DomainEvent>) {
        if !economy::hold_harvest_unlocked(catalog, self) {
            events.push(DomainEvent::HoldHarvestLocked);
            return;
        }
        if self.play_time - self.last_hold_harvest < HOLD_HARVEST_INTERVAL - 1e-9 {
            return;
        }
        self.last_hold_harvest = self.play_time;
        self.harvest(catalog, events);
    }

    /// Achat de générateurs selon le mode d'achat.
    pub fn buy_generator(
        &mut self,
        catalog: &Catalog,
        id: GeneratorId,
        mode: BuyMode,
        events: &mut Vec<DomainEvent>,
    ) {
        let fail = DomainEvent::PurchaseFailed {
            target: PurchaseTarget::Generator(id),
        };
        let Some(def) = catalog.generator(id) else {
            events.push(fail);
            return;
        };
        if !self.is_visible(catalog, id) {
            events.push(fail);
            return;
        }
        let owned = self.owned_of(id);
        let count = match mode {
            BuyMode::Max => def.max_affordable(owned, self.stardust.value()),
            other => other.quantity(def, owned, self.stardust.value()),
        };
        let cost = def.cost_for(owned, count);
        if count == 0 || !self.stardust.spend(cost) {
            events.push(fail);
            return;
        }
        let after = owned + count;
        self.owned[id.index()] = after;
        events.push(DomainEvent::Purchased {
            generator: id,
            count,
            cost,
        });
        for m in milestones_crossed(owned, after) {
            events.push(DomainEvent::MilestoneReached {
                generator: id,
                count: m,
            });
        }
        self.check_reveals(catalog, events);
    }

    /// Achat d'une amélioration.
    pub fn buy_upgrade(&mut self, catalog: &Catalog, id: UpgradeId, events: &mut Vec<DomainEvent>) {
        let fail = DomainEvent::PurchaseFailed {
            target: PurchaseTarget::Upgrade(id),
        };
        let Some(def) = catalog.upgrade(id) else {
            events.push(fail);
            return;
        };
        if !self.is_upgrade_available(catalog, id) || !self.stardust.spend(def.cost) {
            events.push(fail);
            return;
        }
        self.upgrades[id.index()] = true;
        events.push(DomainEvent::UpgradePurchased {
            upgrade: id,
            cost: def.cost,
        });
    }

    /// Fait avancer le jeu de `dt` secondes. `roll` est un tirage uniforme dans
    /// [0, 1) fourni par l'appelant (utilisé pour planifier les comètes).
    pub fn tick(&mut self, catalog: &Catalog, dt: f64, roll: f64, events: &mut Vec<DomainEvent>) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.play_time += dt;
        let production = economy::total_production(catalog, self);
        self.earn(production * dt);

        let comet = self.comet.advance(dt, roll);
        if comet.spawned {
            events.push(DomainEvent::CometSpawned);
        }
        if comet.missed {
            events.push(DomainEvent::CometMissed);
        }
        if comet.storm_ended {
            events.push(DomainEvent::SolarStormEnded);
        }

        self.check_reveals(catalog, events);

        self.achievement_timer += dt;
        if self.achievement_timer >= ACHIEVEMENT_CHECK_INTERVAL {
            self.achievement_timer = 0.0;
            self.check_achievements(catalog, events);
        }
    }

    /// Capture la comète en cours. `roll_reward` choisit la récompense,
    /// `roll_next` l'intervalle avant la suivante. Sans comète, ne fait rien.
    pub fn catch_comet(
        &mut self,
        catalog: &Catalog,
        roll_reward: f64,
        roll_next: f64,
        events: &mut Vec<DomainEvent>,
    ) {
        if !self.comet.take(roll_next) {
            return;
        }
        let production = economy::total_production(catalog, self);
        let reward = CometReward::from_roll(roll_reward, self.stardust.value(), production);
        match reward {
            CometReward::SolarStorm { duration, .. } => self.comet.start_storm(duration),
            CometReward::StarShower { amount } => self.earn(amount),
        }
        self.meteors_caught += 1;
        events.push(DomainEvent::CometCaught { reward });
        self.check_reveals(catalog, events);
    }

    /// Déclenche une Supernova si au moins un fragment est gagnable.
    pub fn supernova(&mut self, catalog: &Catalog, events: &mut Vec<DomainEvent>) {
        let gained = self.fragments_available();
        if gained < 1.0 {
            events.push(DomainEvent::SupernovaUnavailable);
            return;
        }
        self.fragments += gained;
        self.fragments_this_run = 0.0;
        self.supernovas += 1;
        self.stardust = Stardust::ZERO;
        self.total_run = 0.0;
        self.owned.iter_mut().for_each(|o| *o = 0);
        self.upgrades.iter_mut().for_each(|u| *u = false);
        self.comet.storm_remaining = 0.0;
        self.revealed = catalog
            .generators
            .iter()
            .map(|g| self.is_visible(catalog, g.id))
            .collect();
        events.push(DomainEvent::Supernova {
            fragments_gained: gained,
            total_fragments: self.fragments,
        });
        self.check_achievements(catalog, events);
    }

    /// Gains hors-ligne : `prod × min(écoulé, 12 h) × 0.5`.
    pub fn apply_offline(
        &mut self,
        catalog: &Catalog,
        elapsed_secs: f64,
        events: &mut Vec<DomainEvent>,
    ) {
        if !elapsed_secs.is_finite() || elapsed_secs <= 0.0 {
            return;
        }
        let seconds = elapsed_secs.min(OFFLINE_CAP_SECS);
        let amount = economy::steady_production(catalog, self) * seconds * OFFLINE_RATE;
        if amount <= 0.0 {
            return;
        }
        self.earn(amount);
        events.push(DomainEvent::OfflineGains {
            seconds: elapsed_secs,
            amount,
        });
        // Les générateurs dévoilés pendant l'absence apparaissent sans annonce.
        self.revealed = catalog
            .generators
            .iter()
            .map(|g| self.is_visible(catalog, g.id))
            .collect();
    }

    /// Vérifie les succès et émet un événement par succès nouvellement débloqué.
    pub fn check_achievements(&mut self, catalog: &Catalog, events: &mut Vec<DomainEvent>) {
        let production = economy::total_production(catalog, self);
        let mut unlocked = Vec::new();
        {
            let ctx = AchievementContext {
                owned: &self.owned,
                total_all: self.total_all,
                clicks: self.clicks,
                comets: self.meteors_caught,
                supernovas: self.supernovas,
                production,
            };
            for a in &catalog.achievements {
                if !self.has_achievement(a.id) && a.condition.is_met(&ctx) {
                    unlocked.push(a.id);
                }
            }
        }
        for id in unlocked {
            self.achievements[id.index()] = true;
            events.push(DomainEvent::AchievementUnlocked { achievement: id });
        }
    }

    fn check_reveals(&mut self, catalog: &Catalog, events: &mut Vec<DomainEvent>) {
        for g in &catalog.generators {
            let i = g.id.index();
            if i < self.revealed.len() && !self.revealed[i] && self.is_visible(catalog, g.id) {
                self.revealed[i] = true;
                events.push(DomainEvent::GeneratorRevealed { generator: g.id });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Catalog, GameState, Vec<DomainEvent>) {
        let c = Catalog::standard();
        let s = GameState::new(&c);
        (c, s, Vec::new())
    }

    #[test]
    fn first_generator_in_15_clicks() {
        let (c, mut s, mut ev) = setup();
        for _ in 0..15 {
            s.harvest(&c, &mut ev);
        }
        assert_eq!(s.clicks, 15);
        assert_eq!(s.stardust.value(), 15.0);
        s.buy_generator(&c, GeneratorId(0), BuyMode::One, &mut ev);
        assert_eq!(s.owned[0], 1);
        assert!(ev
            .iter()
            .any(|e| matches!(e, DomainEvent::Purchased { count: 1, .. })));
        // Le drone devient visible car la sonde précédente est possédée.
        assert!(ev.contains(&DomainEvent::GeneratorRevealed {
            generator: GeneratorId(1)
        }));
    }

    #[test]
    fn hold_harvest_requires_upgrade_and_is_rate_limited() {
        let (c, mut s, mut ev) = setup();
        s.harvest_held(&c, &mut ev);
        assert_eq!(ev, [DomainEvent::HoldHarvestLocked]);
        assert_eq!(s.clicks, 0);
        ev.clear();

        let hold = c.upgrade_by_key("hold_0").unwrap();
        s.total_run = 150.0;
        s.stardust = Stardust::new(300.0);
        s.buy_upgrade(&c, hold, &mut ev);
        assert!(s.has_upgrade(hold));
        ev.clear();

        // Trois répétitions dans la même frame : une seule récolte.
        for _ in 0..3 {
            s.harvest_held(&c, &mut ev);
        }
        assert_eq!(s.clicks, 1);
        // 0.1 s plus tard : une de plus. 10 s → 100 récoltes au maximum.
        s.tick(&c, 0.1, 0.5, &mut ev);
        s.harvest_held(&c, &mut ev);
        assert_eq!(s.clicks, 2);
        for _ in 0..100 {
            s.tick(&c, 0.05, 0.5, &mut ev);
            s.harvest_held(&c, &mut ev);
            s.harvest_held(&c, &mut ev);
        }
        assert!((51..=52).contains(&s.clicks), "{}", s.clicks);
        // Un clic normal n'est jamais limité.
        let before = s.clicks;
        s.harvest(&c, &mut ev);
        s.harvest(&c, &mut ev);
        assert_eq!(s.clicks, before + 2);
    }

    #[test]
    fn purchase_fails_without_funds() {
        let (c, mut s, mut ev) = setup();
        s.buy_generator(&c, GeneratorId(0), BuyMode::One, &mut ev);
        assert_eq!(
            ev,
            [DomainEvent::PurchaseFailed {
                target: PurchaseTarget::Generator(GeneratorId(0))
            }]
        );
        assert_eq!(s.owned[0], 0);
    }

    #[test]
    fn hidden_generator_cannot_be_bought() {
        let (c, mut s, mut ev) = setup();
        s.stardust = Stardust::new(1e12);
        s.buy_generator(&c, GeneratorId(8), BuyMode::One, &mut ev);
        assert!(matches!(ev[0], DomainEvent::PurchaseFailed { .. }));
    }

    #[test]
    fn buy_max_and_milestones() {
        let (c, mut s, mut ev) = setup();
        let cost = c.generators[0].cost_for(0, 60);
        s.stardust = Stardust::new(cost * (1.0 + 1e-9));
        s.buy_generator(&c, GeneratorId(0), BuyMode::Max, &mut ev);
        assert_eq!(s.owned[0], 60);
        let milestones: Vec<_> = ev
            .iter()
            .filter_map(|e| match e {
                DomainEvent::MilestoneReached { count, .. } => Some(*count),
                _ => None,
            })
            .collect();
        assert_eq!(milestones, [25, 50]);
    }

    #[test]
    fn upgrade_purchase_and_availability() {
        let (c, mut s, mut ev) = setup();
        let id = c.upgrade_by_key("gen0_t0").unwrap();
        s.stardust = Stardust::new(1e6);
        s.buy_upgrade(&c, id, &mut ev);
        assert!(
            matches!(ev[0], DomainEvent::PurchaseFailed { .. }),
            "verrouillé"
        );
        s.owned[0] = 10;
        assert!(s.is_upgrade_available(&c, id));
        s.buy_upgrade(&c, id, &mut ev);
        assert!(s.has_upgrade(id));
        assert!(!s.is_upgrade_available(&c, id));
    }

    #[test]
    fn tick_produces_and_spawns_comet() {
        let (c, mut s, mut ev) = setup();
        s.owned[1] = 10;
        s.tick(&c, 2.0, 0.5, &mut ev);
        assert!((s.stardust.value() - 20.0).abs() < 1e-9);
        assert_eq!(s.play_time, 2.0);
        s.tick(&c, 50.0, 0.5, &mut ev);
        assert!(ev.contains(&DomainEvent::CometSpawned));
        let before = s.stardust.value();
        s.catch_comet(&c, 0.9, 0.5, &mut ev);
        assert_eq!(s.meteors_caught, 1);
        assert!(s.stardust.value() > before);
        assert!(ev
            .iter()
            .any(|e| matches!(e, DomainEvent::CometCaught { .. })));
    }

    #[test]
    fn catch_without_comet_does_nothing() {
        let (c, mut s, mut ev) = setup();
        s.catch_comet(&c, 0.1, 0.1, &mut ev);
        assert!(ev.is_empty());
        assert_eq!(s.meteors_caught, 0);
    }

    #[test]
    fn supernova_resets_and_keeps() {
        let (c, mut s, mut ev) = setup();
        s.supernova(&c, &mut ev);
        assert_eq!(ev, [DomainEvent::SupernovaUnavailable]);
        ev.clear();

        s.total_run = 9e8;
        s.total_all = 1e10;
        s.stardust = Stardust::new(5e8);
        s.owned[2] = 40;
        s.upgrades[0] = true;
        s.achievements[0] = true;
        s.clicks = 77;
        s.play_time = 1234.0;
        s.supernova(&c, &mut ev);
        assert_eq!(s.fragments, 3.0);
        assert_eq!(s.supernovas, 1);
        assert_eq!(s.stardust.value(), 0.0);
        assert_eq!(s.total_run, 0.0);
        assert!(s.owned.iter().all(|&o| o == 0));
        assert!(s.upgrades.iter().all(|&u| !u));
        assert_eq!(s.total_all, 1e10);
        assert_eq!(s.clicks, 77);
        assert_eq!(s.play_time, 1234.0);
        assert!(s.achievements[0]);
        assert!(ev.iter().any(|e| matches!(
            e,
            DomainEvent::Supernova { fragments_gained, total_fragments }
                if *fragments_gained == 3.0 && *total_fragments == 3.0
        )));
        // Le succès « Supernova ! » est débloqué immédiatement.
        let nova = c.achievement_by_key("nova1").unwrap();
        assert!(s.has_achievement(nova));
    }

    #[test]
    fn offline_gains_capped_and_halved() {
        let (c, mut s, mut ev) = setup();
        s.owned[1] = 1; // 1 /s
        s.apply_offline(&c, 100.0, &mut ev);
        assert!((s.stardust.value() - 50.0).abs() < 1e-9);
        ev.clear();
        s.apply_offline(&c, 100.0 * 3600.0, &mut ev);
        assert!((s.stardust.value() - 50.0 - 12.0 * 3600.0 * 0.5).abs() < 1e-6);
        assert!(matches!(ev[0], DomainEvent::OfflineGains { .. }));
    }

    #[test]
    fn achievements_checked_every_second() {
        let (c, mut s, mut ev) = setup();
        s.clicks = 100;
        s.tick(&c, 0.5, 0.0, &mut ev);
        assert!(ev.is_empty());
        s.tick(&c, 0.6, 0.0, &mut ev);
        let id = c.achievement_by_key("clicks100").unwrap();
        assert!(ev.contains(&DomainEvent::AchievementUnlocked { achievement: id }));
        ev.clear();
        s.tick(&c, 1.0, 0.0, &mut ev);
        assert!(ev.is_empty(), "pas de double annonce");
    }

    #[test]
    fn three_generators_revealed_quickly() {
        // Simulation simple : clic 5×/s + achat glouton du générateur le moins cher.
        let (c, mut s, mut ev) = setup();
        let dt = 0.2;
        let mut t = 0.0;
        while t < 300.0 {
            s.harvest(&c, &mut ev);
            for i in (0..3).rev() {
                s.buy_generator(&c, GeneratorId(i), BuyMode::One, &mut ev);
            }
            s.tick(&c, dt, 0.5, &mut ev);
            t += dt;
        }
        assert!(s.is_visible(&c, GeneratorId(2)), "3 générateurs en < 5 min");
    }

    #[test]
    fn normalize_fixes_sizes() {
        let c = Catalog::standard();
        let mut s = GameState::new(&c);
        s.owned = vec![3];
        s.total_run = f64::NAN;
        s.normalize(&c);
        assert_eq!(s.owned.len(), 9);
        assert_eq!(s.total_run, 0.0);
    }
}
