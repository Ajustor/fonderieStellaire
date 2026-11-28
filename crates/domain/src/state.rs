//! `GameState` : l'agrégat racine et toutes les règles qui le modifient.

use crate::achievement::{AchievementContext, AchievementId};
use crate::catalog::balance::{
    BIGBANG_CRUNCHES, CRUNCH_FRAGMENTS, CRUNCH_PRODUCTION, CRUNCH_SUPERNOVAS,
    HOLD_HARVEST_INTERVAL, LAB_UNLOCK_CRUNCHES, OFFLINE_CAP_SECS, OFFLINE_RATE,
    PARTICLES_PER_CRUNCHES, PARTICLE_FRAGMENTS, REVEAL_RATIO, UNIVERSES_UNLOCK_CRUNCHES,
};
use crate::catalog::Catalog;
use crate::challenge::{ChallengeId, ChallengeReward, Restriction};
use crate::comet::{
    CometKind, CometReward, CometState, BLACK_THEFT, GOLDEN_DURATION, GOLDEN_MULTIPLIER,
};
use crate::constellation::{ConstellationEffect, ConstellationId};
use crate::cosmic::{
    next_event_delay, Boost, BoostKind, EventEffect, CONVERGENCE_SECS, EVENT_EXPIRY,
};
use crate::cosmos::{particles_for, AttractorEffect};
use crate::economy;
use crate::events::RelicOutcome;
use crate::events::{DomainEvent, PurchaseTarget};
use crate::generator::{milestones_crossed, BuyMode, GeneratorId};
use crate::lab::{LabEffect, LabId};
use crate::megastructure::{self, GridEffects, StructureKind};
use crate::perk::{PerkEffect, PerkId};
use crate::prestige::{fragment_divisor, fragments_for};
use crate::relic::{roll_rarity, RelicEffect, RelicId, RelicSet};
use crate::stardust::Stardust;
use crate::starmap::{self, StarMap, SystemKind, BASE_FLEETS};
use crate::universe::{KeyReward, UniverseId, UniverseRules, ENTROPY_FLOOR};
use crate::upgrade::{Effect, UpgradeId};

/// Intervalle entre deux vérifications de succès.
const ACHIEVEMENT_CHECK_INTERVAL: f64 = 1.0;
/// Intervalle entre deux passages de l'automate d'achat.
const AUTOBUY_INTERVAL: f64 = 1.0;
/// Récolte maintenue : au-delà de cet écart entre deux répétitions, la touche
/// a été relâchée et la cadence repart (pas de rattrapage). La répétition du
/// clavier, elle, arrive au moins 20 fois par seconde.
const HOLD_HARVEST_RESTART: f64 = 0.3;
/// Historique de production : un point par quart d'heure de jeu, au plus 2000.
const HISTORY_STEP: f64 = 15.0 * 60.0;
const HISTORY_MAX: usize = 2000;
/// Systèmes explorés pour débloquer le pilote automatique des flottes.
pub const AUTO_FLEETS_EXPLORED: usize = 10;
/// Nombre maximal de générateurs achetés par passage de l'automate.
const AUTOBUY_MAX_PER_TICK: usize = 10;

