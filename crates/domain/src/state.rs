//! `GameState` : l'agrégat racine et toutes les règles qui le modifient.

use crate::achievement::{AchievementContext, AchievementId};
use crate::catalog::balance::{
    HOLD_HARVEST_INTERVAL, OFFLINE_CAP_SECS, OFFLINE_RATE, REVEAL_RATIO,
};
use crate::catalog::Catalog;
use crate::challenge::{ChallengeId, ChallengeReward, Restriction};
use crate::comet::{
    CometKind, CometReward, CometState, BLACK_THEFT, GOLDEN_DURATION, GOLDEN_MULTIPLIER,
};
use crate::constellation::{ConstellationEffect, ConstellationId};
use crate::economy;
use crate::events::{DomainEvent, PurchaseTarget};
use crate::generator::{milestones_crossed, BuyMode, GeneratorId};
use crate::perk::{PerkEffect, PerkId};
use crate::prestige::{fragment_divisor, fragments_for};
use crate::stardust::Stardust;
use crate::upgrade::{Effect, UpgradeId};

/// Intervalle entre deux vérifications de succès.
const ACHIEVEMENT_CHECK_INTERVAL: f64 = 1.0;
/// Intervalle entre deux passages de l'automate d'achat.
const AUTOBUY_INTERVAL: f64 = 1.0;
/// Nombre maximal de générateurs achetés par passage de l'automate.
const AUTOBUY_MAX_PER_TICK: usize = 10;

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
    /// Améliorations de l'Atelier achetées (permanentes).
    pub perks: Vec<bool>,
    /// Fragments dépensés à l'Atelier (le bonus de production reste calculé
    /// sur le total `fragments`).
    pub fragments_spent: f64,
    /// Automate d'achat activé (si acheté).
    pub autobuy: bool,
    /// Défi en cours et `play_time` de son lancement.
    pub challenge: Option<ChallengeId>,
    pub challenge_started: f64,
    /// Défis réussis (récompenses permanentes).
    pub challenges_done: Vec<bool>,
    pub golden_caught: u32,
    pub black_caught: u32,
    /// Niveau de chaque Constellation.
    pub constellations: Vec<u32>,
    revealed: Vec<bool>,
    autobuy_timer: f64,
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
            perks: Vec::new(),
            fragments_spent: 0.0,
            autobuy: false,
            challenge: None,
            challenge_started: 0.0,
            challenges_done: Vec::new(),
            golden_caught: 0,
            black_caught: 0,
            constellations: Vec::new(),
            revealed: Vec::new(),
            autobuy_timer: 0.0,
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
        self.perks.resize(catalog.perks.len(), false);
        self.constellations.resize(catalog.constellations.len(), 0);
        self.challenges_done.resize(catalog.challenges.len(), false);
        if self
            .challenge
            .is_some_and(|c| c.index() >= catalog.challenges.len())
        {
            self.challenge = None;
        }
        for (lvl, def) in self.constellations.iter_mut().zip(&catalog.constellations) {
            if let Some(max) = def.max_level {
                *lvl = (*lvl).min(max);
            }
        }
        for v in [
            &mut self.fragments_spent,
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
        self.fragments_spent = self.fragments_spent.min(self.fragments);
        self.autobuy &= self.has_perk_effect(catalog, |e| matches!(e, PerkEffect::AutoBuyer));
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

    pub fn has_perk(&self, id: PerkId) -> bool {
        self.perks.get(id.index()).copied().unwrap_or(false)
    }

    pub fn perks_owned(&self) -> usize {
        self.perks.iter().filter(|&&p| p).count()
    }

    /// Effets des améliorations de l'Atelier possédées.
    pub fn perk_effects<'a>(
        &'a self,
        catalog: &'a Catalog,
    ) -> impl Iterator<Item = PerkEffect> + 'a {
        catalog
            .perks
            .iter()
            .filter(|p| self.has_perk(p.id))
            .map(|p| p.effect)
    }

    fn has_perk_effect(&self, catalog: &Catalog, f: impl Fn(&PerkEffect) -> bool) -> bool {
        self.perk_effects(catalog).any(|e| f(&e))
    }

    /// Fragments disponibles pour l'Atelier.
    pub fn fragments_spendable(&self) -> f64 {
        (self.fragments - self.fragments_spent).max(0.0)
    }

    /// Multiplicateur appliqué au coût des générateurs (Atelier : « Ingénierie frugale »).
    pub fn generator_cost_factor(&self, catalog: &Catalog) -> f64 {
        let perks: f64 = self
            .perk_effects(catalog)
            .filter_map(|e| match e {
                PerkEffect::GeneratorDiscount(f) => Some(f),
                _ => None,
            })
            .product();
        let constellations: f64 = catalog
            .constellations
            .iter()
            .filter_map(|c| match c.effect {
                ConstellationEffect::Discount(f) => {
                    Some(f.powi(self.constellation_level(c.id) as i32))
                }
                _ => None,
            })
            .product();
        perks * constellations
    }

    fn comet_interval_factor(&self, catalog: &Catalog) -> f64 {
        let rewards: f64 = self
            .challenge_rewards(catalog)
            .filter_map(|r| match r {
                ChallengeReward::CometFrequency(f) => Some(f),
                _ => None,
            })
            .product();
        rewards
            * self
                .perk_effects(catalog)
                .filter_map(|e| match e {
                    PerkEffect::CometMagnet(f) => Some(f),
                    _ => None,
                })
                .product::<f64>()
    }

    fn storm_factor(&self, catalog: &Catalog) -> f64 {
        self.perk_effects(catalog)
            .filter_map(|e| match e {
                PerkEffect::LongStorm(f) => Some(f),
                _ => None,
            })
            .product()
    }

    fn nova_factor(&self, catalog: &Catalog) -> f64 {
        let perks: f64 = self
            .perk_effects(catalog)
            .filter_map(|e| match e {
                PerkEffect::RichNova(f) => Some(f),
                _ => None,
            })
            .product();
        perks
            * (1.0
                + self.constellation_sum(catalog, |e| match e {
                    ConstellationEffect::Fragments(x) => Some(x),
                    _ => None,
                }))
    }

    pub fn constellation_level(&self, id: ConstellationId) -> u32 {
        self.constellations.get(id.index()).copied().unwrap_or(0)
    }

    pub fn constellation_levels_total(&self) -> u32 {
        self.constellations.iter().sum()
    }

    /// Somme « valeur par niveau × niveau » des Constellations d'un type.
    fn constellation_sum(
        &self,
        catalog: &Catalog,
        f: impl Fn(ConstellationEffect) -> Option<f64>,
    ) -> f64 {
        catalog
            .constellations
            .iter()
            .filter_map(|c| f(c.effect).map(|x| x * self.constellation_level(c.id) as f64))
            .sum()
    }

    /// Multiplicateur de production des Constellations.
    pub fn constellation_production(&self, catalog: &Catalog) -> f64 {
        1.0 + self.constellation_sum(catalog, |e| match e {
            ConstellationEffect::Production(x) => Some(x),
            _ => None,
        })
    }

    /// Fraction de la prod/s ajoutée aux clics par les Constellations.
    pub fn constellation_click(&self, catalog: &Catalog) -> f64 {
        self.constellation_sum(catalog, |e| match e {
            ConstellationEffect::Click(x) => Some(x),
            _ => None,
        })
    }

    fn comet_levels(&self, catalog: &Catalog) -> (f64, f64) {
        catalog
            .constellations
            .iter()
            .filter_map(|c| match c.effect {
                ConstellationEffect::Comets(x) => Some((self.constellation_level(c.id) as f64, x)),
                _ => None,
            })
            .fold((0.0, 0.0), |(l, s), (lvl, x)| (l + lvl, s + lvl * x))
    }

    /// Achète un niveau de Constellation avec des fragments.
    pub fn buy_constellation(
        &mut self,
        catalog: &Catalog,
        id: ConstellationId,
        events: &mut Vec<DomainEvent>,
    ) {
        let fail = DomainEvent::PurchaseFailed {
            target: PurchaseTarget::Constellation(id),
        };
        let Some(def) = catalog.constellation(id) else {
            events.push(fail);
            return;
        };
        let level = self.constellation_level(id);
        let cost = def.cost_at(level);
        if def.is_maxed(level) || self.fragments_spendable() + 1e-9 < cost {
            events.push(fail);
            return;
        }
        self.fragments_spent += cost;
        self.constellations[id.index()] = level + 1;
        events.push(DomainEvent::ConstellationUpgraded {
            constellation: id,
            level: level + 1,
        });
        self.check_achievements(catalog, events);
    }

    /// (rendement, plafond en secondes) de la production hors-ligne.
    pub fn offline_terms(&self, catalog: &Catalog) -> (f64, f64) {
        let (rate, cap) = self
            .perk_effects(catalog)
            .find_map(|e| match e {
                PerkEffect::OfflineMastery { rate, cap_secs } => Some((rate, cap_secs)),
                _ => None,
            })
            .unwrap_or((OFFLINE_RATE, OFFLINE_CAP_SECS));
        let extra_hours = self.constellation_sum(catalog, |e| match e {
            ConstellationEffect::OfflineHours(h) => Some(h),
            _ => None,
        });
        (rate, cap + extra_hours * 3600.0)
    }

    /// Vrai si l'automate d'achat a été acheté.
    pub fn autobuy_unlocked(&self, catalog: &Catalog) -> bool {
        self.has_perk_effect(catalog, |e| matches!(e, PerkEffect::AutoBuyer))
    }

    /// Coût réel de `n` exemplaires (remises comprises).
    pub fn generator_cost(&self, catalog: &Catalog, id: GeneratorId, n: u64) -> f64 {
        catalog
            .generator(id)
            .map(|d| d.cost_for(self.owned_of(id), n) * self.generator_cost_factor(catalog))
            .unwrap_or(f64::INFINITY)
    }

    /// Quantité achetée selon le mode, remises comprises (au moins 1 en MAX).
    pub fn purchase_quantity(&self, catalog: &Catalog, id: GeneratorId, mode: BuyMode) -> u64 {
        let Some(def) = catalog.generator(id) else {
            return 0;
        };
        let budget = self.stardust.value() / self.generator_cost_factor(catalog);
        mode.quantity(def, self.owned_of(id), budget)
    }

    /// Prochaine amélioration encore verrouillée (la moins chère).
    pub fn next_locked_upgrade(&self, catalog: &Catalog) -> Option<UpgradeId> {
        catalog
            .upgrades
            .iter()
            .filter(|u| {
                !self.has_upgrade(u.id) && !u.requirement.is_met(&self.owned, self.total_run)
            })
            .min_by(|a, b| {
                a.cost
                    .partial_cmp(&b.cost)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|u| u.id)
    }

    /// Visible si possédé, si le précédent est possédé, ou si `total_run ≥ coût × 0.5`.
    /// Le premier générateur est toujours visible.
    pub fn is_visible(&self, catalog: &Catalog, id: GeneratorId) -> bool {
        let Some(def) = catalog.generator(id) else {
            return false;
        };
        if self.supernovas < def.unlock_supernovas {
            return false;
        }
        if let Some(Restriction::OnlyFirstGenerators(n)) = self.active_restriction(catalog) {
            if id.index() >= n {
                return false;
            }
        }
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

    /// Fragments gagnables maintenant (Atelier « Nova dorée » compris).
    pub fn fragments_available_with(&self, catalog: &Catalog) -> f64 {
        let mut base =
            (fragments_for(self.total_run, self.supernovas) * self.nova_factor(catalog)).floor();
        if base >= 1.0 {
            // Défi « Course contre la montre » : fragments en plus.
            base += self
                .challenge_rewards(catalog)
                .filter_map(|r| match r {
                    ChallengeReward::BonusFragments(n) => Some(n),
                    _ => None,
                })
                .sum::<f64>();
        }
        (base - self.fragments_this_run).max(0.0)
    }

    /// `total_run` nécessaire pour gagner un fragment de plus que maintenant.
    pub fn total_run_for_next_fragment(&self, catalog: &Catalog) -> f64 {
        let factor = self.nova_factor(catalog).max(1e-9);
        let next = self.fragments_available_with(catalog) + self.fragments_this_run + 1.0;
        // floor(sqrt(t / D) × facteur) ≥ next  ⇔  t ≥ (next / facteur)² × D
        (next / factor).powi(2) * fragment_divisor(self.supernovas)
    }

    /// Fragments gagnables sans tenir compte de l'Atelier.
    pub fn fragments_available(&self) -> f64 {
        (fragments_for(self.total_run, self.supernovas) - self.fragments_this_run).max(0.0)
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
        if self.active_restriction(catalog) == Some(Restriction::NoClick) {
            events.push(DomainEvent::ChallengeBlocked {
                restriction: Restriction::NoClick,
            });
            return;
        }
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
        if self.active_restriction(catalog) == Some(Restriction::NoClick) {
            return;
        }
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
        let factor = self.generator_cost_factor(catalog);
        let budget = self.stardust.value() / factor;
        let count = match mode {
            BuyMode::Max => def.max_affordable(owned, budget),
            other => other.quantity(def, owned, budget),
        };
        let cost = def.cost_for(owned, count) * factor;
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
        if self.active_restriction(catalog) == Some(Restriction::NoUpgrades) {
            events.push(DomainEvent::ChallengeBlocked {
                restriction: Restriction::NoUpgrades,
            });
            return;
        }
        let cost = def.cost * self.upgrade_cost_factor(catalog);
        if !self.is_upgrade_available(catalog, id) || !self.stardust.spend(cost) {
            events.push(fail);
            return;
        }
        self.upgrades[id.index()] = true;
        events.push(DomainEvent::UpgradePurchased { upgrade: id, cost });
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

        self.comet.interval_factor = self.comet_interval_factor(catalog);
        if self.active_restriction(catalog) == Some(Restriction::NoComets) {
            // Nuit noire : la prochaine comète n'arrive jamais.
            self.comet.active = None;
            self.comet.next_in = self.comet.next_in.max(1.0) + dt;
        }
        let comet = self.comet.advance(dt, roll);
        if comet.spawned {
            events.push(DomainEvent::CometSpawned {
                kind: self.comet.kind,
            });
        }
        if comet.missed {
            events.push(DomainEvent::CometMissed);
            if self.comet.kind == CometKind::Black {
                let amount = self.stardust.value() * BLACK_THEFT;
                if amount > 0.0 && self.stardust.spend(amount) {
                    events.push(DomainEvent::CometStole { amount });
                }
            }
        }
        if comet.storm_ended {
            events.push(DomainEvent::SolarStormEnded);
        }

        self.check_reveals(catalog, events);
        self.check_challenge(catalog, events);

        if self.autobuy {
            self.autobuy_timer += dt;
            if self.autobuy_timer >= AUTOBUY_INTERVAL {
                self.autobuy_timer = 0.0;
                self.run_autobuyer(catalog, events);
            }
        }

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
        let (comet_levels, shower_bonus) = self.comet_levels(catalog);
        let base = match self.comet.kind {
            CometKind::Golden => CometReward::GoldenFrenzy {
                multiplier: GOLDEN_MULTIPLIER,
                duration: GOLDEN_DURATION,
            },
            CometKind::Black => CometReward::DarkShard { fragments: 1.0 },
            CometKind::Normal => {
                CometReward::from_roll(roll_reward, self.stardust.value(), production)
            }
        };
        let reward = match base {
            CometReward::SolarStorm {
                multiplier,
                duration,
            } => CometReward::SolarStorm {
                multiplier: multiplier + comet_levels,
                duration: duration * self.storm_factor(catalog),
            },
            CometReward::StarShower { amount } => CometReward::StarShower {
                amount: amount * (1.0 + shower_bonus),
            },
            other => other,
        };
        match reward {
            CometReward::SolarStorm {
                duration,
                multiplier,
            } => self.comet.start_storm_with(duration, multiplier),
            CometReward::StarShower { amount } => self.earn(amount),
            CometReward::GoldenFrenzy {
                multiplier,
                duration,
            } => {
                self.comet.start_storm_with(duration, multiplier);
                self.golden_caught += 1;
            }
            CometReward::DarkShard { fragments } => {
                self.fragments += fragments;
                self.black_caught += 1;
            }
        }
        self.meteors_caught += 1;
        events.push(DomainEvent::CometCaught { reward });
        self.check_reveals(catalog, events);
    }

    /// Déclenche une Supernova si au moins un fragment est gagnable.
    pub fn supernova(&mut self, catalog: &Catalog, events: &mut Vec<DomainEvent>) {
        let gained = self.fragments_available_with(catalog);
        if gained < 1.0 {
            events.push(DomainEvent::SupernovaUnavailable);
            return;
        }
        self.fragments += gained;
        self.fragments_this_run = 0.0;
        self.supernovas += 1;
        self.reset_run(catalog);
        events.push(DomainEvent::Supernova {
            fragments_gained: gained,
            total_fragments: self.fragments,
        });
        self.check_achievements(catalog, events);
    }

    /// Remet le cycle à zéro (Supernova ou lancement d'un Défi).
    fn reset_run(&mut self, catalog: &Catalog) {
        self.stardust = Stardust::ZERO;
        self.total_run = 0.0;
        self.owned.iter_mut().for_each(|o| *o = 0);
        // « Récolte continue » est un confort, pas une progression : on la garde.
        for u in &catalog.upgrades {
            if u.effect != Effect::HoldHarvest {
                self.upgrades[u.id.index()] = false;
            }
        }
        self.comet.storm_remaining = 0.0;
        // Atelier « Héritage » : le nouveau cycle démarre équipé.
        let kits: Vec<(u64, u64)> = self
            .perk_effects(catalog)
            .filter_map(|e| match e {
                PerkEffect::StartingKit { probes, drones } => Some((probes, drones)),
                _ => None,
            })
            .collect();
        for (probes, drones) in kits {
            if let Some(o) = self.owned.get_mut(0) {
                *o += probes;
            }
            if let Some(o) = self.owned.get_mut(1) {
                *o += drones;
            }
        }
        self.revealed = catalog
            .generators
            .iter()
            .map(|g| self.is_visible(catalog, g.id))
            .collect();
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
        let (rate, cap) = self.offline_terms(catalog);
        let seconds = elapsed_secs.min(cap);
        let amount = economy::steady_production(catalog, self) * seconds * rate;
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

    /// Achète toutes les améliorations abordables, de la moins chère à la plus chère.
    pub fn buy_all_upgrades(&mut self, catalog: &Catalog, events: &mut Vec<DomainEvent>) {
        if self.active_restriction(catalog) == Some(Restriction::NoUpgrades) {
            events.push(DomainEvent::ChallengeBlocked {
                restriction: Restriction::NoUpgrades,
            });
            return;
        }
        let factor = self.upgrade_cost_factor(catalog);
        let mut count = 0;
        loop {
            let next = catalog
                .upgrades
                .iter()
                .filter(|u| {
                    self.is_upgrade_available(catalog, u.id)
                        && self.stardust.can_afford(u.cost * factor)
                })
                .min_by(|a, b| {
                    a.cost
                        .partial_cmp(&b.cost)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|u| u.id);
            match next {
                Some(id) => {
                    self.buy_upgrade(catalog, id, events);
                    count += 1;
                }
                None => break,
            }
        }
        events.push(DomainEvent::BulkUpgrades { count });
    }

    /// Contrainte du Défi en cours.
    pub fn active_restriction(&self, catalog: &Catalog) -> Option<Restriction> {
        self.challenge
            .and_then(|id| catalog.challenge(id))
            .map(|c| c.restriction)
    }

    pub fn challenge_done(&self, id: ChallengeId) -> bool {
        self.challenges_done
            .get(id.index())
            .copied()
            .unwrap_or(false)
    }

    /// Récompenses des Défis réussis.
    pub fn challenge_rewards<'a>(
        &'a self,
        catalog: &'a Catalog,
    ) -> impl Iterator<Item = ChallengeReward> + 'a {
        catalog
            .challenges
            .iter()
            .filter(|c| self.challenge_done(c.id))
            .map(|c| c.reward)
    }

    /// Multiplicateur du coût des améliorations (Défi « Ascèse » réussi).
    pub fn upgrade_cost_factor(&self, catalog: &Catalog) -> f64 {
        self.challenge_rewards(catalog)
            .filter_map(|r| match r {
                ChallengeReward::UpgradeDiscount(f) => Some(f),
                _ => None,
            })
            .product()
    }

    /// Un Défi peut-il être lancé maintenant ?
    pub fn can_start_challenge(&self, catalog: &Catalog, id: ChallengeId) -> bool {
        catalog.challenge(id).is_some_and(|c| {
            self.challenge.is_none()
                && !self.challenge_done(id)
                && self.supernovas >= c.unlock_supernovas
        })
    }

    /// Lance un Défi : si une Supernova rapporte des fragments, elle a lieu
    /// d'abord ; sinon le cycle est simplement relancé.
    pub fn start_challenge(
        &mut self,
        catalog: &Catalog,
        id: ChallengeId,
        events: &mut Vec<DomainEvent>,
    ) {
        if !self.can_start_challenge(catalog, id) {
            events.push(DomainEvent::ChallengeUnavailable { challenge: id });
            return;
        }
        if self.fragments_available_with(catalog) >= 1.0 {
            self.supernova(catalog, events);
        } else {
            self.reset_run(catalog);
        }
        self.challenge = Some(id);
        self.challenge_started = self.play_time;
        // La contrainte s'applique tout de suite (générateurs cachés…).
        self.reset_reveals(catalog);
        events.push(DomainEvent::ChallengeStarted { challenge: id });
    }

    /// Abandonne le Défi en cours (le cycle continue normalement).
    pub fn abandon_challenge(&mut self, catalog: &Catalog, events: &mut Vec<DomainEvent>) {
        if let Some(id) = self.challenge.take() {
            self.reset_reveals(catalog);
            events.push(DomainEvent::ChallengeAbandoned { challenge: id });
        }
    }

    /// Avancement du Défi en cours : (objectif atteint ?, temps écoulé).
    fn check_challenge(&mut self, catalog: &Catalog, events: &mut Vec<DomainEvent>) {
        let Some(id) = self.challenge else { return };
        let Some(def) = catalog.challenge(id) else {
            self.challenge = None;
            return;
        };
        let elapsed = self.play_time - self.challenge_started;
        if self.total_run >= def.goal {
            self.challenge = None;
            self.challenges_done[id.index()] = true;
            self.reset_reveals(catalog);
            events.push(DomainEvent::ChallengeCompleted { challenge: id });
            self.check_achievements(catalog, events);
        } else if let Restriction::TimeLimit(limit) = def.restriction {
            if elapsed > limit {
                self.challenge = None;
                events.push(DomainEvent::ChallengeFailed { challenge: id });
            }
        }
    }

    /// Recalcule silencieusement les générateurs visibles (après un changement de règle).
    fn reset_reveals(&mut self, catalog: &Catalog) {
        self.revealed = catalog
            .generators
            .iter()
            .map(|g| self.is_visible(catalog, g.id))
            .collect();
    }

    /// Achat à l'Atelier stellaire avec des fragments.
    pub fn buy_perk(&mut self, catalog: &Catalog, id: PerkId, events: &mut Vec<DomainEvent>) {
        let Some(def) = catalog.perk(id) else {
            events.push(DomainEvent::PurchaseFailed {
                target: PurchaseTarget::Perk(id),
            });
            return;
        };
        if self.has_perk(id) || self.fragments_spendable() + 1e-9 < def.cost {
            events.push(DomainEvent::PurchaseFailed {
                target: PurchaseTarget::Perk(id),
            });
            return;
        }
        self.fragments_spent += def.cost;
        self.perks[id.index()] = true;
        events.push(DomainEvent::PerkPurchased { perk: id });
        if def.effect == PerkEffect::AutoBuyer {
            self.autobuy = true;
            events.push(DomainEvent::AutoBuyChanged { enabled: true });
        }
        self.check_achievements(catalog, events);
    }

    /// Active ou arrête l'automate d'achat.
    pub fn toggle_autobuy(&mut self, catalog: &Catalog, events: &mut Vec<DomainEvent>) {
        if !self.autobuy_unlocked(catalog) {
            events.push(DomainEvent::AutoBuyLocked);
            return;
        }
        self.autobuy = !self.autobuy;
        self.autobuy_timer = 0.0;
        events.push(DomainEvent::AutoBuyChanged {
            enabled: self.autobuy,
        });
    }

    /// Automate : achète le générateur visible qui rapporte le plus par
    /// poussière dépensée, tant que c'est abordable (10 achats max par passage).
    fn run_autobuyer(&mut self, catalog: &Catalog, events: &mut Vec<DomainEvent>) {
        for _ in 0..AUTOBUY_MAX_PER_TICK {
            let best = catalog
                .generators
                .iter()
                .filter(|g| self.is_visible(catalog, g.id))
                .map(|g| {
                    let cost = self.generator_cost(catalog, g.id, 1);
                    let gain = g.base_prod * economy::generator_multiplier(catalog, self, g.id);
                    (g.id, cost, gain / cost)
                })
                .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
            match best {
                Some((id, cost, _)) if self.stardust.can_afford(cost) => {
                    self.buy_generator(catalog, id, BuyMode::One, events);
                }
                _ => break,
            }
        }
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
                fragments: self.fragments,
                perks_owned: self.perks_owned(),
                perks_total: catalog.perks.len(),
                constellation_levels: self.constellation_levels_total(),
                golden_comets: self.golden_caught,
                black_comets: self.black_caught,
                challenges_done: self.challenges_done.iter().filter(|&&d| d).count(),
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

    fn perk(c: &Catalog, key: &str) -> PerkId {
        c.perk_by_key(key).unwrap()
    }

    #[test]
    fn perks_cost_fragments_but_keep_the_bonus() {
        let (c, mut s, mut ev) = setup();
        s.fragments = 3.0;
        let bonus_before = crate::prestige::fragment_multiplier(s.fragments);
        s.buy_perk(&c, perk(&c, "perk_start"), &mut ev);
        assert!(ev.contains(&DomainEvent::PerkPurchased {
            perk: perk(&c, "perk_start")
        }));
        assert_eq!(s.fragments_spendable(), 2.0);
        assert_eq!(
            crate::prestige::fragment_multiplier(s.fragments),
            bonus_before,
            "bonus intact"
        );
        ev.clear();
        // Trop cher (5 > 2) puis déjà acheté.
        s.buy_perk(&c, perk(&c, "perk_auto"), &mut ev);
        s.buy_perk(&c, perk(&c, "perk_start"), &mut ev);
        assert!(ev
            .iter()
            .all(|e| matches!(e, DomainEvent::PurchaseFailed { .. })));
        assert_eq!(s.fragments_spendable(), 2.0);
        let artisan = c.achievement_by_key("perk1").unwrap();
        assert!(s.has_achievement(artisan));
    }

    #[test]
    fn starting_kit_and_hold_survive_supernova() {
        let (c, mut s, mut ev) = setup();
        s.fragments = 1.0;
        s.buy_perk(&c, perk(&c, "perk_start"), &mut ev);
        let hold = c.upgrade_by_key("hold_0").unwrap();
        let alloy = c.upgrade_by_key("gen0_t0").unwrap();
        s.upgrades[hold.index()] = true;
        s.upgrades[alloy.index()] = true;
        s.total_run = 1e8;
        s.supernova(&c, &mut ev);
        assert_eq!(&s.owned[..2], &[10, 5]);
        assert!(s.has_upgrade(hold), "Récolte continue conservée");
        assert!(!s.has_upgrade(alloy));
    }

    #[test]
    fn discount_comet_storm_offline_and_nova_perks() {
        let (c, mut s, mut ev) = setup();
        let full = s.generator_cost(&c, GeneratorId(0), 1);
        s.fragments = 100.0;
        for key in [
            "perk_discount",
            "perk_comet",
            "perk_storm",
            "perk_offline",
            "perk_nova",
        ] {
            s.buy_perk(&c, perk(&c, key), &mut ev);
        }
        assert!((s.generator_cost(&c, GeneratorId(0), 1) - full * 0.8).abs() < 1e-9);
        s.stardust = Stardust::new(12.0);
        s.buy_generator(&c, GeneratorId(0), BuyMode::One, &mut ev);
        assert_eq!(s.owned[0], 1, "15 × 0.8 = 12 : abordable");
        // Comète : intervalle × 0.6.
        s.tick(&c, 50.0, 0.0, &mut ev);
        s.catch_comet(&c, 0.1, 0.0, &mut ev);
        assert!((s.comet.next_in - 90.0 * 0.6).abs() < 1e-9);
        assert!(
            (s.comet.storm_remaining - 60.0).abs() < 1e-9,
            "tempête de 60 s"
        );
        assert_eq!(s.offline_terms(&c), (1.0, 24.0 * 3600.0));
        s.total_run = 1e9; // 10 fragments de base → 15 avec Nova dorée
        assert_eq!(s.fragments_available_with(&c), 15.0);
    }

    #[test]
    fn autobuyer_buys_and_can_be_toggled() {
        let (c, mut s, mut ev) = setup();
        s.toggle_autobuy(&c, &mut ev);
        assert_eq!(ev, [DomainEvent::AutoBuyLocked]);
        ev.clear();
        s.fragments = 5.0;
        s.buy_perk(&c, perk(&c, "perk_auto"), &mut ev);
        assert!(s.autobuy && ev.contains(&DomainEvent::AutoBuyChanged { enabled: true }));
        s.stardust = Stardust::new(1000.0);
        s.tick(&c, 1.0, 0.5, &mut ev);
        assert!(s.owned.iter().sum::<u64>() > 0, "l'automate a acheté");
        let spent_on = s.owned.clone();
        s.toggle_autobuy(&c, &mut ev);
        assert!(!s.autobuy);
        s.stardust = Stardust::new(1e6);
        s.tick(&c, 2.0, 0.5, &mut ev);
        assert_eq!(s.owned, spent_on, "arrêté : plus d'achats");
    }

    #[test]
    fn buy_all_upgrades_cheapest_first() {
        let (c, mut s, mut ev) = setup();
        s.buy_all_upgrades(&c, &mut ev);
        assert_eq!(ev, [DomainEvent::BulkUpgrades { count: 0 }]);
        ev.clear();
        s.owned[0] = 10;
        s.owned[1] = 10;
        s.total_run = 2e4;
        s.stardust = Stardust::new(1_500.0);
        s.buy_all_upgrades(&c, &mut ev);
        // Abordables : Sondes t0 (225) puis Récolte continue (300) puis Gants (1 000) = 1 525 > 1 500.
        let bought: Vec<_> = ev
            .iter()
            .filter_map(|e| match e {
                DomainEvent::UpgradePurchased { upgrade, .. } => {
                    Some(c.upgrade(*upgrade).unwrap().key.clone())
                }
                _ => None,
            })
            .collect();
        assert_eq!(bought, ["gen0_t0", "hold_0"]);
        assert_eq!(ev.last(), Some(&DomainEvent::BulkUpgrades { count: 2 }));
    }

    #[test]
    fn next_locked_upgrade_is_the_cheapest_locked() {
        let (c, s, _) = setup();
        let next = s
            .next_locked_upgrade(&c)
            .map(|id| c.upgrade(id).unwrap().key.clone());
        assert_eq!(
            next.as_deref(),
            Some("gen0_t0"),
            "Alliage · Sondes (225) est la moins chère"
        );
    }

    fn constellation(c: &Catalog, key: &str) -> ConstellationId {
        c.constellation_by_key(key).unwrap()
    }

    #[test]
    fn late_generators_unlock_with_supernovas() {
        let (c, mut s, _) = setup();
        s.total_run = 1e15; // largement assez pour tout dévoiler… sauf les verrous
        s.owned[8] = 1;
        assert!(s.is_visible(&c, GeneratorId(8)));
        assert!(
            !s.is_visible(&c, GeneratorId(9)),
            "Trou blanc : 1 Supernova"
        );
        s.supernovas = 1;
        assert!(s.is_visible(&c, GeneratorId(9)));
        assert!(!s.is_visible(&c, GeneratorId(10)), "Quasars : 5 Supernovas");
        s.supernovas = 15;
        assert!(s.is_visible(&c, GeneratorId(11)));
    }

    #[test]
    fn supernovas_get_harder() {
        let (c, mut s, _) = setup();
        s.total_run = 1e9;
        assert_eq!(s.fragments_available_with(&c), 10.0);
        s.supernovas = 4; // seuil × 2
        assert_eq!(s.fragments_available_with(&c), 7.0);
    }

    #[test]
    fn constellations_level_up_with_growing_costs() {
        let (c, mut s, mut ev) = setup();
        let forge = constellation(&c, "const_forge");
        s.fragments = 10.0;
        let prod_before = economy::permanent_multiplier(&c, &s);
        s.buy_constellation(&c, forge, &mut ev); // coût 2
        s.buy_constellation(&c, forge, &mut ev); // coût 4 (3.2 arrondi)
        assert_eq!(s.constellation_level(forge), 2);
        assert_eq!(s.fragments_spendable(), 4.0);
        assert!(ev.contains(&DomainEvent::ConstellationUpgraded {
            constellation: forge,
            level: 2
        }));
        // +40 % de production (et +1 % du succès « Première étoile »).
        let ratio = economy::permanent_multiplier(&c, &s) / prod_before;
        assert!((ratio - 1.4 * 1.01).abs() < 1e-9, "{ratio}");
        assert!(s.has_achievement(c.achievement_by_key("const1").unwrap()));
        ev.clear();
        s.buy_constellation(&c, forge, &mut ev); // coût 6 (5.12 → 6) > 4 libres
        assert!(matches!(ev[0], DomainEvent::PurchaseFailed { .. }));
    }

    #[test]
    fn constellation_effects() {
        let (c, mut s, mut ev) = setup();
        s.fragments = 1000.0;
        let full = s.generator_cost(&c, GeneratorId(0), 1);
        for _ in 0..2 {
            s.buy_constellation(&c, constellation(&c, "const_builder"), &mut ev);
            s.buy_constellation(&c, constellation(&c, "const_comet"), &mut ev);
            s.buy_constellation(&c, constellation(&c, "const_hourglass"), &mut ev);
            s.buy_constellation(&c, constellation(&c, "const_hand"), &mut ev);
        }
        assert!((s.generator_cost(&c, GeneratorId(0), 1) - full * 0.97 * 0.97).abs() < 1e-9);
        assert_eq!(s.offline_terms(&c).1, OFFLINE_CAP_SECS + 4.0 * 3600.0);
        assert!((economy::click_percent(&c, &s) - 0.02).abs() < 1e-12);
        s.tick(&c, 50.0, 0.0, &mut ev);
        s.catch_comet(&c, 0.1, 0.0, &mut ev); // Tempête : ×7 + 2 niveaux
        assert_eq!(s.comet.boost(), 9.0);
        // Niveau max respecté.
        let hourglass = constellation(&c, "const_hourglass");
        for _ in 0..30 {
            s.buy_constellation(&c, hourglass, &mut ev);
        }
        assert_eq!(s.constellation_level(hourglass), 12);
    }

    fn challenge(c: &Catalog, key: &str) -> ChallengeId {
        c.challenge_by_key(key).unwrap()
    }

    #[test]
    fn challenges_need_supernovas_and_reset_the_run() {
        let (c, mut s, mut ev) = setup();
        let noclick = challenge(&c, "ch_noclick");
        s.start_challenge(&c, noclick, &mut ev);
        assert_eq!(
            ev,
            [DomainEvent::ChallengeUnavailable { challenge: noclick }]
        );
        ev.clear();
        s.supernovas = 1;
        s.stardust = Stardust::new(5000.0);
        s.total_run = 5000.0; // pas assez pour un fragment : simple relance
        s.owned[0] = 30;
        s.start_challenge(&c, noclick, &mut ev);
        assert_eq!(s.challenge, Some(noclick));
        assert_eq!(s.owned[0], 0);
        assert_eq!(s.stardust.value(), 0.0);
        assert_eq!(s.supernovas, 1, "pas de Supernova sans fragment");
        assert!(ev.contains(&DomainEvent::ChallengeStarted { challenge: noclick }));
        // Un seul Défi à la fois.
        ev.clear();
        s.start_challenge(&c, challenge(&c, "ch_noupgrade"), &mut ev);
        assert!(matches!(ev[0], DomainEvent::ChallengeUnavailable { .. }));
    }

    #[test]
    fn starting_a_challenge_cashes_in_fragments_first() {
        let (c, mut s, mut ev) = setup();
        s.supernovas = 1;
        s.total_run = 1e9; // 8 fragments au seuil 1.25e7
        s.start_challenge(&c, challenge(&c, "ch_noclick"), &mut ev);
        assert_eq!(s.supernovas, 2);
        assert!(s.fragments >= 8.0);
        assert!(ev
            .iter()
            .any(|e| matches!(e, DomainEvent::Supernova { .. })));
    }

    #[test]
    fn restrictions_are_enforced() {
        let (c, mut s, mut ev) = setup();
        s.supernovas = 3;
        s.start_challenge(&c, challenge(&c, "ch_noclick"), &mut ev);
        ev.clear();
        s.harvest(&c, &mut ev);
        assert_eq!(s.clicks, 0);
        assert_eq!(
            ev,
            [DomainEvent::ChallengeBlocked {
                restriction: Restriction::NoClick
            }]
        );
        s.abandon_challenge(&c, &mut ev);
        s.harvest(&c, &mut ev);
        assert_eq!(s.clicks, 1, "abandonné : les clics reviennent");

        ev.clear();
        s.start_challenge(&c, challenge(&c, "ch_noupgrade"), &mut ev);
        s.owned[0] = 10;
        s.stardust = Stardust::new(1e6);
        s.buy_upgrade(&c, c.upgrade_by_key("gen0_t0").unwrap(), &mut ev);
        s.buy_all_upgrades(&c, &mut ev);
        assert!(s.upgrades.iter().all(|&u| !u));
        s.abandon_challenge(&c, &mut ev);

        s.start_challenge(&c, challenge(&c, "ch_minimal"), &mut ev);
        s.total_run = 1e7;
        s.owned[2] = 1;
        assert!(s.is_visible(&c, GeneratorId(2)));
        assert!(
            !s.is_visible(&c, GeneratorId(3)),
            "Minimaliste : 3 premiers seulement"
        );
        s.abandon_challenge(&c, &mut ev);
        assert!(s.is_visible(&c, GeneratorId(3)));

        s.start_challenge(&c, challenge(&c, "ch_nocomet"), &mut ev);
        ev.clear();
        for _ in 0..600 {
            s.tick(&c, 1.0, 0.5, &mut ev);
        }
        assert!(!ev
            .iter()
            .any(|e| matches!(e, DomainEvent::CometSpawned { .. })));
    }

    #[test]
    fn completing_a_challenge_grants_its_reward() {
        let (c, mut s, mut ev) = setup();
        s.supernovas = 1;
        let noclick = challenge(&c, "ch_noclick");
        s.start_challenge(&c, noclick, &mut ev);
        s.owned[1] = 10;
        let before = economy::total_production(&c, &s);
        s.total_run = 1e8;
        ev.clear();
        s.tick(&c, 0.1, 0.5, &mut ev);
        assert!(ev.contains(&DomainEvent::ChallengeCompleted { challenge: noclick }));
        assert!(s.challenge_done(noclick));
        assert_eq!(s.challenge, None);
        assert!(s.has_achievement(c.achievement_by_key("challenge1").unwrap()));
        let ratio = economy::total_production(&c, &s) / before;
        assert!(ratio > 1.25, "×1,25 (+ succès) : {ratio}");
        // Déjà réussi : on ne peut pas le refaire.
        ev.clear();
        s.start_challenge(&c, noclick, &mut ev);
        assert!(matches!(ev[0], DomainEvent::ChallengeUnavailable { .. }));
    }

    #[test]
    fn timed_challenge_fails_after_15_minutes() {
        let (c, mut s, mut ev) = setup();
        s.supernovas = 5;
        let speed = challenge(&c, "ch_speed");
        s.start_challenge(&c, speed, &mut ev);
        ev.clear();
        s.tick(&c, 15.0 * 60.0 + 1.0, 0.5, &mut ev);
        assert!(ev.contains(&DomainEvent::ChallengeFailed { challenge: speed }));
        assert!(!s.challenge_done(speed));
        // Réussi : +1 fragment à chaque Supernova.
        s.challenges_done[speed.index()] = true;
        s.total_run = 1e9;
        let with_bonus = s.fragments_available_with(&c);
        s.challenges_done[speed.index()] = false;
        assert_eq!(with_bonus, s.fragments_available_with(&c) + 1.0);
    }

    #[test]
    fn ascese_reward_discounts_upgrades() {
        let (c, mut s, mut ev) = setup();
        s.challenges_done[challenge(&c, "ch_noupgrade").index()] = true;
        s.owned[0] = 10;
        s.stardust = Stardust::new(225.0 * 0.75);
        s.buy_upgrade(&c, c.upgrade_by_key("gen0_t0").unwrap(), &mut ev);
        assert!(ev.iter().any(|e| matches!(e, DomainEvent::UpgradePurchased { cost, .. } if (*cost - 168.75).abs() < 1e-9)));
    }

    #[test]
    fn rare_comets() {
        assert_eq!(CometKind::from_roll(0.5), CometKind::Normal);
        assert_eq!(CometKind::from_roll(0.90), CometKind::Golden);
        assert_eq!(CometKind::from_roll(0.97), CometKind::Black);

        // Comète dorée : ×77 pendant 7 s.
        let (c, mut s, mut ev) = setup();
        s.tick(&c, 50.0, 0.90, &mut ev);
        assert!(ev.contains(&DomainEvent::CometSpawned {
            kind: CometKind::Golden
        }));
        s.catch_comet(&c, 0.1, 0.5, &mut ev);
        assert_eq!(s.comet.boost(), 77.0);
        assert_eq!(s.golden_caught, 1);

        // Comète noire capturée : +1 fragment.
        let (c, mut s, mut ev) = setup();
        s.tick(&c, 50.0, 0.97, &mut ev);
        s.catch_comet(&c, 0.1, 0.5, &mut ev);
        assert_eq!(s.fragments, 1.0);
        assert_eq!(s.black_caught, 1);

        // Comète noire ratée : elle vole 10 % de la poussière.
        let (c, mut s, mut ev) = setup();
        s.tick(&c, 50.0, 0.97, &mut ev);
        s.stardust = Stardust::new(1000.0);
        ev.clear();
        s.tick(&c, 16.0, 0.5, &mut ev);
        assert!(ev.contains(&DomainEvent::CometStole { amount: 100.0 }));
        assert!((s.stardust.value() - 900.0).abs() < 1e-9);
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
        assert!(ev.contains(&DomainEvent::CometSpawned {
            kind: CometKind::Normal
        }));
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
        assert_eq!(s.fragments, 9.0, "floor(sqrt(9e8 / 1e7))");
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
                if *fragments_gained == 9.0 && *total_fragments == 9.0
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
        assert_eq!(s.owned.len(), 12);
        assert_eq!(s.total_run, 0.0);
    }
}