/// Une flotte en expédition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Expedition {
    pub target: usize,
    /// Secondes restantes et durée totale.
    pub left: f64,
    pub total: f64,
    /// Issue tirée au départ.
    pub success: bool,
    /// Tirage du butin (relique), fait au départ lui aussi.
    pub loot: f64,
    /// Prospection : retour vers un système déjà exploré, pour des reliques.
    pub prospect: bool,
}

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
    /// Big Crunch déclenchés (matière noire).
    pub crunches: u32,
    /// Matière noire dépensée au Laboratoire (le bonus reste calculé sur `crunches`).
    pub dark_matter_spent: u32,
    /// Recherches terminées, et automatismes mis en pause.
    pub lab: Vec<bool>,
    pub lab_paused: Vec<bool>,
    /// Recherche en cours et secondes restantes.
    pub research: Option<(LabId, f64)>,
    /// Univers du cycle de Big Crunch en cours, et Clés obtenues.
    pub universe: UniverseId,
    pub universe_keys: Vec<bool>,
    /// `play_time` du début du cycle (pour l'entropie).
    pub run_started: f64,
    /// Big Bang déclenchés et particules primordiales (gagnées / dépensées :
    /// le bonus reste calculé sur `particles`).
    pub big_bangs: u32,
    pub particles: u32,
    pub particles_spent: u32,
    /// Générateurs primordiaux forgés, indexés par générateur.
    pub forged: Vec<bool>,
    /// Étapes du Grand Attracteur achevées, et secondes restantes de l'étape
    /// en construction.
    pub attractor: usize,
    pub attractor_build: Option<f64>,
    /// Acte III : graine de la Carte stellaire (0 : pas encore de carte), la
    /// carte qui en découle (non sauvegardée), les systèmes explorés et les
    /// flottes en expédition.
    pub map_seed: u64,
    pub map: StarMap,
    pub explored: Vec<bool>,
    pub fleets: Vec<Expedition>,
    pub expeditions_launched: u32,
    /// Reliques : meilleure rareté possédée (indexée par relique), reliques
    /// équipées, et Éclats (doublons).
    pub relics: Vec<Option<u8>>,
    pub equipped: Vec<RelicId>,
    pub shards: u32,
    /// Événements cosmiques : délai avant le prochain, événement en attente
    /// (index, secondes pour répondre), bonus temporaires, événements résolus.
    pub event_timer: f64,
    pub pending_event: Option<(usize, f64)>,
    pub boosts: Vec<Boost>,
    pub events_resolved: u32,
    /// Convergence : secondes restantes si elle est en cours, et achevée ou non.
    pub convergence: Option<f64>,
    pub converged: bool,
    /// Acte IV : matière exotique (disponible, gagnée au total) et grille des
    /// Mégastructures (une case = une structure ou rien).
    pub exotic: f64,
    pub exotic_total: f64,
    pub grid: Vec<Option<StructureKind>>,
    /// Éveil du Cosmos : secondes restantes s'il est en cours, et achevé.
    pub awakening: Option<f64>,
    pub awakened: bool,
    /// Pilote automatique des flottes (après 10 systèmes explorés).
    pub auto_fleets: bool,
    /// Règles imposées (univers de poche du Défi du jour), à la place de
    /// celles de l'univers en cours.
    pub rules_override: Option<UniverseRules>,
    /// Défi du jour : jour du dernier défi, meilleur score ce jour-là, et
    /// récompense déjà touchée ou non.
    pub daily_day: u64,
    pub daily_best: f64,
    pub daily_rewarded: bool,
    /// Historique de la production (temps de jeu, log10 de la prod/s), un
    /// point tous les quarts d'heure, et record de production.
    pub history: Vec<(f64, f32)>,
    pub best_production: f64,
    /// Temps de jeu des grandes étapes (0 : pas encore) : 1re Supernova,
    /// 1er Big Crunch, 1er Big Bang, fins des Actes II, III et IV.
    pub milestones: [f64; 6],
    /// Niveau de chaque Constellation.
    pub constellations: Vec<u32>,
    revealed: Vec<bool>,
    /// Mode d'achat choisi par le joueur (×1, ×10…) : l'automate le suit.
    /// Réglage d'interface, non sauvegardé.
    pub autobuy_mode: BuyMode,
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
            crunches: 0,
            dark_matter_spent: 0,
            lab: Vec::new(),
            lab_paused: Vec::new(),
            research: None,
            universe: UniverseId::ORIGIN,
            universe_keys: Vec::new(),
            run_started: 0.0,
            big_bangs: 0,
            particles: 0,
            particles_spent: 0,
            forged: Vec::new(),
            attractor: 0,
            attractor_build: None,
            map_seed: 0,
            map: StarMap {
                systems: Vec::new(),
                columns: 0,
            },
            explored: Vec::new(),
            fleets: Vec::new(),
            expeditions_launched: 0,
            relics: Vec::new(),
            equipped: Vec::new(),
            shards: 0,
            event_timer: 0.0,
            pending_event: None,
            boosts: Vec::new(),
            events_resolved: 0,
            convergence: None,
            converged: false,
            exotic: 0.0,
            exotic_total: 0.0,
            grid: Vec::new(),
            awakening: None,
            awakened: false,
            auto_fleets: false,
            rules_override: None,
            daily_day: 0,
            daily_best: 0.0,
            daily_rewarded: false,
            history: Vec::new(),
            best_production: 0.0,
            milestones: [0.0; 6],
            constellations: Vec::new(),
            revealed: Vec::new(),
            autobuy_mode: BuyMode::One,
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
        self.lab.resize(catalog.lab.len(), false);
        self.universe_keys.resize(catalog.universes.len(), false);
        if self.universe.index() >= catalog.universes.len() {
            self.universe = UniverseId::ORIGIN;
        }
        self.lab_paused.resize(catalog.lab.len(), false);
        self.dark_matter_spent = self.dark_matter_spent.min(self.crunches);
        self.forged.resize(catalog.generators.len(), false);
        for (f, g) in self.forged.iter_mut().zip(&catalog.generators) {
            *f &= g.is_primordial();
        }
        self.particles_spent = self.particles_spent.min(self.particles);
        self.relics.resize(catalog.relics.len(), None);
        for r in self.relics.iter_mut().flatten() {
            *r = (*r).min(3);
        }
        let owned: Vec<bool> = self.relics.iter().map(|r| r.is_some()).collect();
        let mut seen = Vec::new();
        self.equipped.retain(|id| {
            let ok = owned.get(id.index()).copied().unwrap_or(false) && !seen.contains(id);
            seen.push(*id);
            ok
        });
        let slots = self.relic_slots();
        self.equipped.truncate(slots);
        if self
            .pending_event
            .is_some_and(|(i, left)| i >= catalog.events.len() || !left.is_finite())
        {
            self.pending_event = None;
        }
        self.boosts.retain(|b| {
            b.left.is_finite() && b.left > 0.0 && b.factor.is_finite() && b.factor > 0.0
        });
        if !self.event_timer.is_finite() || self.event_timer < 0.0 {
            self.event_timer = 0.0;
        }
        if self.converged || self.convergence.is_some_and(|l| !l.is_finite()) {
            self.convergence = None;
        }
        self.history.retain(|(t, p)| t.is_finite() && p.is_finite());
        if self.history.len() > HISTORY_MAX {
            let extra = self.history.len() - HISTORY_MAX;
            self.history.drain(..extra);
        }
        if !self.best_production.is_finite() || self.best_production < 0.0 {
            self.best_production = 0.0;
        }
        for m in &mut self.milestones {
            if !m.is_finite() || *m < 0.0 {
                *m = 0.0;
            }
        }
        self.auto_fleets &= self.auto_fleets_unlocked();
        self.grid.resize(megastructure::CELLS, None);
        let mut core = false;
        for c in self.grid.iter_mut() {
            if *c == Some(StructureKind::Core) {
                if core {
                    *c = None;
                }
                core = true;
            }
        }
        for v in [&mut self.exotic, &mut self.exotic_total] {
            if !v.is_finite() || *v < 0.0 {
                *v = 0.0;
            }
        }
        // Carte stellaire : reconstruite depuis sa graine.
        if self.map_seed != 0 && self.map.systems.is_empty() {
            self.map = starmap::generate(self.map_seed);
        }
        self.explored.resize(self.map.systems.len(), false);
        if let Some(home) = self.explored.first_mut() {
            *home = true;
        }
        let n = self.map.systems.len();
        self.fleets
            .retain(|f| f.target < n && f.left.is_finite() && f.total > 0.0);
        let mut seen = Vec::new();
        self.fleets.retain(|f| {
            let fresh = !seen.contains(&f.target);
            seen.push(f.target);
            fresh
        });
        self.attractor = self.attractor.min(catalog.attractor.len());
        if self.attractor >= catalog.attractor.len()
            || self.attractor_build.is_some_and(|l| !l.is_finite())
        {
            self.attractor_build = None;
        }
        if self
            .research
            .is_some_and(|(id, left)| id.index() >= catalog.lab.len() || !left.is_finite())
        {
            self.research = None;
        }
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
        perks
            * constellations
            * self.rules(catalog).cost
            * self.key_product(catalog, |r| match r {
                KeyReward::GeneratorDiscount(f) => Some(f),
                _ => None,
            })
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
            / self.map_bonus(SystemKind::Pulsar, 0.10)
            / (1.0
                + self.relic_sum(catalog, |e| match e {
                    RelicEffect::Comets(x) => Some(x),
                    _ => None,
                }))
            * self.rules(catalog).comet_interval
            * self
                .perk_effects(catalog)
                .filter_map(|e| match e {
                    PerkEffect::CometMagnet(f) => Some(f),
                    _ => None,
                })
                .product::<f64>()
    }

    fn storm_factor(&self, catalog: &Catalog) -> f64 {
        self.rules(catalog).storm_duration
            * self
                .perk_effects(catalog)
                .filter_map(|e| match e {
                    PerkEffect::LongStorm(f) => Some(f),
                    _ => None,
                })
                .product::<f64>()
    }

    /// Règles de l'univers en cours.
    pub fn rules(&self, catalog: &Catalog) -> UniverseRules {
        if let Some(r) = self.rules_override {
            return r;
        }
        catalog
            .universe(self.universe)
            .map(|u| u.rules)
            .unwrap_or(UniverseRules::NORMAL)
    }

    pub fn has_universe_key(&self, id: UniverseId) -> bool {
        self.universe_keys.get(id.index()).copied().unwrap_or(false)
    }

    /// Produit des bonus de Clés d'un type.
    fn key_product(&self, catalog: &Catalog, f: impl Fn(KeyReward) -> Option<f64>) -> f64 {
        catalog
            .universes
            .iter()
            .filter(|u| self.has_universe_key(u.id))
            .filter_map(|u| u.key_reward.and_then(&f))
            .product()
    }

    /// Choisir son univers est-il possible ? (10 Big Crunch, ou déjà un Big Bang)
    pub fn universes_unlocked(&self) -> bool {
        self.crunches >= UNIVERSES_UNLOCK_CRUNCHES || self.big_bangs > 0
    }

    /// Facteur d'entropie du cycle en cours (1 = aucune perte).
    pub fn entropy_factor(&self, catalog: &Catalog) -> f64 {
        let per_min = self.rules(catalog).entropy_per_min;
        if per_min <= 0.0 {
            return 1.0;
        }
        let minutes = (self.play_time - self.run_started).max(0.0) / 60.0;
        (1.0 - per_min * minutes).max(ENTROPY_FLOOR)
    }

    /// Multiplicateur de production de l'univers (règles, entropie, Clés).
    pub fn universe_production(&self, catalog: &Catalog) -> f64 {
        self.rules(catalog).production
            * self.entropy_factor(catalog)
            * self.key_product(catalog, |r| match r {
                KeyReward::Production(f) => Some(f),
                _ => None,
            })
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
            * self.crunch_fragments()
            * self.particle_fragments()
            * self.map_bonus(SystemKind::Cluster, 0.10)
            * self.boost_factor(BoostKind::Fragments)
            * self.grid_effects().fragments
            * (1.0
                + self.relic_sum(catalog, |e| match e {
                    RelicEffect::Fragments(x) => Some(x),
                    _ => None,
                }))
            * self.attractor_product(catalog, |e| match e {
                AttractorEffect::Fragments(f) => Some(f),
                _ => None,
            })
            * self.rules(catalog).fragments
            * self.key_product(catalog, |r| match r {
                KeyReward::Fragments(f) => Some(f),
                _ => None,
            })
            * self
                .lab_effects(catalog)
                .filter_map(|e| match e {
                    LabEffect::Fragments(f) => Some(f),
                    _ => None,
                })
                .product::<f64>()
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
        let (mut rate, mut cap) = self
            .perk_effects(catalog)
            .find_map(|e| match e {
                PerkEffect::OfflineMastery { rate, cap_secs } => Some((rate, cap_secs)),
                _ => None,
            })
            .unwrap_or((OFFLINE_RATE, OFFLINE_CAP_SECS));
        for e in self.lab_effects(catalog) {
            if let LabEffect::Offline { rate: r, cap_secs } = e {
                rate = rate.max(r);
                cap = cap.max(cap_secs);
            }
        }
        for e in self.attractor_effects(catalog) {
            if let AttractorEffect::Offline(r) = e {
                rate = rate.max(r);
            }
        }
        // Trous noirs explorés : +2 h de plafond chacun (selon le danger).
        cap += (self.map_bonus(SystemKind::BlackHole, 2.0) - 1.0) * 3600.0;
        // Reliques : plafond et rendement.
        cap += self.relic_sum(catalog, |e| match e {
            RelicEffect::OfflineHours(h) => Some(h),
            _ => None,
        }) * 3600.0;
        rate += self.relic_sum(catalog, |e| match e {
            RelicEffect::OfflineRate(x) => Some(x),
            _ => None,
        });
        if self.relic_set_active(catalog, RelicSet::Eclipse) {
            rate += 0.25;
        }
        let extra_hours = self.constellation_sum(catalog, |e| match e {
            ConstellationEffect::OfflineHours(h) => Some(h),
            _ => None,
        });
        let (key_factor, key_cap) = catalog
            .universes
            .iter()
            .filter(|u| self.has_universe_key(u.id))
            .filter_map(|u| match u.key_reward {
                Some(KeyReward::Offline {
                    factor,
                    extra_cap_secs,
                }) => Some((factor, extra_cap_secs)),
                _ => None,
            })
            .fold((1.0, 0.0), |(f, c), (kf, kc)| (f * kf, c + kc));
        (
            rate * self.rules(catalog).offline * key_factor,
            cap + extra_hours * 3600.0 + key_cap,
        )
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
        if def.is_primordial() && !self.is_forged(id) {
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
    /// `HOLD_HARVEST_INTERVAL` secondes de jeu. Les répétitions trop rapprochées
    /// sont ignorées ; quand elles arrivent en retard (images/s basses, clavier
    /// lent), les récoltes dues sont rattrapées : exactement 10 par seconde
    /// quelle que soit la machine.
    pub fn harvest_held(&mut self, catalog: &Catalog, events: &mut Vec<DomainEvent>) {
        if self.active_restriction(catalog) == Some(Restriction::NoClick) {
            return;
        }
        if !economy::hold_harvest_unlocked(catalog, self) {
            events.push(DomainEvent::HoldHarvestLocked);
            return;
        }
        let since = self.play_time - self.last_hold_harvest;
        if since < HOLD_HARVEST_INTERVAL - 1e-9 {
            return;
        }
        let due = if since > HOLD_HARVEST_RESTART {
            // Nouvelle prise (touche relâchée entre-temps) : la cadence repart.
            self.last_hold_harvest = self.play_time;
            1
        } else {
            let due = ((since + 1e-9) / HOLD_HARVEST_INTERVAL).floor() as u32;
            self.last_hold_harvest += due as f64 * HOLD_HARVEST_INTERVAL;
            due
        };
        for _ in 0..due {
            self.harvest(catalog, events);
        }
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
        if self.rules(catalog).no_upgrades {
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
        self.comet.rare_factor = self
            .lab_effects(catalog)
            .filter_map(|e| match e {
                LabEffect::RareComets(f) => Some(f),
                _ => None,
            })
            .product::<f64>()
            * self.key_product(catalog, |r| match r {
                KeyReward::Comets { rare, .. } => Some(rare),
                _ => None,
            });
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
        self.advance_research(catalog, dt, events);
        self.advance_attractor(catalog, dt, events);
        self.ensure_map(catalog, roll, events);
        self.advance_expeditions(catalog, dt, events);
        self.advance_convergence(catalog, dt, events);
        self.advance_boosts(dt);
        self.advance_cosmic_events(catalog, dt, roll, events);
        self.accrue_exotic(dt);
        self.advance_awakening(catalog, dt, events);

        // Filet à comètes : capture les comètes normales après 2 s de passage.
        if self.lab_active(catalog, LabEffect::CometNet)
            && self.comet.kind == CometKind::Normal
            && self.comet.active.is_some_and(|e| e >= 2.0)
        {
            self.catch_comet(catalog, roll, 1.0 - roll, events);
        }

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
            self.run_lab_automation(catalog, events);
            if self.auto_fleets {
                self.run_auto_fleets(catalog, roll, events);
            }
            if self
                .history
                .last()
                .is_none_or(|&(t, _)| self.play_time - t >= HISTORY_STEP)
            {
                self.record_history(catalog);
            }
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
                amount: amount
                    * (1.0 + shower_bonus)
                    * self.key_product(catalog, |r| match r {
                        KeyReward::Comets { shower, .. } => Some(shower),
                        _ => None,
                    }),
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
        self.mark_milestone(0);
        self.reset_run(catalog);
        events.push(DomainEvent::Supernova {
            fragments_gained: gained,
            total_fragments: self.fragments,
        });
        self.check_universe_key(catalog, events);
        self.check_achievements(catalog, events);
    }

    /// Remet le cycle à zéro (Supernova ou lancement d'un Défi).
    fn reset_run(&mut self, catalog: &Catalog) {
        self.run_started = self.play_time;
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
        // Les recherches et le Grand Attracteur avancent en temps réel, même jeu fermé.
        self.advance_research(catalog, elapsed_secs, events);
        self.advance_attractor(catalog, elapsed_secs, events);
        self.advance_fleets_offline(catalog, elapsed_secs, events);
        self.accrue_exotic(elapsed_secs);
        self.advance_awakening(catalog, elapsed_secs, events);
        self.advance_convergence(catalog, elapsed_secs, events);
        self.advance_boosts(elapsed_secs);
        if let Some((event, left)) = self.pending_event {
            if left <= elapsed_secs {
                self.pending_event = None;
                events.push(DomainEvent::CosmicEventExpired { event });
            } else {
                self.pending_event = Some((event, left - elapsed_secs));
            }
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
        self.record_history(catalog);
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
        if self.rules(catalog).no_upgrades {
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

    /// Multiplicateur de production de la matière noire : ×2 par Big Crunch
    /// (2^crunches), ×3 par Big Crunch avec le Cœur de superamas du Grand
    /// Attracteur.
    pub fn crunch_production(&self, catalog: &Catalog) -> f64 {
        self.crunch_production_with(catalog, self.crunches)
    }

    /// Multiplicateur de production avec `crunches` Big Crunch (aperçu du
    /// prochain), Cœur de superamas compris.
    pub fn crunch_production_with(&self, catalog: &Catalog, crunches: u32) -> f64 {
        let boost = self.attractor_product(catalog, |e| match e {
            AttractorEffect::DarkMatter(f) => Some(f),
            _ => None,
        });
        (1.0 + CRUNCH_PRODUCTION * boost).powi(crunches as i32)
    }

    /// Multiplicateur de fragments de la matière noire.
    pub fn crunch_fragments(&self) -> f64 {
        1.0 + CRUNCH_FRAGMENTS * self.crunches as f64
    }

    /// Le Big Crunch est-il possible ? (15 Supernovas, aucun Défi en cours)
    pub fn can_big_crunch(&self) -> bool {
        self.supernovas >= CRUNCH_SUPERNOVAS && self.challenge.is_none()
    }

    /// Big Crunch : remet à zéro tout le niveau Supernova (fragments, Atelier,
    /// Constellations, Supernovas, Défis) contre un bonus permanent.
    /// Les succès, les statistiques et les comètes capturées sont conservés.
    pub fn big_crunch(
        &mut self,
        catalog: &Catalog,
        next: UniverseId,
        events: &mut Vec<DomainEvent>,
    ) {
        if !self.can_big_crunch() {
            events.push(DomainEvent::BigCrunchUnavailable {
                supernovas: self.supernovas,
                needed: CRUNCH_SUPERNOVAS,
            });
            return;
        }
        self.crunches += 1;
        self.supernovas = 0;
        self.fragments = 0.0;
        self.fragments_spent = 0.0;
        self.fragments_this_run = 0.0;
        let keep_atelier = self.lab_owned_effect(catalog, LabEffect::KeepAtelier);
        if !keep_atelier {
            self.perks.iter_mut().for_each(|p| *p = false);
            self.autobuy = false;
        }
        self.constellations.iter_mut().for_each(|l| *l = 0);
        if !self.lab_owned_effect(catalog, LabEffect::KeepChallenges) {
            self.challenges_done.iter_mut().for_each(|d| *d = false);
        }
        self.challenge = None;
        // Départ lancé : des Supernovas déjà faites (Trou blanc débloqué…).
        self.supernovas = self
            .lab_effects(catalog)
            .filter_map(|e| match e {
                LabEffect::HeadStart(n) => Some(n),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        // Univers du prochain cycle (l'origine tant que les univers sont fermés).
        self.universe = if self.universes_unlocked() && catalog.universe(next).is_some() {
            next
        } else {
            UniverseId::ORIGIN
        };
        self.reset_run(catalog);
        self.mark_milestone(1);
        events.push(DomainEvent::BigCrunch {
            crunches: self.crunches,
        });
        events.push(DomainEvent::UniverseEntered {
            universe: self.universe,
        });
        // « Départ lancé » peut suffire pour une Clé à objectif bas.
        self.check_universe_key(catalog, events);
        self.check_achievements(catalog, events);
    }

    /// Clé de l'univers en cours si son objectif de Supernovas est atteint.
    fn check_universe_key(&mut self, catalog: &Catalog, events: &mut Vec<DomainEvent>) {
        let Some(def) = catalog.universe(self.universe) else {
            return;
        };
        if def.key_reward.is_none()
            || self.has_universe_key(def.id)
            || self.supernovas < def.key_goal
        {
            return;
        }
        self.universe_keys[def.id.index()] = true;
        events.push(DomainEvent::UniverseKeyEarned { universe: def.id });
        self.check_achievements(catalog, events);
    }

    /// Le Laboratoire est-il ouvert ? (10 Big Crunch, ou déjà un Big Bang)
    pub fn lab_unlocked(&self) -> bool {
        self.crunches >= LAB_UNLOCK_CRUNCHES || self.big_bangs > 0
    }

    // ------------------------------------------------------------------
    // Big Bang, générateurs primordiaux, Grand Attracteur
    // ------------------------------------------------------------------

    /// Multiplicateur de production des particules : ×10 par particule.
    pub fn particle_production(&self) -> f64 {
        crate::cosmos::particle_production_for(self.particles)
    }

    /// Multiplicateur de fragments des particules : +25 % par particule.
    pub fn particle_fragments(&self) -> f64 {
        1.0 + PARTICLE_FRAGMENTS * self.particles as f64
    }

    pub fn particles_spendable(&self) -> u32 {
        self.particles.saturating_sub(self.particles_spent)
    }

    /// Le Big Bang est-il possible ? (15 Big Crunch, aucun Défi en cours)
    pub fn can_big_bang(&self) -> bool {
        self.crunches >= BIGBANG_CRUNCHES && self.challenge.is_none()
    }

    /// Particules que rapporterait un Big Bang maintenant.
    pub fn particles_available(&self) -> u32 {
        if self.can_big_bang() {
            particles_for(self.crunches, PARTICLES_PER_CRUNCHES)
        } else {
            0
        }
    }

    /// Big Bang : remet à zéro les Big Crunch (et tout le niveau Supernova)
    /// contre des particules primordiales. Le Laboratoire, les Clés d'univers,
    /// les succès, les forges et le Grand Attracteur sont conservés.
    pub fn big_bang(&mut self, catalog: &Catalog, events: &mut Vec<DomainEvent>) {
        if !self.can_big_bang() {
            events.push(DomainEvent::BigBangUnavailable {
                crunches: self.crunches,
                needed: BIGBANG_CRUNCHES,
            });
            return;
        }
        let gained = self.particles_available();
        self.particles += gained;
        self.big_bangs += 1;
        self.crunches = 0;
        self.dark_matter_spent = 0;
        self.fragments = 0.0;
        self.fragments_spent = 0.0;
        self.fragments_this_run = 0.0;
        if !self.lab_owned_effect(catalog, LabEffect::KeepAtelier) {
            self.perks.iter_mut().for_each(|p| *p = false);
            self.autobuy = false;
        }
        self.constellations.iter_mut().for_each(|l| *l = 0);
        if !self.lab_owned_effect(catalog, LabEffect::KeepChallenges) {
            self.challenges_done.iter_mut().for_each(|d| *d = false);
        }
        self.challenge = None;
        self.supernovas = self
            .lab_effects(catalog)
            .filter_map(|e| match e {
                LabEffect::HeadStart(n) => Some(n),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        self.universe = UniverseId::ORIGIN;
        self.reset_run(catalog);
        self.mark_milestone(2);
        events.push(DomainEvent::BigBang {
            particles_gained: gained,
            total_particles: self.particles,
        });
        self.check_achievements(catalog, events);
    }

    pub fn is_forged(&self, id: GeneratorId) -> bool {
        self.forged.get(id.index()).copied().unwrap_or(false)
    }

    /// Un générateur primordial peut-il être forgé ? (dans l'ordre)
    pub fn can_forge(&self, catalog: &Catalog, id: GeneratorId) -> bool {
        let Some(def) = catalog.generator(id) else {
            return false;
        };
        let previous_ok = id.index() == 0
            || catalog
                .generator(GeneratorId(id.index() - 1))
                .is_none_or(|p| !p.is_primordial() || self.is_forged(p.id));
        def.is_primordial()
            && !self.is_forged(id)
            && previous_ok
            && self.particles_spendable() >= def.particle_cost
    }

    /// Forge un générateur primordial : il apparaît pour toujours.
    pub fn forge(&mut self, catalog: &Catalog, id: GeneratorId, events: &mut Vec<DomainEvent>) {
        if !self.can_forge(catalog, id) {
            events.push(DomainEvent::PurchaseFailed {
                target: PurchaseTarget::Forge(id),
            });
            return;
        }
        let Some(def) = catalog.generator(id) else {
            return;
        };
        self.particles_spent += def.particle_cost;
        self.forged[id.index()] = true;
        events.push(DomainEvent::GeneratorForged { generator: id });
        self.check_reveals(catalog, events);
        self.check_achievements(catalog, events);
    }

    /// Effets des étapes achevées du Grand Attracteur.
    pub fn attractor_effects<'a>(
        &'a self,
        catalog: &'a Catalog,
    ) -> impl Iterator<Item = AttractorEffect> + 'a {
        catalog
            .attractor
            .iter()
            .take(self.attractor)
            .map(|s| s.effect)
    }

    fn attractor_product(
        &self,
        catalog: &Catalog,
        f: impl Fn(AttractorEffect) -> Option<f64>,
    ) -> f64 {
        self.attractor_effects(catalog).filter_map(f).product()
    }

    /// Multiplicateur de production du Grand Attracteur.
    pub fn attractor_production(&self, catalog: &Catalog) -> f64 {
        self.attractor_product(catalog, |e| match e {
            AttractorEffect::Production(f) | AttractorEffect::Awakening(f) => Some(f),
            _ => None,
        })
    }

    // ------------------------------------------------------------------
    // Acte III : Carte stellaire et expéditions
    // ------------------------------------------------------------------

    /// La Carte stellaire s'ouvre à l'éveil du Grand Attracteur.
    pub fn starmap_unlocked(&self, catalog: &Catalog) -> bool {
        self.attractor_awakened(catalog)
    }

    pub fn is_explored(&self, system: usize) -> bool {
        self.explored.get(system).copied().unwrap_or(false)
    }

    /// Un système est visible s'il est exploré ou relié à un système exploré.
    pub fn is_revealed(&self, system: usize) -> bool {
        self.map.systems.get(system).is_some_and(|s| {
            self.is_explored(system) || s.links.iter().any(|&l| self.is_explored(l))
        })
    }

    pub fn explored_count(&self) -> usize {
        self.explored.iter().skip(1).filter(|&&e| e).count()
    }

    /// Flottes disponibles au total (une de plus par Civilisation explorée).
    pub fn fleets_max(&self) -> usize {
        BASE_FLEETS
            + self
                .map
                .systems
                .iter()
                .filter(|s| s.kind == SystemKind::Civilization && self.is_explored(s.index))
                .count()
    }

    /// Chance de succès d'une expédition (le danger la baisse, l'expérience
    /// des systèmes déjà explorés la remonte).
    pub fn expedition_chance(&self, system: usize) -> f64 {
        self.expedition_chance_with(None, system)
    }

    /// Chance de succès, reliques comprises (quand le catalogue est connu).
    pub fn expedition_chance_in(&self, catalog: &Catalog, system: usize) -> f64 {
        self.expedition_chance_with(Some(catalog), system)
    }

    fn expedition_chance_with(&self, catalog: Option<&Catalog>, system: usize) -> f64 {
        let danger = self.map.systems.get(system).map(|s| s.danger).unwrap_or(5) as f64;
        // Une prospection (système déjà exploré) est plus sûre.
        let prospect = if self.is_explored(system) { 0.15 } else { 0.0 };
        let relics = catalog
            .map(|c| {
                self.relic_sum(c, |e| match e {
                    RelicEffect::ExpeditionLuck(x) => Some(x),
                    _ => None,
                }) + if self.relic_set_active(c, RelicSet::Voyager) {
                    0.10
                } else {
                    0.0
                }
            })
            .unwrap_or(0.0);
        let beacons = self.grid_effects().expedition_luck;
        (1.10 - 0.15 * danger + 0.02 * self.explored_count() as f64 + prospect + relics + beacons)
            .clamp(0.30, 0.95)
    }

    /// Durée d'une expédition (reliques et prospection comprises).
    pub fn expedition_duration(&self, catalog: &Catalog, system: usize) -> f64 {
        let Some(sys) = self.map.systems.get(system) else {
            return 0.0;
        };
        let prospect = if self.is_explored(system) { 0.6 } else { 1.0 };
        let speed = (1.0
            - self.relic_sum(catalog, |e| match e {
                RelicEffect::ExpeditionSpeed(x) => Some(x),
                _ => None,
            }))
        .max(0.4);
        let set = if self.relic_set_active(catalog, RelicSet::Voyager) {
            0.75
        } else {
            1.0
        };
        sys.duration() * prospect * speed * set * (1.0 - self.grid_effects().expedition_speed)
    }

    /// Une flotte peut partir : vers un système à portée pas encore exploré,
    /// ou en prospection vers un système déjà exploré.
    pub fn can_launch_expedition(&self, catalog: &Catalog, system: usize) -> bool {
        self.starmap_unlocked(catalog)
            && system > 0
            && self.is_revealed(system)
            && !self.fleets.iter().any(|f| f.target == system)
            && self.fleets.len() < self.fleets_max()
    }

    // ------------------------------------------------------------------
    // Événements cosmiques et Convergence
    // ------------------------------------------------------------------

    /// Multiplicateur des bonus temporaires d'un type.
    pub fn boost_factor(&self, kind: BoostKind) -> f64 {
        self.boosts
            .iter()
            .filter(|b| b.kind == kind)
            .map(|b| b.factor)
            .product()
    }

    fn advance_boosts(&mut self, secs: f64) {
        for b in &mut self.boosts {
            b.left -= secs;
        }
        self.boosts.retain(|b| b.left > 0.0);
    }

    /// Les événements arrivent pendant le jeu (pas jeu fermé) à partir de
    /// l'Acte III ; on a une heure pour répondre.
    fn advance_cosmic_events(
        &mut self,
        catalog: &Catalog,
        dt: f64,
        roll: f64,
        events: &mut Vec<DomainEvent>,
    ) {
        if self.map_seed == 0 || catalog.events.is_empty() {
            return;
        }
        if let Some((event, left)) = self.pending_event {
            let left = left - dt;
            if left <= 0.0 {
                self.pending_event = None;
                events.push(DomainEvent::CosmicEventExpired { event });
            } else {
                self.pending_event = Some((event, left));
            }
            return;
        }
        if self.event_timer <= 0.0 {
            // Premier passage : on arme le compte à rebours.
            self.event_timer = next_event_delay(roll);
            return;
        }
        self.event_timer -= dt;
        if self.event_timer <= 0.0 {
            let event = ((roll * 7.31).fract() * catalog.events.len() as f64) as usize;
            let event = event.min(catalog.events.len() - 1);
            self.pending_event = Some((event, EVENT_EXPIRY));
            self.event_timer = next_event_delay((roll * 13.7).fract()).max(1.0);
            events.push(DomainEvent::CosmicEventStarted { event });
        }
    }

    /// Répond à l'événement en attente. `roll` décide d'un choix risqué.
    pub fn resolve_event(
        &mut self,
        catalog: &Catalog,
        choice: usize,
        roll: f64,
        events: &mut Vec<DomainEvent>,
    ) {
        let Some((event, _)) = self.pending_event else {
            return;
        };
        let Some(c) = catalog
            .events
            .get(event)
            .and_then(|e| e.choices.get(choice))
            .copied()
        else {
            return;
        };
        self.pending_event = None;
        let (effect, success) = match c.risk {
            Some((chance, fail)) if roll >= chance => (fail, false),
            _ => (c.effect, true),
        };
        self.apply_event_effect(catalog, effect, roll, events);
        self.events_resolved += 1;
        events.push(DomainEvent::CosmicEventResolved {
            event,
            choice,
            success,
        });
        self.check_achievements(catalog, events);
    }

    fn apply_event_effect(
        &mut self,
        catalog: &Catalog,
        effect: EventEffect,
        roll: f64,
        events: &mut Vec<DomainEvent>,
    ) {
        let relic = |s: &mut Self, danger: u8, events: &mut Vec<DomainEvent>| {
            if catalog.relics.is_empty() {
                return;
            }
            let pick = ((roll * 977.31).fract() * catalog.relics.len() as f64) as usize;
            let rarity = roll_rarity((roll * 131.7).fract(), danger);
            s.gain_relic(
                catalog,
                RelicId(pick.min(catalog.relics.len() - 1)),
                rarity,
                events,
            );
        };
        match effect {
            EventEffect::Nothing => {}
            EventEffect::Production { factor, hours } => self.boosts.push(Boost {
                kind: BoostKind::Production,
                factor,
                left: hours * 3600.0,
            }),
            EventEffect::Fragments { factor, hours } => self.boosts.push(Boost {
                kind: BoostKind::Fragments,
                factor,
                left: hours * 3600.0,
            }),
            EventEffect::Stardust { hours } => {
                let amount = economy::steady_production(catalog, self) * hours * 3600.0;
                self.earn(amount);
            }
            EventEffect::Relic { danger } => relic(self, danger, events),
            EventEffect::FleetHaste(p) => {
                for f in &mut self.fleets {
                    f.left = (f.left - f.total * p).max(1.0);
                }
            }
            EventEffect::Shard => self.shards += 1,
            EventEffect::LoseStardust(p) => {
                let amount = self.stardust.value() * p;
                self.stardust.spend(amount);
            }
            EventEffect::BuyRelic(p) => {
                let amount = self.stardust.value() * p;
                self.stardust.spend(amount);
                relic(self, 5, events);
            }
        }
    }

    /// La Convergence est prête : carte complète et 16 reliques réunies.
    pub fn convergence_ready(&self, catalog: &Catalog) -> bool {
        !self.converged
            && self.convergence.is_none()
            && !self.map.systems.is_empty()
            && self.explored_count() + 1 >= self.map.systems.len()
            && self.relics_owned() >= catalog.relics.len()
    }

    pub fn start_convergence(&mut self, catalog: &Catalog, events: &mut Vec<DomainEvent>) {
        if !self.convergence_ready(catalog) {
            events.push(DomainEvent::PurchaseFailed {
                target: PurchaseTarget::Convergence,
            });
            return;
        }
        self.convergence = Some(CONVERGENCE_SECS);
        events.push(DomainEvent::ConvergenceStarted);
    }

    fn advance_convergence(&mut self, catalog: &Catalog, secs: f64, events: &mut Vec<DomainEvent>) {
        let Some(left) = self.convergence else {
            return;
        };
        let left = left - secs;
        if left > 0.0 {
            self.convergence = Some(left);
            return;
        }
        self.convergence = None;
        self.converged = true;
        self.mark_milestone(4);
        events.push(DomainEvent::MegastructuresOpened);
        self.check_achievements(catalog, events);
        events.push(DomainEvent::ConvergenceDone);
    }

    // ------------------------------------------------------------------
    // Acte IV : Mégastructures
    // ------------------------------------------------------------------

    /// Le Défi du jour s'ouvre avec l'Acte IV.
    pub fn daily_unlocked(&self) -> bool {
        self.converged
    }

    /// Partie vierge pour le Défi du jour `day` : règles du jour, succès tous
    /// acquis (même bonus pour tous les joueurs, pas d'annonces).
    pub fn daily_pocket(catalog: &Catalog, day: u64) -> GameState {
        let mut s = GameState::new(catalog);
        s.rules_override = Some(crate::daily::rules_for(day).1);
        s.achievements.iter_mut().for_each(|a| *a = true);
        s
    }

    /// Enregistre le score d'un défi ; renvoie la matière exotique gagnée
    /// (seulement pour le premier défi du jour).
    pub fn record_daily(&mut self, day: u64, score: f64) -> f64 {
        if self.daily_day != day {
            self.daily_day = day;
            self.daily_best = 0.0;
            self.daily_rewarded = false;
        }
        self.daily_best = self.daily_best.max(score);
        if self.daily_rewarded {
            return 0.0;
        }
        self.daily_rewarded = true;
        let reward = crate::daily::reward_for(score);
        self.exotic += reward;
        self.exotic_total += reward;
        reward
    }

    /// Les Mégastructures s'ouvrent après la Convergence.
    pub fn megastructures_unlocked(&self) -> bool {
        self.converged
    }

    pub fn grid_effects(&self) -> GridEffects {
        megastructure::effects(&self.grid)
    }

    pub fn structures_built(&self) -> usize {
        self.grid.iter().filter(|c| c.is_some()).count()
    }

    pub fn structure_count(&self, kind: StructureKind) -> usize {
        self.grid.iter().filter(|&&c| c == Some(kind)).count()
    }

    pub fn structure_cost(&self, kind: StructureKind) -> f64 {
        kind.cost(self.structure_count(kind))
    }

    pub fn can_place_structure(&self, cell: usize, kind: StructureKind) -> bool {
        self.megastructures_unlocked()
            && self.grid.get(cell).is_some_and(|c| c.is_none())
            && !(kind.unique() && self.structure_count(kind) > 0)
            && self.exotic + 1e-9 >= self.structure_cost(kind)
    }

    pub fn place_structure(
        &mut self,
        catalog: &Catalog,
        cell: usize,
        kind: StructureKind,
        events: &mut Vec<DomainEvent>,
    ) {
        if !self.can_place_structure(cell, kind) {
            events.push(DomainEvent::PurchaseFailed {
                target: PurchaseTarget::Structure(cell),
            });
            return;
        }
        self.exotic -= self.structure_cost(kind);
        self.grid[cell] = Some(kind);
        events.push(DomainEvent::StructurePlaced { cell, kind });
        self.check_achievements(catalog, events);
    }

    /// L'Éveil du Cosmos est prêt : Cœur bâti, les six types présents, et au
    /// moins 30 structures.
    pub fn awakening_ready(&self) -> bool {
        !self.awakened
            && self.awakening.is_none()
            && self.structures_built() >= megastructure::AWAKENING_STRUCTURES
            && StructureKind::ALL
                .iter()
                .all(|&k| self.structure_count(k) > 0)
    }

    pub fn start_awakening(&mut self, events: &mut Vec<DomainEvent>) {
        if !self.awakening_ready() {
            events.push(DomainEvent::PurchaseFailed {
                target: PurchaseTarget::Awakening,
            });
            return;
        }
        self.awakening = Some(megastructure::AWAKENING_SECS);
        events.push(DomainEvent::AwakeningStarted);
    }

    fn advance_awakening(&mut self, catalog: &Catalog, secs: f64, events: &mut Vec<DomainEvent>) {
        let Some(left) = self.awakening else {
            return;
        };
        let left = left - secs;
        if left > 0.0 {
            self.awakening = Some(left);
            return;
        }
        self.awakening = None;
        self.awakened = true;
        self.mark_milestone(5);
        self.check_achievements(catalog, events);
        events.push(DomainEvent::CosmosAwakened);
    }

    /// Démolit une structure (sans remboursement) pour réorganiser la grille.
    pub fn demolish(&mut self, cell: usize, events: &mut Vec<DomainEvent>) {
        if self.awakening.is_some() {
            events.push(DomainEvent::PurchaseFailed {
                target: PurchaseTarget::Structure(cell),
            });
            return;
        }
        if self.grid.get(cell).is_some_and(|c| c.is_some()) {
            self.grid[cell] = None;
            events.push(DomainEvent::StructureRemoved { cell });
        }
    }

    /// La matière exotique s'accumule en temps réel, même jeu fermé.
    fn accrue_exotic(&mut self, secs: f64) {
        if !self.megastructures_unlocked() || secs <= 0.0 {
            return;
        }
        let gained = self.grid_effects().exotic_per_hour * secs / 3600.0;
        self.exotic += gained;
        self.exotic_total += gained;
    }

    // ------------------------------------------------------------------
    // Pilote automatique des flottes, historique, jalons
    // ------------------------------------------------------------------

    /// Le pilote automatique se débloque à 10 systèmes explorés.
    pub fn auto_fleets_unlocked(&self) -> bool {
        self.explored_count() >= AUTO_FLEETS_EXPLORED
    }

    pub fn toggle_auto_fleets(&mut self, events: &mut Vec<DomainEvent>) {
        if !self.auto_fleets_unlocked() {
            events.push(DomainEvent::PurchaseFailed {
                target: PurchaseTarget::Expedition(0),
            });
            return;
        }
        self.auto_fleets = !self.auto_fleets;
        events.push(DomainEvent::AutoFleetsToggled {
            enabled: self.auto_fleets,
        });
    }

    /// Cible du pilote : explorer d'abord (le plus sûr), sinon prospecter le
    /// système exploré le plus dangereux (meilleures reliques).
    pub fn auto_fleet_target(&self, catalog: &Catalog) -> Option<usize> {
        let n = self.map.systems.len();
        let unexplored = (1..n)
            .filter(|&i| !self.is_explored(i) && self.can_launch_expedition(catalog, i))
            .max_by(|&a, &b| {
                self.expedition_chance_in(catalog, a)
                    .partial_cmp(&self.expedition_chance_in(catalog, b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        unexplored.or_else(|| {
            (1..n)
                .filter(|&i| self.is_explored(i) && self.can_launch_expedition(catalog, i))
                .max_by_key(|&i| self.map.systems[i].danger)
        })
    }

    fn run_auto_fleets(&mut self, catalog: &Catalog, roll: f64, events: &mut Vec<DomainEvent>) {
        let mut k = 0.0;
        while self.fleets.len() < self.fleets_max() {
            let Some(target) = self.auto_fleet_target(catalog) else {
                break;
            };
            k += 1.0;
            let r = (roll * 7.77 + k * 0.618_034).fract();
            self.launch_expedition(catalog, target, r, events);
        }
    }

    /// Jeu fermé : les flottes avancent ; en pilote automatique, elles
    /// repartent à chaque retour, comme si le joueur était là.
    fn advance_fleets_offline(
        &mut self,
        catalog: &Catalog,
        secs: f64,
        events: &mut Vec<DomainEvent>,
    ) {
        if !self.auto_fleets {
            self.advance_expeditions(catalog, secs, events);
            return;
        }
        let mut left = secs;
        for step in 0..1000 {
            let seed = (self.map_seed
                ^ (self.expeditions_launched as u64).wrapping_mul(0x9E37_79B9))
                as f64
                / u64::MAX as f64;
            self.run_auto_fleets(catalog, (seed + step as f64 * 0.137).fract(), events);
            let next = self
                .fleets
                .iter()
                .map(|f| f.left)
                .fold(f64::INFINITY, f64::min);
            if !next.is_finite() || next > left {
                break;
            }
            let dt = next.max(1.0).min(left);
            self.advance_expeditions(catalog, dt, events);
            left -= dt;
            if left <= 0.0 {
                break;
            }
        }
        if left > 0.0 {
            self.advance_expeditions(catalog, left, events);
        }
        // Les flottes rentrées en dernier repartent aussitôt.
        let seed = (self.map_seed ^ self.expeditions_launched as u64) as f64 / u64::MAX as f64;
        self.run_auto_fleets(catalog, seed.fract(), events);
    }

    fn record_history(&mut self, catalog: &Catalog) {
        let p = economy::steady_production(catalog, self);
        if !p.is_finite() {
            return;
        }
        self.best_production = self.best_production.max(p);
        self.history
            .push((self.play_time, (p.max(1.0)).log10() as f32));
        if self.history.len() > HISTORY_MAX {
            // On garde tout le début de la partie en ne gardant qu'un point
            // sur deux dans la première moitié.
            let half = self.history.len() / 2;
            let kept: Vec<(f64, f32)> = self.history[..half].iter().step_by(2).copied().collect();
            let rest = self.history.split_off(half);
            self.history = kept;
            self.history.extend(rest);
        }
    }

    fn mark_milestone(&mut self, i: usize) {
        if let Some(m) = self.milestones.get_mut(i) {
            if *m <= 0.0 {
                *m = self.play_time.max(1.0);
            }
        }
    }

    // ------------------------------------------------------------------
    // Reliques
    // ------------------------------------------------------------------

    /// Emplacements : 3, +1 à 10 systèmes explorés, +1 à la carte complète.
    pub fn relic_slots(&self) -> usize {
        let n = self.explored_count();
        let all = self.map.systems.len().saturating_sub(1);
        3 + usize::from(n >= 10) + usize::from(all > 0 && n >= all)
    }

    pub fn relic_rarity(&self, id: RelicId) -> Option<u8> {
        self.relics.get(id.index()).copied().flatten()
    }

    pub fn relics_owned(&self) -> usize {
        self.relics.iter().filter(|r| r.is_some()).count()
    }

    pub fn is_equipped(&self, id: RelicId) -> bool {
        self.equipped.contains(&id)
    }

    /// Effets des reliques équipées (rareté comprise).
    pub fn relic_effects<'a>(
        &'a self,
        catalog: &'a Catalog,
    ) -> impl Iterator<Item = RelicEffect> + 'a {
        self.equipped.iter().filter_map(|&id| {
            let def = catalog.relic(id)?;
            Some(def.effect_at(self.relic_rarity(id)?))
        })
    }

    pub fn relic_sum(&self, catalog: &Catalog, f: impl Fn(RelicEffect) -> Option<f64>) -> f64 {
        self.relic_effects(catalog).filter_map(f).sum()
    }

    /// Bonus d'ensemble : 3 reliques équipées du même ensemble.
    pub fn relic_set_active(&self, catalog: &Catalog, set: RelicSet) -> bool {
        self.equipped
            .iter()
            .filter(|&&id| catalog.relic(id).is_some_and(|d| d.set == set))
            .count()
            >= 3
    }

    /// Multiplicateur de production des reliques (effets, ensembles, Éclats).
    pub fn relic_production(&self, catalog: &Catalog) -> f64 {
        let base = 1.0
            + self.relic_sum(catalog, |e| match e {
                RelicEffect::Production(x) => Some(x),
                _ => None,
            });
        let forge = if self.relic_set_active(catalog, RelicSet::Forge) {
            1.5
        } else {
            1.0
        };
        let origins = if self.relic_set_active(catalog, RelicSet::Origins) {
            2.0
        } else {
            1.0
        };
        base * forge * origins * (1.0 + 0.01 * self.shards as f64)
    }

    /// Équipe ou range une relique.
    pub fn toggle_relic(&mut self, catalog: &Catalog, id: RelicId, events: &mut Vec<DomainEvent>) {
        if self.relic_rarity(id).is_none() {
            events.push(DomainEvent::PurchaseFailed {
                target: PurchaseTarget::Relic(id),
            });
            return;
        }
        if let Some(pos) = self.equipped.iter().position(|&e| e == id) {
            self.equipped.remove(pos);
            events.push(DomainEvent::RelicToggled {
                relic: id,
                equipped: false,
            });
            return;
        }
        if self.equipped.len() >= self.relic_slots() {
            events.push(DomainEvent::PurchaseFailed {
                target: PurchaseTarget::Relic(id),
            });
            return;
        }
        self.equipped.push(id);
        events.push(DomainEvent::RelicToggled {
            relic: id,
            equipped: true,
        });
        self.check_achievements(catalog, events);
    }

    /// Ajoute une relique trouvée à la collection.
    fn gain_relic(
        &mut self,
        catalog: &Catalog,
        id: RelicId,
        rarity: u8,
        events: &mut Vec<DomainEvent>,
    ) {
        let Some(slot) = self.relics.get_mut(id.index()) else {
            return;
        };
        let outcome = match *slot {
            None => {
                *slot = Some(rarity);
                RelicOutcome::New
            }
            Some(r) if rarity > r => {
                *slot = Some(rarity);
                RelicOutcome::Upgraded
            }
            Some(_) => {
                self.shards += 1;
                RelicOutcome::Shard
            }
        };
        events.push(DomainEvent::RelicFound {
            relic: id,
            rarity,
            outcome,
        });
        self.check_achievements(catalog, events);
    }

    /// Envoie une flotte. `roll` décide dès le départ de l'issue (secret).
    pub fn launch_expedition(
        &mut self,
        catalog: &Catalog,
        system: usize,
        roll: f64,
        events: &mut Vec<DomainEvent>,
    ) {
        if !self.can_launch_expedition(catalog, system) {
            events.push(DomainEvent::PurchaseFailed {
                target: PurchaseTarget::Expedition(system),
            });
            return;
        }
        let total = self.expedition_duration(catalog, system);
        let success = roll < self.expedition_chance_in(catalog, system);
        let loot = (roll * 7919.73 + 0.317).fract();
        let prospect = self.is_explored(system);
        self.fleets.push(Expedition {
            target: system,
            left: total,
            total,
            success,
            loot,
            prospect,
        });
        self.expeditions_launched += 1;
        events.push(DomainEvent::ExpeditionLaunched { system });
        self.check_achievements(catalog, events);
    }

    /// Bonus des systèmes explorés d'un type : 1 + `per` × Σ échelle de danger.
    pub fn map_bonus(&self, kind: SystemKind, per: f64) -> f64 {
        1.0 + per
            * self
                .map
                .systems
                .iter()
                .filter(|s| s.kind == kind && self.is_explored(s.index) && s.index > 0)
                .map(|s| s.reward_scale())
                .sum::<f64>()
    }

    /// Ouvre la carte (graine tirée de `roll`) dès que l'Acte III commence.
    fn ensure_map(&mut self, catalog: &Catalog, roll: f64, events: &mut Vec<DomainEvent>) {
        if self.map_seed != 0 || !self.starmap_unlocked(catalog) {
            return;
        }
        self.map_seed = ((roll * (1u64 << 52) as f64) as u64).max(1);
        self.map = starmap::generate(self.map_seed);
        self.explored = vec![false; self.map.systems.len()];
        self.explored[0] = true;
        events.push(DomainEvent::StarMapOpened);
    }

    /// Les flottes avancent en temps réel (même jeu fermé).
    fn advance_expeditions(&mut self, catalog: &Catalog, secs: f64, events: &mut Vec<DomainEvent>) {
        if self.fleets.is_empty() {
            return;
        }
        let mut back = Vec::new();
        for f in &mut self.fleets {
            f.left -= secs;
            if f.left <= 0.0 {
                back.push(*f);
            }
        }
        self.fleets.retain(|f| f.left > 0.0);
        let was_complete = self.explored_count() + 1 >= self.map.systems.len();
        for f in back {
            let mut stardust = 0.0;
            // Butin : une relique (toujours en prospection, une fois sur deux sinon).
            if f.success && (f.prospect || f.loot < 0.5) && !catalog.relics.is_empty() {
                let pick = ((f.loot * 977.31).fract() * catalog.relics.len() as f64) as usize;
                let danger = self.map.systems[f.target].danger;
                let rarity = roll_rarity((f.loot * 131.7).fract(), danger);
                self.gain_relic(
                    catalog,
                    RelicId(pick.min(catalog.relics.len() - 1)),
                    rarity,
                    events,
                );
            }
            if f.success && f.prospect {
                events.push(DomainEvent::ExpeditionReturned {
                    system: f.target,
                    success: true,
                    stardust: 0.0,
                });
                continue;
            }
            if f.success {
                self.explored[f.target] = true;
                let sys = &self.map.systems[f.target];
                if sys.kind == SystemKind::Anomaly {
                    // Des heures de production d'un coup.
                    stardust =
                        economy::steady_production(catalog, self) * 3600.0 * sys.reward_scale();
                    self.earn(stardust);
                }
            }
            events.push(DomainEvent::ExpeditionReturned {
                system: f.target,
                success: f.success,
                stardust,
            });
        }
        if !was_complete && self.explored_count() + 1 >= self.map.systems.len() {
            events.push(DomainEvent::StarMapCompleted);
        }
        self.check_achievements(catalog, events);
    }

    /// Le Grand Attracteur est-il éveillé ? (fin de l'Acte II)
    pub fn attractor_awakened(&self, catalog: &Catalog) -> bool {
        !catalog.attractor.is_empty() && self.attractor >= catalog.attractor.len()
    }

    /// La prochaine étape peut-elle être lancée ?
    pub fn can_build_attractor(&self, catalog: &Catalog) -> bool {
        self.big_bangs > 0
            && self.attractor_build.is_none()
            && catalog
                .attractor_stage(self.attractor)
                .is_some_and(|s| self.particles_spendable() >= s.cost)
    }

    /// Lance la construction de la prochaine étape (particules payées tout de suite).
    pub fn build_attractor(&mut self, catalog: &Catalog, events: &mut Vec<DomainEvent>) {
        if !self.can_build_attractor(catalog) {
            events.push(DomainEvent::PurchaseFailed {
                target: PurchaseTarget::Attractor(self.attractor),
            });
            return;
        }
        let Some(stage) = catalog.attractor_stage(self.attractor) else {
            return;
        };
        self.particles_spent += stage.cost;
        self.attractor_build = Some(stage.build_secs);
        events.push(DomainEvent::AttractorStageStarted {
            stage: self.attractor,
        });
    }

    /// La construction avance en temps réel (même jeu fermé).
    fn advance_attractor(&mut self, catalog: &Catalog, secs: f64, events: &mut Vec<DomainEvent>) {
        let Some(left) = self.attractor_build else {
            return;
        };
        let left = left - secs;
        if left > 0.0 {
            self.attractor_build = Some(left);
            return;
        }
        self.attractor_build = None;
        let stage = self.attractor;
        self.attractor = (self.attractor + 1).min(catalog.attractor.len());
        events.push(DomainEvent::AttractorStageDone { stage });
        self.check_achievements(catalog, events);
        if self.attractor_awakened(catalog) {
            self.mark_milestone(3);
            events.push(DomainEvent::AttractorAwakened);
        }
    }

    pub fn dark_matter_spendable(&self) -> u32 {
        self.crunches.saturating_sub(self.dark_matter_spent)
    }

    pub fn lab_done(&self, id: LabId) -> bool {
        self.lab.get(id.index()).copied().unwrap_or(false)
    }

    pub fn lab_is_paused(&self, id: LabId) -> bool {
        self.lab_paused.get(id.index()).copied().unwrap_or(false)
    }

    /// Effets des recherches terminées et non en pause.
    pub fn lab_effects<'a>(&'a self, catalog: &'a Catalog) -> impl Iterator<Item = LabEffect> + 'a {
        catalog
            .lab
            .iter()
            .filter(|l| self.lab_done(l.id) && !self.lab_is_paused(l.id))
            .map(|l| l.effect)
    }

    fn lab_active(&self, catalog: &Catalog, effect: LabEffect) -> bool {
        self.lab_effects(catalog).any(|e| e == effect)
    }

    /// Recherche terminée (pause ou non) : pour les effets de conservation.
    fn lab_owned_effect(&self, catalog: &Catalog, effect: LabEffect) -> bool {
        catalog
            .lab
            .iter()
            .any(|l| l.effect == effect && self.lab_done(l.id))
    }

    pub fn lab_production(&self, catalog: &Catalog) -> f64 {
        self.lab_effects(catalog)
            .filter_map(|e| match e {
                LabEffect::Production(f) => Some(f),
                _ => None,
            })
            .product()
    }

    pub fn lab_click(&self, catalog: &Catalog) -> f64 {
        self.lab_effects(catalog)
            .filter_map(|e| match e {
                LabEffect::Click(p) => Some(p),
                _ => None,
            })
            .sum()
    }

    /// Une recherche peut-elle commencer ?
    pub fn can_research(&self, catalog: &Catalog, id: LabId) -> bool {
        let Some(def) = catalog.lab_node(id) else {
            return false;
        };
        let prereq_ok = def
            .requires
            .and_then(|k| catalog.lab_by_key(k))
            .is_none_or(|p| self.lab_done(p));
        self.lab_unlocked()
            && self.research.is_none()
            && !self.lab_done(id)
            && prereq_ok
            && self.dark_matter_spendable() >= def.cost
    }

    /// Lance une recherche (une seule à la fois) : la matière noire est payée tout de suite.
    pub fn start_research(&mut self, catalog: &Catalog, id: LabId, events: &mut Vec<DomainEvent>) {
        if !self.can_research(catalog, id) {
            events.push(DomainEvent::PurchaseFailed {
                target: PurchaseTarget::Lab(id),
            });
            return;
        }
        let Some(def) = catalog.lab_node(id) else {
            return;
        };
        self.dark_matter_spent += def.cost;
        self.research = Some((id, def.research_secs));
        events.push(DomainEvent::LabResearchStarted { node: id });
    }

    /// Met en pause / relance un automatisme terminé.
    pub fn toggle_lab(&mut self, catalog: &Catalog, id: LabId, events: &mut Vec<DomainEvent>) {
        let toggleable = catalog
            .lab_node(id)
            .is_some_and(|d| d.effect.is_toggleable());
        if !self.lab_done(id) || !toggleable {
            events.push(DomainEvent::PurchaseFailed {
                target: PurchaseTarget::Lab(id),
            });
            return;
        }
        let paused = !self.lab_is_paused(id);
        self.lab_paused[id.index()] = paused;
        events.push(DomainEvent::LabToggled {
            node: id,
            enabled: !paused,
        });
    }

    fn advance_research(&mut self, catalog: &Catalog, secs: f64, events: &mut Vec<DomainEvent>) {
        let Some((id, left)) = self.research else {
            return;
        };
        let left = left - secs;
        if left > 0.0 {
            self.research = Some((id, left));
            return;
        }
        self.research = None;
        if let Some(slot) = self.lab.get_mut(id.index()) {
            *slot = true;
        }
        events.push(DomainEvent::LabResearchDone { node: id });
        self.check_achievements(catalog, events);
    }

    /// Automatismes du Laboratoire (une fois par seconde).
    fn run_lab_automation(&mut self, catalog: &Catalog, events: &mut Vec<DomainEvent>) {
        if self.lab_active(catalog, LabEffect::AutoConstellations) {
            for _ in 0..20 {
                let next = catalog
                    .constellations
                    .iter()
                    .filter(|c| !c.is_maxed(self.constellation_level(c.id)))
                    .map(|c| (c.id, c.cost_at(self.constellation_level(c.id))))
                    .filter(|&(_, cost)| self.fragments_spendable() + 1e-9 >= cost)
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                match next {
                    Some((id, _)) => self.buy_constellation(catalog, id, events),
                    None => break,
                }
            }
        }
        if self.lab_active(catalog, LabEffect::AutoSupernova) && self.challenge.is_none() {
            let gain = self.fragments_available_with(catalog);
            if gain >= 5.0f64.max(self.fragments) {
                self.supernova(catalog, events);
            }
        }
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
    /// poussière dépensée, par lots du mode d'achat du joueur (×10 : attend
    /// d'avoir de quoi en prendre 10), au plus 10 achats par passage.
    fn run_autobuyer(&mut self, catalog: &Catalog, events: &mut Vec<DomainEvent>) {
        let mode = self.autobuy_mode;
        for _ in 0..AUTOBUY_MAX_PER_TICK {
            let best = catalog
                .generators
                .iter()
                .filter(|g| self.is_visible(catalog, g.id))
                .map(|g| {
                    let cost = self.generator_cost(catalog, g.id, 1);
                    let gain = g.base_prod * economy::generator_multiplier(catalog, self, g.id);
                    (g, gain / cost)
                })
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            let Some((def, _)) = best else { break };
            let owned = self.owned_of(def.id);
            let budget = self.stardust.value() / self.generator_cost_factor(catalog);
            let count = match mode {
                BuyMode::Max => def.max_affordable(owned, budget),
                other => other.quantity(def, owned, budget),
            };
            if count == 0
                || !self
                    .stardust
                    .can_afford(self.generator_cost(catalog, def.id, count))
            {
                break;
            }
            self.buy_generator(catalog, def.id, mode, events);
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
                crunches: self.crunches,
                lab_done: self.lab.iter().filter(|&&d| d).count(),
                universe_keys: self.universe_keys.iter().filter(|&&k| k).count(),
                big_bangs: self.big_bangs,
                forged: self.forged.iter().filter(|&&f| f).count(),
                attractor_stages: self.attractor,
                expeditions: self.expeditions_launched,
                explored: self.explored_count(),
                relics: self.relics_owned(),
                relic_set_active: RelicSet::ALL
                    .iter()
                    .any(|&s| self.relic_set_active(catalog, s)),
                cosmic_events: self.events_resolved,
                converged: self.converged,
                structures: self.structures_built(),
                core_built: self.structure_count(StructureKind::Core) > 0,
                awakened: self.awakened,
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
        // Images/s basses ou clavier lent : les récoltes dues sont
        // rattrapées, 10 par seconde exactement.
        for (dt, per_frame) in [(1.0 / 15.0, 1), (1.0 / 25.0, 1), (1.0 / 60.0, 1), (0.04, 2)] {
            let start = s.clicks;
            for _ in 0..(2.0 / dt) as usize {
                s.tick(&c, dt, 0.5, &mut ev);
                for _ in 0..per_frame {
                    s.harvest_held(&c, &mut ev);
                }
            }
            let got = s.clicks - start;
            assert!((19..=21).contains(&got), "dt {dt} : {got} en 2 s");
        }
        // Touche relâchée une seconde : pas de rattrapage à la reprise.
        s.tick(&c, 1.0, 0.5, &mut ev);
        let before = s.clicks;
        s.harvest_held(&c, &mut ev);
        assert_eq!(s.clicks, before + 1);
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
    fn autobuyer_follows_the_buy_mode() {
        let (c, mut s, mut ev) = setup();
        s.fragments = 5.0;
        s.buy_perk(&c, perk(&c, "perk_auto"), &mut ev);
        assert!(s.autobuy);
        // ×10 : pas assez pour 10 Sondes, l'automate attend.
        s.autobuy_mode = BuyMode::Ten;
        let ten = s.generator_cost(&c, GeneratorId(0), 10);
        s.stardust = Stardust::new(ten * 0.9);
        s.tick(&c, 1.0, 0.5, &mut ev);
        assert_eq!(s.owned.iter().sum::<u64>(), 0, "attend le lot complet");
        // Assez : il en prend 10 d'un coup.
        s.stardust = Stardust::new(ten * 1.05);
        s.tick(&c, 1.0, 0.5, &mut ev);
        assert_eq!(s.owned_of(GeneratorId(0)), 10, "un lot de 10");
        // MAX : tout ce qui est abordable.
        s.autobuy_mode = BuyMode::Max;
        s.stardust = Stardust::new(1e4);
        s.tick(&c, 1.0, 0.5, &mut ev);
        assert!(s.owned.iter().sum::<u64>() > 11, "lot maximal");
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
    fn big_crunch_resets_the_supernova_layer() {
        let (c, mut s, mut ev) = setup();
        s.supernovas = 14;
        s.big_crunch(&c, UniverseId::ORIGIN, &mut ev);
        assert_eq!(
            ev,
            [DomainEvent::BigCrunchUnavailable {
                supernovas: 14,
                needed: 15
            }]
        );
        ev.clear();

        // Un joueur au bout du contenu : 15 Supernovas, Atelier, Constellations, Défis.
        s.supernovas = 15;
        s.fragments = 500.0;
        s.fragments_spent = 100.0;
        s.perks[0] = true;
        s.constellations[0] = 7;
        s.challenges_done[0] = true;
        s.owned[5] = 40;
        s.total_all = 1e20;
        s.clicks = 999;
        s.achievements[0] = true;
        s.golden_caught = 3;
        let hold = c.upgrade_by_key("hold_0").unwrap();
        s.upgrades[hold.index()] = true;

        s.big_crunch(&c, UniverseId::ORIGIN, &mut ev);
        assert!(ev.contains(&DomainEvent::BigCrunch { crunches: 1 }));
        assert_eq!(s.crunches, 1);
        // Remis à zéro.
        assert_eq!(s.supernovas, 0);
        assert_eq!(s.fragments, 0.0);
        assert_eq!(s.fragments_spendable(), 0.0);
        assert!(s.perks.iter().all(|&p| !p));
        assert!(s.constellations.iter().all(|&l| l == 0));
        assert!(s.challenges_done.iter().all(|&d| !d));
        assert!(s.owned.iter().all(|&o| o == 0));
        assert_eq!(s.stardust.value(), 0.0);
        // Conservé.
        assert_eq!(s.total_all, 1e20);
        assert_eq!(s.clicks, 999);
        assert!(s.achievements[0]);
        assert_eq!(s.golden_caught, 3);
        assert!(s.has_upgrade(hold), "Récolte continue reste un confort");
        assert!(s.has_achievement(c.achievement_by_key("crunch1").unwrap()));
        // Les générateurs de prestige redeviennent verrouillés.
        s.total_run = 1e15;
        s.owned[8] = 1;
        assert!(!s.is_visible(&c, GeneratorId(9)));
    }

    #[test]
    fn dark_matter_bonuses() {
        let (c, mut s, mut ev) = setup();
        s.owned[1] = 10;
        let base = economy::total_production(&c, &s);
        s.total_run = 1e9;
        let frags = s.fragments_available_with(&c);
        s.crunches = 3;
        let ratio = economy::total_production(&c, &s) / base;
        assert!((ratio - 8.0).abs() < 1e-9, "×2³ : {ratio}");
        // floor(√100 × 1.75) = 17 au lieu de 10.
        assert_eq!(frags, 10.0);
        assert_eq!(s.fragments_available_with(&c), 17.0);
        // Pas de Big Crunch pendant un Défi.
        s.supernovas = 20;
        s.challenge = Some(c.challenge_by_key("ch_noclick").unwrap());
        s.big_crunch(&c, UniverseId::ORIGIN, &mut ev);
        assert!(matches!(ev[0], DomainEvent::BigCrunchUnavailable { .. }));
    }

    fn lab(c: &Catalog, key: &str) -> LabId {
        c.lab_by_key(key).unwrap()
    }

    /// Termine immédiatement la recherche en cours (en faisant passer le temps).
    fn finish_research(c: &Catalog, s: &mut GameState, ev: &mut Vec<DomainEvent>) {
        s.tick(c, 100.0 * 3600.0, 0.5, ev);
    }

    #[test]
    fn laboratory_research_takes_real_time() {
        let (c, mut s, mut ev) = setup();
        let prod1 = lab(&c, "lab_prod1");
        s.crunches = 9;
        s.start_research(&c, prod1, &mut ev);
        assert!(
            matches!(ev[0], DomainEvent::PurchaseFailed { .. }),
            "fermé avant 10 Big Crunch"
        );
        ev.clear();
        s.crunches = 10;
        s.owned[1] = 10;
        let before = economy::total_production(&c, &s);
        s.start_research(&c, prod1, &mut ev);
        assert_eq!(ev, [DomainEvent::LabResearchStarted { node: prod1 }]);
        assert_eq!(s.dark_matter_spendable(), 9);
        // Une seule recherche à la fois.
        ev.clear();
        s.start_research(&c, lab(&c, "lab_click"), &mut ev);
        assert!(matches!(ev[0], DomainEvent::PurchaseFailed { .. }));
        // 3 h de recherche : pas finie après 2 h 59…
        ev.clear();
        s.tick(&c, 3.0 * 3600.0 - 1.0, 0.5, &mut ev);
        assert!(!s.lab_done(prod1));
        // … finie juste après, et la production double.
        s.tick(&c, 2.0, 0.5, &mut ev);
        assert!(ev.contains(&DomainEvent::LabResearchDone { node: prod1 }));
        let ratio = economy::total_production(&c, &s) / before;
        assert!(ratio >= 2.0, "{ratio}");
        // Densité II exige Densité I : désormais possible.
        assert!(s.can_research(&c, lab(&c, "lab_prod2")));
    }

    #[test]
    fn research_advances_while_offline() {
        let (c, mut s, mut ev) = setup();
        s.crunches = 10;
        s.owned[1] = 1;
        let net = lab(&c, "lab_comet_net");
        s.start_research(&c, net, &mut ev);
        s.apply_offline(&c, 2.0 * 3600.0, &mut ev);
        assert!(s.lab_done(net), "1 h de recherche, 2 h d'absence");
    }

    #[test]
    fn laboratory_preserves_things_across_big_crunch() {
        let (c, mut s, mut ev) = setup();
        s.crunches = 20;
        for key in ["lab_keep_atelier", "lab_keep_challenges", "lab_head_start"] {
            s.start_research(&c, lab(&c, key), &mut ev);
            finish_research(&c, &mut s, &mut ev);
        }
        s.perks[0] = true;
        s.challenges_done[0] = true;
        s.constellations[0] = 5;
        s.supernovas = 15;
        s.big_crunch(&c, UniverseId::ORIGIN, &mut ev);
        assert!(s.perks[0], "Mémoire de l'Atelier");
        assert!(s.challenges_done[0], "Défis gravés");
        assert_eq!(
            s.constellations[0], 0,
            "les Constellations repartent de zéro"
        );
        assert_eq!(s.supernovas, 3, "Départ lancé");
        assert_eq!(s.dark_matter_spendable(), 21 - 7);
    }

    #[test]
    fn laboratory_automation() {
        let (c, mut s, mut ev) = setup();
        s.crunches = 30;
        for key in ["lab_autonova", "lab_autoconst", "lab_comet_net"] {
            s.start_research(&c, lab(&c, key), &mut ev);
            finish_research(&c, &mut s, &mut ev);
        }
        // Constellations automatiques : les fragments libres sont dépensés.
        s.fragments = 10.0;
        ev.clear();
        s.tick(&c, 1.0, 0.5, &mut ev);
        assert!(s.constellation_levels_total() > 0);
        assert!(s.fragments_spendable() < 2.0);
        // Supernova automatique dès qu'elle rapporte au moins autant qu'on a (≥ 5).
        s.total_run = 1e10;
        let novas = s.supernovas;
        s.tick(&c, 1.0, 0.5, &mut ev);
        assert_eq!(s.supernovas, novas + 1);
        // Pause : plus de Supernova automatique.
        let auto = lab(&c, "lab_autonova");
        s.toggle_lab(&c, auto, &mut ev);
        assert!(ev.contains(&DomainEvent::LabToggled {
            node: auto,
            enabled: false
        }));
        s.total_run = 1e12;
        let novas = s.supernovas;
        s.tick(&c, 1.0, 0.5, &mut ev);
        assert_eq!(s.supernovas, novas);
        // Filet à comètes : une comète normale est capturée toute seule.
        let caught = s.meteors_caught;
        s.comet.next_in = 0.5;
        s.tick(&c, 1.0, 0.5, &mut ev); // apparition
        s.tick(&c, 2.5, 0.5, &mut ev); // 2 s de passage → capture
        assert_eq!(s.meteors_caught, caught + 1);
    }

    fn universe(c: &Catalog, key: &str) -> UniverseId {
        c.universe_by_key(key).unwrap()
    }

    #[test]
    fn universes_open_at_10_crunches() {
        let (c, mut s, mut ev) = setup();
        let gravity = universe(&c, "gravity");
        s.supernovas = 15;
        s.big_crunch(&c, gravity, &mut ev);
        assert_eq!(s.universe, UniverseId::ORIGIN, "fermés avant 10 Big Crunch");
        s.crunches = 10;
        s.supernovas = 15;
        ev.clear();
        s.big_crunch(&c, gravity, &mut ev);
        assert_eq!(s.universe, gravity);
        assert!(ev.contains(&DomainEvent::UniverseEntered { universe: gravity }));
    }

    #[test]
    fn universe_rules_apply() {
        let (c, mut s, mut ev) = setup();
        s.owned[1] = 10;
        let base_prod = economy::total_production(&c, &s);
        let base_cost = s.generator_cost(&c, GeneratorId(0), 1);
        // Gravité forte : coûts ×2, production ×4.
        s.universe = universe(&c, "gravity");
        assert!((s.generator_cost(&c, GeneratorId(0), 1) - base_cost * 2.0).abs() < 1e-9);
        assert!((economy::total_production(&c, &s) / base_prod - 4.0).abs() < 1e-9);
        // Désert : améliorations interdites, production ×10.
        s.universe = universe(&c, "desert");
        s.stardust = Stardust::new(1e6);
        s.buy_upgrade(&c, c.upgrade_by_key("gen1_t0").unwrap(), &mut ev);
        assert!(matches!(
            ev.last(),
            Some(DomainEvent::ChallengeBlocked { .. })
        ));
        assert!((economy::total_production(&c, &s) / base_prod - 10.0).abs() < 1e-9);
        // Entropie : −1 %/min, plancher à 10 %.
        s.universe = universe(&c, "entropy");
        s.run_started = s.play_time;
        assert_eq!(s.entropy_factor(&c), 1.0);
        s.play_time += 30.0 * 60.0;
        assert!((s.entropy_factor(&c) - 0.7).abs() < 1e-9);
        s.play_time += 1000.0 * 60.0;
        assert_eq!(s.entropy_factor(&c), 0.1);
        // Temps dilaté : hors-ligne ×4.
        s.universe = universe(&c, "time");
        assert_eq!(s.offline_terms(&c).0, OFFLINE_RATE * 4.0);
        // Nuée : comètes 5× plus fréquentes.
        s.universe = universe(&c, "swarm");
        s.tick(&c, 0.1, 0.0, &mut ev);
        assert!((s.comet.interval_factor - 0.2).abs() < 1e-9);
    }

    #[test]
    fn universe_key_is_earned_and_kept() {
        let (c, mut s, mut ev) = setup();
        let desert = universe(&c, "desert");
        s.crunches = 10;
        s.supernovas = 15;
        s.big_crunch(&c, desert, &mut ev);
        s.owned[1] = 10;
        let prod_in_desert = economy::total_production(&c, &s);
        // 25 Supernovas dans le Désert → Clé des sables.
        s.supernovas = 24;
        s.total_run = 1e14;
        ev.clear();
        s.supernova(&c, &mut ev);
        assert!(ev.contains(&DomainEvent::UniverseKeyEarned { universe: desert }));
        assert!(s.has_universe_key(desert));
        assert!(s.has_achievement(c.achievement_by_key("key1").unwrap()));
        // Retour à l'origine : la Clé reste, production ×2 partout.
        s.supernovas = 15;
        s.big_crunch(&c, UniverseId::ORIGIN, &mut ev);
        s.owned[1] = 10;
        let prod_origin = economy::total_production(&c, &s);
        assert!(
            prod_origin > 0.0 && prod_in_desert > prod_origin,
            "le Désert ×10 > la Clé ×2"
        );
        s.universe_keys[desert.index()] = false;
        let without_key = economy::total_production(&c, &s);
        assert!((prod_origin / without_key - 2.0).abs() < 1e-9);
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
        assert_eq!(s.owned.len(), 15);
        assert_eq!(s.total_run, 0.0);
    }

    fn gen(c: &Catalog, i: usize) -> GeneratorId {
        c.generators[i].id
    }

    #[test]
    fn big_bang_needs_15_crunches_and_resets_the_crunch_layer() {
        let (c, mut s, mut ev) = setup();
        s.crunches = 14;
        s.big_bang(&c, &mut ev);
        assert!(matches!(
            ev.last(),
            Some(DomainEvent::BigBangUnavailable { crunches: 14, .. })
        ));
        // Une particule par tranche de 5 Big Crunch sacrifiés.
        for (crunches, particles) in [(15, 3), (19, 3), (20, 4), (30, 6)] {
            s.crunches = crunches;
            assert_eq!(s.particles_available(), particles, "{crunches} Big Crunch");
        }
        s.crunches = 17;
        s.dark_matter_spent = 20;
        s.supernovas = 15;
        s.fragments = 1e4;
        s.lab[0] = true;
        s.universe_keys[1] = true;
        s.universe = UniverseId(2);
        let prod_before = economy::permanent_multiplier(&c, &s);
        ev.clear();
        s.big_bang(&c, &mut ev);
        assert!(ev.iter().any(|e| matches!(
            e,
            DomainEvent::BigBang {
                particles_gained: 3,
                total_particles: 3
            }
        )));
        assert_eq!(s.crunches, 0);
        assert_eq!(s.dark_matter_spent, 0);
        assert_eq!(s.supernovas, 0);
        assert_eq!(s.fragments, 0.0);
        assert_eq!(s.universe, UniverseId::ORIGIN);
        assert!(s.lab[0], "le Laboratoire est gardé");
        assert!(s.universe_keys[1], "les Clés sont gardées");
        assert!(s.lab_unlocked() && s.universes_unlocked());
        assert_eq!(s.particle_production(), 1000.0);
        assert_eq!(s.particle_fragments(), 1.75);
        assert!(economy::permanent_multiplier(&c, &s) < prod_before);
        assert!(s.has_achievement(c.achievement_by_key("bang1").unwrap()));
    }

    #[test]
    fn primordial_generators_are_forged_in_order_with_particles() {
        let (c, mut s, mut ev) = setup();
        let neutron = gen(&c, 12);
        let reality = gen(&c, 13);
        s.total_run = 1e20;
        assert!(!s.is_visible(&c, neutron), "pas encore forgé");
        s.particles = 3;
        assert!(!s.can_forge(&c, reality), "dans l'ordre");
        s.forge(&c, neutron, &mut ev);
        assert!(s.is_forged(neutron));
        assert!(s.is_visible(&c, neutron));
        assert_eq!(s.particles_spendable(), 2);
        assert_eq!(s.particle_production(), 1000.0, "le bonus reste entier");
        s.forge(&c, reality, &mut ev);
        assert_eq!(s.particles_spendable(), 0);
        s.forge(&c, gen(&c, 14), &mut ev);
        assert!(matches!(
            ev.last(),
            Some(DomainEvent::PurchaseFailed {
                target: PurchaseTarget::Forge(_)
            })
        ));
        assert!(!s.can_forge(&c, gen(&c, 3)), "les autres ne se forgent pas");
    }

    #[test]
    fn attractor_is_built_in_real_time_and_awakens() {
        let (c, mut s, mut ev) = setup();
        s.particles = 30;
        s.build_attractor(&c, &mut ev);
        assert!(s.attractor_build.is_none(), "il faut un Big Bang");
        s.big_bangs = 1;
        s.build_attractor(&c, &mut ev);
        assert_eq!(s.attractor_build, Some(3.0 * 3600.0));
        assert_eq!(s.particles_spendable(), 28);
        s.build_attractor(&c, &mut ev);
        assert_eq!(s.particles_spendable(), 28, "une étape à la fois");
        // Jeu fermé 4 h : l'étape est achevée.
        ev.clear();
        s.apply_offline(&c, 4.0 * 3600.0, &mut ev);
        assert_eq!(s.attractor, 1);
        assert!(ev
            .iter()
            .any(|e| matches!(e, DomainEvent::AttractorStageDone { stage: 0 })));
        assert_eq!(s.attractor_production(&c), 2.0);
        // Toutes les étapes, jusqu'à l'éveil.
        let mut awakened = false;
        for _ in 1..c.attractor.len() {
            s.build_attractor(&c, &mut ev);
            ev.clear();
            s.tick(&c, 40.0 * 3600.0, 0.5, &mut ev);
            awakened |= ev.contains(&DomainEvent::AttractorAwakened);
        }
        assert!(awakened && s.attractor_awakened(&c));
        assert_eq!(s.attractor_production(&c), 2.0 * 3.0 * 5.0 * 10.0);
        assert_eq!(s.crunch_production(&c), 1.0, "aucun Crunch : ×1");
        s.crunches = 3;
        assert_eq!(s.crunch_production(&c), 27.0, "matière noire ×2 : 3³");
        // L'aperçu du prochain Big Crunch compte aussi le Cœur de superamas.
        assert_eq!(s.crunch_production_with(&c, s.crunches + 1), 81.0);
        assert_eq!(s.offline_terms(&c).0, 1.0);
        assert!(s.has_achievement(c.achievement_by_key("attractor").unwrap()));
    }

    /// Partie à l'éveil du Grand Attracteur, carte ouverte.
    fn act_three(c: &Catalog) -> (GameState, Vec<DomainEvent>) {
        let mut s = GameState::new(c);
        let mut ev = Vec::new();
        s.big_bangs = 1;
        s.attractor = c.attractor.len();
        s.tick(c, 0.1, 0.42, &mut ev);
        (s, ev)
    }

    #[test]
    fn star_map_opens_at_the_end_of_act_two() {
        let c = Catalog::standard();
        let mut s = GameState::new(&c);
        let mut ev = Vec::new();
        s.tick(&c, 0.1, 0.42, &mut ev);
        assert_eq!(s.map_seed, 0, "pas avant l'éveil");
        let (s, ev) = act_three(&c);
        assert!(ev.contains(&DomainEvent::StarMapOpened));
        assert_eq!(s.map.systems.len(), 25);
        assert!(s.is_explored(0));
        assert!(s.is_revealed(1) && !s.is_explored(1));
        let far = s.map.systems.len() - 1;
        assert!(!s.is_revealed(far), "brouillard au loin");
        // La carte se reconstruit à l'identique depuis sa graine.
        let mut copy = s.clone();
        copy.map = StarMap {
            systems: Vec::new(),
            columns: 0,
        };
        copy.normalize(&c);
        assert_eq!(copy.map, s.map);
    }

    #[test]
    fn expeditions_take_real_time_and_reward_success() {
        let c = Catalog::standard();
        let (mut s, mut ev) = act_three(&c);
        let target = 1;
        assert_eq!(s.fleets_max(), 2);
        s.launch_expedition(&c, target, 0.0, &mut ev);
        assert_eq!(s.fleets.len(), 1);
        s.launch_expedition(&c, target, 0.0, &mut ev);
        assert_eq!(s.fleets.len(), 1, "une flotte par système");
        let secs = s.fleets[0].total;
        ev.clear();
        s.apply_offline(&c, secs + 1.0, &mut ev);
        assert!(ev.iter().any(|e| matches!(
            e,
            DomainEvent::ExpeditionReturned {
                system: 1,
                success: true,
                ..
            }
        )));
        assert!(s.is_explored(target));
        assert!(s.has_achievement(c.achievement_by_key("exp1").unwrap()));
        // Une expédition ratée n'explore rien.
        let next = s.map.systems[0].links[1];
        s.launch_expedition(&c, next, 0.999, &mut ev);
        let secs = s.fleets[0].total;
        ev.clear();
        s.tick(&c, secs + 1.0, 0.5, &mut ev);
        assert!(ev
            .iter()
            .any(|e| matches!(e, DomainEvent::ExpeditionReturned { success: false, .. })));
        assert!(!s.is_explored(next));
    }

    #[test]
    fn explored_systems_give_permanent_bonuses() {
        let c = Catalog::standard();
        let (mut s, _) = act_three(&c);
        let prod = economy::permanent_multiplier(&c, &s);
        let nebula = s
            .map
            .systems
            .iter()
            .position(|x| x.kind == SystemKind::Nebula && x.index > 0)
            .unwrap();
        s.explored[nebula] = true;
        let scale = s.map.systems[nebula].reward_scale();
        let ratio = economy::permanent_multiplier(&c, &s) / prod;
        assert!((ratio - (1.0 + 0.10 * scale)).abs() < 1e-9);
        // Chaque Civilisation explorée ajoute une flotte.
        let civ = s
            .map
            .systems
            .iter()
            .position(|x| x.kind == SystemKind::Civilization)
            .unwrap();
        s.explored[civ] = true;
        assert_eq!(s.fleets_max(), 3);
        // Le danger baisse la chance de succès.
        let easy = s.expedition_chance(1);
        let hard = s.expedition_chance(s.map.systems.len() - 1);
        assert!(easy > hard && hard >= 0.25);
    }

    #[test]
    fn relics_are_found_equipped_and_combined_into_sets() {
        let c = Catalog::standard();
        let (mut s, mut ev) = act_three(&c);
        // Butin d'une expédition réussie.
        s.gain_relic(&c, RelicId(0), 1, &mut ev);
        assert!(ev.iter().any(|e| matches!(
            e,
            DomainEvent::RelicFound {
                outcome: RelicOutcome::New,
                ..
            }
        )));
        s.gain_relic(&c, RelicId(0), 0, &mut ev);
        assert_eq!(s.shards, 1, "doublon → Éclat");
        s.gain_relic(&c, RelicId(0), 3, &mut ev);
        assert_eq!(
            s.relic_rarity(RelicId(0)),
            Some(3),
            "améliorée en légendaire"
        );
        // Équiper : Marteau stellaire légendaire = +80 % de production.
        let before = economy::permanent_multiplier(&c, &s);
        s.toggle_relic(&c, RelicId(0), &mut ev);
        let ratio = economy::permanent_multiplier(&c, &s) / before;
        assert!((ratio - 1.8).abs() < 1e-9, "{ratio}");
        // Ensemble du Forgeron (3 reliques) : production ×1,5 en plus.
        for i in 1..=2 {
            s.gain_relic(&c, RelicId(i), 0, &mut ev);
            s.toggle_relic(&c, RelicId(i), &mut ev);
        }
        assert!(s.relic_set_active(&c, RelicSet::Forge));
        assert!(s.has_achievement(c.achievement_by_key("relicset").unwrap()));
        // 3 emplacements au départ.
        s.gain_relic(&c, RelicId(5), 0, &mut ev);
        s.toggle_relic(&c, RelicId(5), &mut ev);
        assert_eq!(s.equipped.len(), 3);
        // Ranger libère un emplacement.
        s.toggle_relic(&c, RelicId(0), &mut ev);
        assert_eq!(s.equipped.len(), 2);
    }

    #[test]
    fn prospecting_an_explored_system_brings_back_a_relic() {
        let c = Catalog::standard();
        let (mut s, mut ev) = act_three(&c);
        s.explored[1] = true;
        assert!(s.can_launch_expedition(&c, 1), "prospection possible");
        assert!(s.expedition_duration(&c, 1) < s.map.systems[1].duration());
        s.launch_expedition(&c, 1, 0.01, &mut ev);
        let secs = s.fleets[0].total;
        ev.clear();
        s.tick(&c, secs + 1.0, 0.5, &mut ev);
        assert!(ev
            .iter()
            .any(|e| matches!(e, DomainEvent::RelicFound { .. })));
        assert_eq!(s.relics_owned(), 1);
    }

    #[test]
    fn cosmic_events_arrive_and_resolve_with_risks() {
        let c = Catalog::standard();
        let (mut s, mut ev) = act_three(&c);
        // Le compte à rebours s'arme, puis un événement arrive (2 à 4 h).
        for _ in 0..(5 * 60) {
            s.tick(&c, 60.0, 0.5, &mut ev);
            if s.pending_event.is_some() {
                break;
            }
        }
        assert!(ev
            .iter()
            .any(|e| matches!(e, DomainEvent::CosmicEventStarted { .. })));
        // Tempête de plasma : récolter l'orage, réussi puis raté.
        let storm = c
            .events
            .iter()
            .position(|e| e.key == "plasma_storm")
            .unwrap();
        s.pending_event = Some((storm, 600.0));
        s.resolve_event(&c, 1, 0.1, &mut ev);
        assert_eq!(s.boost_factor(BoostKind::Production), 3.0);
        s.pending_event = Some((storm, 600.0));
        s.resolve_event(&c, 1, 0.9, &mut ev);
        assert_eq!(s.boost_factor(BoostKind::Production), 1.5, "×3 puis ×0,5");
        assert_eq!(s.events_resolved, 2);
        assert!(s.has_achievement(c.achievement_by_key("event1").unwrap()));
        // Les bonus s'éteignent avec le temps (même jeu fermé).
        s.apply_offline(&c, 2.0 * 3600.0, &mut ev);
        assert_eq!(s.boost_factor(BoostKind::Production), 1.0);
        // Sans réponse dans l'heure, l'événement passe.
        s.pending_event = Some((0, 10.0));
        ev.clear();
        s.tick(&c, 11.0, 0.5, &mut ev);
        assert!(ev
            .iter()
            .any(|e| matches!(e, DomainEvent::CosmicEventExpired { .. })));
    }

    #[test]
    fn convergence_needs_the_full_map_and_all_relics() {
        let c = Catalog::standard();
        let (mut s, mut ev) = act_three(&c);
        assert!(!s.convergence_ready(&c));
        s.explored.iter_mut().for_each(|e| *e = true);
        assert!(!s.convergence_ready(&c), "il manque les reliques");
        s.relics.iter_mut().for_each(|r| *r = Some(0));
        assert!(s.convergence_ready(&c));
        let before = economy::permanent_multiplier(&c, &s);
        s.start_convergence(&c, &mut ev);
        assert!(s.convergence.is_some());
        ev.clear();
        s.apply_offline(&c, 48.0 * 3600.0 + 1.0, &mut ev);
        assert!(ev.contains(&DomainEvent::ConvergenceDone));
        assert!(s.converged);
        let after = economy::permanent_multiplier(&c, &s);
        // ×10, plus le +1 % de chaque succès débloqué au passage (carte, reliques…).
        assert!(
            (10.0..12.0).contains(&(after / before)),
            "{}",
            after / before
        );
        assert!(s.has_achievement(c.achievement_by_key("converge").unwrap()));
    }

    #[test]
    fn auto_fleets_relaunch_even_while_away() {
        let c = Catalog::standard();
        let (mut s, mut ev) = act_three(&c);
        s.toggle_auto_fleets(&mut ev);
        assert!(!s.auto_fleets, "il faut 10 systèmes explorés");
        s.explored
            .iter_mut()
            .skip(1)
            .take(10)
            .for_each(|e| *e = true);
        s.toggle_auto_fleets(&mut ev);
        assert!(s.auto_fleets);
        // Douze heures jeu fermé : les flottes enchaînent plusieurs voyages.
        let before = s.expeditions_launched;
        ev.clear();
        s.apply_offline(&c, 12.0 * 3600.0, &mut ev);
        let returned = ev
            .iter()
            .filter(|e| matches!(e, DomainEvent::ExpeditionReturned { .. }))
            .count();
        assert!(
            s.expeditions_launched >= before + 4,
            "{}",
            s.expeditions_launched - before
        );
        assert!(returned >= 2, "{returned}");
        assert_eq!(s.fleets.len(), s.fleets_max(), "elles sont reparties");
    }

    #[test]
    fn history_and_milestones_are_recorded() {
        let (c, mut s, mut ev) = setup();
        s.owned[0] = 10;
        for _ in 0..(40 * 60) {
            s.tick(&c, 1.0, 0.5, &mut ev);
        }
        assert!(s.history.len() >= 2, "{}", s.history.len());
        assert!(s.best_production > 0.0);
        s.total_run = 1e12;
        s.supernova(&c, &mut ev);
        assert!(s.milestones[0] > 0.0);
        let first = s.milestones[0];
        s.total_run = 1e14;
        s.supernova(&c, &mut ev);
        assert_eq!(s.milestones[0], first, "la première seulement");
    }

    #[test]
    fn megastructures_are_built_with_exotic_matter_after_the_convergence() {
        let (c, mut s, mut ev) = setup();
        s.normalize(&c);
        s.place_structure(&c, 0, StructureKind::Collector, &mut ev);
        assert_eq!(s.structures_built(), 0, "pas avant la Convergence");
        s.converged = true;
        // Une heure, même jeu fermé : 1 matière exotique.
        s.apply_offline(&c, 3600.0, &mut ev);
        assert!((s.exotic - 1.0).abs() < 1e-9);
        s.exotic = 100.0;
        s.place_structure(&c, 10, StructureKind::Collector, &mut ev);
        assert_eq!(s.exotic, 97.0);
        assert_eq!(s.structure_cost(StructureKind::Collector), 4.5);
        s.place_structure(&c, 10, StructureKind::Relay, &mut ev);
        assert!(
            matches!(ev.last(), Some(DomainEvent::PurchaseFailed { .. })),
            "case occupée"
        );
        s.place_structure(&c, 11, StructureKind::WorldForge, &mut ev);
        let prod = economy::permanent_multiplier(&c, &s);
        s.place_structure(&c, 20, StructureKind::Core, &mut ev);
        s.place_structure(&c, 30, StructureKind::Core, &mut ev);
        assert_eq!(s.structure_count(StructureKind::Core), 1, "unique");
        assert!(economy::permanent_multiplier(&c, &s) > prod);
        assert!(s.has_achievement(c.achievement_by_key("core").unwrap()));
        s.demolish(11, &mut ev);
        assert_eq!(s.grid[11], None);
    }

    #[test]
    fn daily_pocket_universe_and_reward_once_a_day() {
        let c = Catalog::standard();
        let pocket = GameState::daily_pocket(&c, 20_000);
        assert_eq!(pocket.rules(&c), crate::daily::rules_for(20_000).1);
        assert!(pocket.achievements.iter().all(|&a| a));
        let mut main = GameState::new(&c);
        main.converged = true;
        let r = main.record_daily(20_000, 1e10);
        assert_eq!(r, 6.0);
        assert_eq!(main.exotic, 6.0);
        assert_eq!(main.record_daily(20_000, 1e12), 0.0, "une fois par jour");
        assert_eq!(main.daily_best, 1e12);
        assert_eq!(main.record_daily(20_001, 1e10), 6.0, "le lendemain, oui");
    }

    #[test]
    fn cosmos_awakening_ends_act_four() {
        let (c, mut s, mut ev) = setup();
        s.normalize(&c);
        s.converged = true;
        assert!(!s.awakening_ready());
        for (i, cell) in s.grid.iter_mut().enumerate().take(30) {
            *cell = StructureKind::from_index(i % 5);
        }
        assert!(!s.awakening_ready(), "il manque le Cœur");
        s.grid[40] = Some(StructureKind::Core);
        assert!(s.awakening_ready());
        let before = economy::permanent_multiplier(&c, &s);
        s.start_awakening(&mut ev);
        s.demolish(0, &mut ev);
        assert!(s.grid[0].is_some(), "on ne démolit pas pendant l'Éveil");
        ev.clear();
        s.apply_offline(&c, 72.0 * 3600.0 + 1.0, &mut ev);
        assert!(ev.contains(&DomainEvent::CosmosAwakened));
        assert!(s.awakened && s.milestones[5] > 0.0);
        let ratio = economy::permanent_multiplier(&c, &s) / before;
        assert!((100.0..110.0).contains(&ratio), "{ratio}");
        assert!(s.has_achievement(c.achievement_by_key("awaken").unwrap()));
    }
}
