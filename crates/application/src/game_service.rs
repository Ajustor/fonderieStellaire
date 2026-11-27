//! `GameService` : le cas d'usage principal. Reçoit des [`Command`], fait avancer
//! le temps, sauvegarde, et renvoie des événements à la présentation.

use std::collections::VecDeque;

use domain::{economy, BuyMode, Catalog, DomainEvent, GameState, Requirement};

use crate::commands::Command;
use crate::error::AppError;
use crate::ports::{Clock, LoadOutcome, RandomSource, SaveRepository};
use crate::save_dto::{SaveData, SAVE_RESET_BELOW};
use crate::snapshot::{
    ActiveChallengeView, ChallengeView, ConstellationView, DailyView, FleetView, ForgeView,
    GameSnapshot, GeneratorView, LabView, NextUpgradeView, PerkView, ProgressView, RelicView,
    ResearchBuildView, ResearchView, SystemView, TeaserView, UniverseView, UpgradeView,
};

/// Nombre d'échantillons de production conservés (1 par seconde).
pub const HISTORY_LEN: usize = 120;
/// Intervalle de sauvegarde automatique.
pub const AUTOSAVE_INTERVAL: f64 = 30.0;
const SAMPLE_INTERVAL: f64 = 1.0;
/// En dessous de ce délai, pas de gain hors-ligne.
const MIN_OFFLINE_SECS: u64 = 10;

/// Événements renvoyés à la présentation : ceux du domaine, plus ceux propres
/// à l'application (sauvegarde, préférences).
#[derive(Clone, Debug, PartialEq)]
pub enum GameEvent {
    Domain(DomainEvent),
    BuyModeChanged(BuyMode),
    ReducedEffectsChanged(bool),
    Saved {
        automatic: bool,
    },
    SaveFailed(String),
    /// La sauvegarde était corrompue : copiée dans `backup`, nouvelle partie.
    SaveRecovered {
        backup: String,
    },
    /// La sauvegarde datait d'un ancien équilibrage : effacée, nouvelle partie.
    SaveWiped,
    /// La sauvegarde avait été modifiée à la main : refusée, nouvelle partie.
    SaveTampered {
        backup: String,
    },
    /// Le joueur a effacé sa partie pour recommencer.
    NewGame,
    FrameRateChanged(u32),
    /// 0 : normales, 1 : daltonien, 2 : contraste élevé.
    ColorModeChanged(u8),
    /// Le Défi du jour commence.
    DailyStarted {
        day: u64,
    },
    /// Fin du Défi du jour : score, record du jour, matière exotique gagnée,
    /// code de score à partager.
    DailyEnded {
        score: f64,
        best: f64,
        reward: f64,
        code: String,
    },
    DailyUnavailable,
}

/// Défi du jour en cours : la partie principale attend de côté.
struct DailyRun {
    main: Box<GameState>,
    day: u64,
    left: f64,
}

pub struct GameService {
    catalog: Catalog,
    state: GameState,
    buy_mode: BuyMode,
    reduced_effects: bool,
    /// Images par seconde choisies (0 : par défaut, 30).
    frame_rate: u32,
    /// Couleurs : 0 normales, 1 daltonien, 2 contraste élevé.
    color_mode: u8,
    /// Début de la partie (`SaveData::started_at`).
    started_at: u64,
    repo: Box<dyn SaveRepository>,
    clock: Box<dyn Clock>,
    rng: Box<dyn RandomSource>,
    history: VecDeque<f64>,
    /// Production de chaque générateur, échantillon par échantillon.
    history_mix: VecDeque<Vec<f64>>,
    sample_timer: f64,
    autosave_timer: f64,
    scratch: Vec<DomainEvent>,
    daily: Option<DailyRun>,
}

impl GameService {
    pub fn new(
        repo: Box<dyn SaveRepository>,
        clock: Box<dyn Clock>,
        rng: Box<dyn RandomSource>,
    ) -> Self {
        let catalog = Catalog::standard();
        let state = GameState::new(&catalog);
        GameService {
            catalog,
            state,
            buy_mode: BuyMode::One,
            reduced_effects: false,
            frame_rate: 0,
            color_mode: 0,
            started_at: 0,
            repo,
            clock,
            rng,
            history: VecDeque::with_capacity(HISTORY_LEN),
            history_mix: VecDeque::with_capacity(HISTORY_LEN),
            sample_timer: 0.0,
            autosave_timer: 0.0,
            scratch: Vec::new(),
            daily: None,
        }
    }

    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    pub fn state(&self) -> &GameState {
        &self.state
    }

    pub fn save_location(&self) -> String {
        self.repo.location()
    }

    /// Charge la sauvegarde et applique les gains hors-ligne.
    pub fn load(&mut self) -> Result<Vec<GameEvent>, AppError> {
        let mut out = Vec::new();
        match self.repo.load()? {
            LoadOutcome::NotFound => {
                self.state = GameState::new(&self.catalog);
            }
            LoadOutcome::Corrupted { backup } => {
                self.state = GameState::new(&self.catalog);
                out.push(GameEvent::SaveRecovered { backup });
            }
            LoadOutcome::Tampered { backup } => {
                self.state = GameState::new(&self.catalog);
                out.push(GameEvent::SaveTampered { backup });
            }
            LoadOutcome::Loaded(data) if data.version < SAVE_RESET_BELOW => {
                // Ancien équilibrage : on repart de zéro et l'ancienne partie
                // est écrasée tout de suite par la nouvelle.
                self.state = GameState::new(&self.catalog);
                self.reduced_effects = data.reduced_effects;
                self.frame_rate = data.frame_rate;
                self.color_mode = data.color_mode.min(2);
                self.save()?;
                out.push(GameEvent::SaveWiped);
            }
            LoadOutcome::Loaded(data) => {
                self.state = data.to_state(&self.catalog);
                self.reduced_effects = data.reduced_effects;
                self.frame_rate = data.frame_rate;
                self.color_mode = data.color_mode.min(2);
                self.started_at = data.started_at;
                let now = self.clock.now_unix_secs();
                let elapsed = now.saturating_sub(data.last_saved);
                if data.last_saved > 0 && elapsed >= MIN_OFFLINE_SECS {
                    let mut ev = Vec::new();
                    self.state
                        .apply_offline(&self.catalog, elapsed as f64, &mut ev);
                    out.extend(ev.into_iter().map(GameEvent::Domain));
                }
            }
        }
        self.history.clear();
        self.history_mix.clear();
        self.sample_timer = 0.0;
        self.autosave_timer = 0.0;
        Ok(out)
    }

    /// Sauvegarde immédiate.
    pub fn save(&mut self) -> Result<(), AppError> {
        let data = self.save_data();
        self.repo.save(&data)?;
        self.autosave_timer = 0.0;
        Ok(())
    }

    /// Contenu de la sauvegarde à cet instant (envoyé aux autres appareils).
    pub fn save_data(&self) -> SaveData {
        // Pendant un Défi du jour, c'est la partie principale qu'on sauvegarde.
        let state = match &self.daily {
            Some(d) => &*d.main,
            None => &self.state,
        };
        let mut data = SaveData::from_state(
            state,
            &self.catalog,
            self.clock.now_unix_secs(),
            self.reduced_effects,
        );
        data.frame_rate = self.frame_rate;
        data.color_mode = self.color_mode;
        data.started_at = self.started_at;
        data
    }

    /// Début de la partie (dernier « Nouvelle partie », secondes Unix).
    pub fn started_at(&self) -> u64 {
        self.started_at
    }

    /// Remplace la sauvegarde par celle d'un autre appareil (l'actuelle est
    /// gardée en copie). Appeler [`GameService::load`] ensuite pour la reprendre.
    pub fn replace_save(&mut self, data: &SaveData) -> Result<(), AppError> {
        self.repo.keep_backup()?;
        self.repo.save(data)?;
        Ok(())
    }

    /// Un Défi du jour est en cours (la partie principale est mise de côté).
    pub fn in_daily(&self) -> bool {
        self.daily.is_some()
    }

    /// Heure Unix selon l'horloge du jeu.
    pub fn now_unix(&self) -> u64 {
        self.clock.now_unix_secs()
    }

    /// Jour (depuis 1970) selon l'horloge.
    fn today(&self) -> u64 {
        self.clock.now_unix_secs() / 86_400
    }

    fn start_daily(&mut self) -> GameEvent {
        if self.daily.is_some() || !self.state.daily_unlocked() {
            return GameEvent::DailyUnavailable;
        }
        let day = self.today();
        let pocket = GameState::daily_pocket(&self.catalog, day);
        let main = std::mem::replace(&mut self.state, pocket);
        self.daily = Some(DailyRun {
            main: Box::new(main),
            day,
            left: domain::daily::DAILY_SECS,
        });
        self.history.clear();
        self.history_mix.clear();
        GameEvent::DailyStarted { day }
    }

    fn end_daily(&mut self) -> GameEvent {
        let Some(run) = self.daily.take() else {
            return GameEvent::DailyUnavailable;
        };
        let score = self.state.total_all;
        self.state = *run.main;
        let reward = self.state.record_daily(run.day, score);
        self.history.clear();
        self.history_mix.clear();
        GameEvent::DailyEnded {
            score,
            best: self.state.daily_best,
            reward,
            code: domain::daily::share_code(run.day, score),
        }
    }

    fn save_event(&mut self, automatic: bool) -> GameEvent {
        match self.save() {
            Ok(()) => GameEvent::Saved { automatic },
            Err(e) => GameEvent::SaveFailed(e.to_string()),
        }
    }

    /// Exécute une commande et renvoie les événements produits.
    pub fn handle(&mut self, command: Command) -> Vec<GameEvent> {
        let mut ev = std::mem::take(&mut self.scratch);
        ev.clear();
        let mut out = Vec::new();
        match command {
            Command::Harvest => self.state.harvest(&self.catalog, &mut ev),
            Command::HarvestHeld => self.state.harvest_held(&self.catalog, &mut ev),
            Command::BuyGenerator(id) => {
                self.state
                    .buy_generator(&self.catalog, id, self.buy_mode, &mut ev)
            }
            Command::BuyUpgrade(id) => self.state.buy_upgrade(&self.catalog, id, &mut ev),
            Command::CycleBuyMode => {
                self.buy_mode = self.buy_mode.next();
                out.push(GameEvent::BuyModeChanged(self.buy_mode));
            }
            Command::CatchComet => {
                let reward = self.rng.next_f64();
                let next = self.rng.next_f64();
                self.state.catch_comet(&self.catalog, reward, next, &mut ev);
            }
            Command::Supernova => self.state.supernova(&self.catalog, &mut ev),
            Command::Save => out.push(self.save_event(false)),
            Command::BuyAllUpgrades => self.state.buy_all_upgrades(&self.catalog, &mut ev),
            Command::BuyPerk(id) => self.state.buy_perk(&self.catalog, id, &mut ev),
            Command::ToggleAutoBuy => self.state.toggle_autobuy(&self.catalog, &mut ev),
            Command::StartChallenge(id) => self.state.start_challenge(&self.catalog, id, &mut ev),
            Command::AbandonChallenge => self.state.abandon_challenge(&self.catalog, &mut ev),
            Command::BigCrunch(next) => self.state.big_crunch(&self.catalog, next, &mut ev),
            Command::StartResearch(id) => self.state.start_research(&self.catalog, id, &mut ev),
            Command::ToggleLab(id) => self.state.toggle_lab(&self.catalog, id, &mut ev),
            Command::BigBang => self.state.big_bang(&self.catalog, &mut ev),
            Command::Forge(id) => self.state.forge(&self.catalog, id, &mut ev),
            Command::BuildAttractor => self.state.build_attractor(&self.catalog, &mut ev),
            Command::LaunchExpedition(system) => {
                let roll = self.rng.next_f64();
                self.state
                    .launch_expedition(&self.catalog, system, roll, &mut ev)
            }
            Command::ToggleRelic(id) => self.state.toggle_relic(&self.catalog, id, &mut ev),
            Command::ResolveEvent(choice) => {
                let roll = self.rng.next_f64();
                self.state
                    .resolve_event(&self.catalog, choice, roll, &mut ev)
            }
            Command::StartConvergence => self.state.start_convergence(&self.catalog, &mut ev),
            Command::ToggleAutoFleets => self.state.toggle_auto_fleets(&mut ev),
            Command::PlaceStructure(cell, kind) => {
                self.state
                    .place_structure(&self.catalog, cell, kind, &mut ev)
            }
            Command::Demolish(cell) => self.state.demolish(cell, &mut ev),
            Command::StartDaily => out.push(self.start_daily()),
            Command::StartAwakening => self.state.start_awakening(&mut ev),
            Command::EndDaily => {
                if self.daily.is_some() {
                    out.push(self.end_daily());
                }
            }
            Command::CycleColorMode => {
                self.color_mode = (self.color_mode + 1) % 3;
                out.push(GameEvent::ColorModeChanged(self.color_mode));
            }
            Command::CycleFrameRate => {
                self.frame_rate = match self.frame_rate {
                    0 | 30 => 60,
                    60 => 20,
                    20 => 15,
                    _ => 30,
                };
                out.push(GameEvent::FrameRateChanged(self.frame_rate));
            }
            Command::NewGame => {
                self.state = GameState::new(&self.catalog);
                // Plus récente que toutes les parties des appareils liés :
                // c'est elle qu'ils reprendront.
                self.started_at = self.clock.now_unix_secs().max(self.started_at + 1);
                self.history.clear();
                self.history_mix.clear();
                self.sample_timer = 0.0;
                out.push(self.save_event(false));
                out.push(GameEvent::NewGame);
            }
            Command::BuyConstellation(id) => {
                self.state.buy_constellation(&self.catalog, id, &mut ev)
            }
            Command::ToggleReducedEffects => {
                self.reduced_effects = !self.reduced_effects;
                out.push(GameEvent::ReducedEffectsChanged(self.reduced_effects));
            }
        }
        self.clear_chart_after_prestige(&ev);
        out.extend(ev.drain(..).map(GameEvent::Domain));
        self.scratch = ev;
        out
    }

    /// Après une Supernova, un Big Crunch ou un Big Bang (à la main ou par un
    /// automatisme), la production repart de presque rien : le graphique
    /// repart de zéro au lieu de tasser la nouvelle courbe sous l'ancienne.
    fn clear_chart_after_prestige(&mut self, ev: &[DomainEvent]) {
        let prestige = ev.iter().any(|e| {
            matches!(
                e,
                DomainEvent::Supernova { .. }
                    | DomainEvent::BigCrunch { .. }
                    | DomainEvent::BigBang { .. }
            )
        });
        if prestige {
            self.history.clear();
            self.history_mix.clear();
            self.sample_timer = 0.0;
        }
    }

    /// Fait avancer le jeu de `dt` secondes (temps réel).
    pub fn tick(&mut self, dt: f64) -> Vec<GameEvent> {
        let mut out = Vec::new();
        if !dt.is_finite() || dt <= 0.0 {
            return out;
        }
        let mut ev = std::mem::take(&mut self.scratch);
        ev.clear();
        let roll = self.rng.next_f64();
        self.state.tick(&self.catalog, dt, roll, &mut ev);
        // Pendant le défi, la partie principale continue en temps réel
        // (recherches, chantiers, flottes, matière exotique), comme jeu fermé.
        if let Some(d) = self.daily.as_mut() {
            let mut ignored = Vec::new();
            d.main.apply_offline(&self.catalog, dt, &mut ignored);
            d.left -= dt;
            if d.left <= 0.0 {
                out.push(self.end_daily());
            }
        }
        self.clear_chart_after_prestige(&ev);
        out.extend(ev.drain(..).map(GameEvent::Domain));
        self.scratch = ev;

        self.sample_timer += dt;
        if self.sample_timer >= SAMPLE_INTERVAL {
            self.sample_timer %= SAMPLE_INTERVAL;
            // Échantillon par générateur ; on recycle le plus ancien buffer.
            let mut mix = if self.history.len() == HISTORY_LEN {
                self.history.pop_front();
                self.history_mix.pop_front().unwrap_or_default()
            } else {
                Vec::with_capacity(self.catalog.generators.len())
            };
            mix.clear();
            mix.extend(
                self.catalog
                    .generators
                    .iter()
                    .map(|g| economy::generator_production(&self.catalog, &self.state, g.id)),
            );
            self.history
                .push_back(economy::total_production(&self.catalog, &self.state));
            self.history_mix.push_back(mix);
        }

        self.autosave_timer += dt;
        if self.autosave_timer >= AUTOSAVE_INTERVAL {
            out.push(self.save_event(true));
        }
        out
    }

    pub fn snapshot(&self) -> GameSnapshot {
        let mut s = GameSnapshot::default();
        self.fill_snapshot(&mut s);
        s
    }

    /// Remplit `snap` en réutilisant ses allocations (appelé à chaque frame).
    pub fn fill_snapshot(&self, snap: &mut GameSnapshot) {
        let c = &self.catalog;
        let st = &self.state;
        let production = economy::total_production(c, st);
        let stardust = st.stardust.value();

        snap.stardust = stardust;
        snap.production = production;
        snap.click_value = economy::click_value(c, st);
        snap.total_run = st.total_run;
        snap.total_all = st.total_all;
        snap.fragments = st.fragments;
        snap.fragments_available = st.fragments_available_with(c);
        snap.fragment_bonus = domain::prestige::fragment_multiplier(st.fragments) - 1.0;
        snap.supernovas = st.supernovas;
        snap.clicks = st.clicks;
        snap.meteors_caught = st.meteors_caught;
        snap.play_time = st.play_time;
        snap.achievements_unlocked = st.achievements_unlocked();
        snap.achievements_total = c.achievements.len();
        snap.achievements.clear();
        snap.achievements
            .extend(c.achievements.iter().map(|a| st.has_achievement(a.id)));
        snap.buy_mode = self.buy_mode;
        snap.storm_remaining = st.comet.storm_remaining;
        snap.comet_progress = st.comet.progress();
        snap.reduced_effects = self.reduced_effects;
        snap.frame_rate = if self.frame_rate == 0 {
            30
        } else {
            self.frame_rate
        };
        snap.hold_harvest = economy::hold_harvest_unlocked(c, st);
        snap.fragments_spendable = st.fragments_spendable();
        snap.autobuy_unlocked = st.autobuy_unlocked(c);
        snap.autobuy = st.autobuy;
        snap.perks.clear();
        snap.perks.extend(c.perks.iter().map(|p| PerkView {
            id: p.id,
            owned: st.has_perk(p.id),
            affordable: !st.has_perk(p.id) && st.fragments_spendable() + 1e-9 >= p.cost,
        }));
        snap.constellations.clear();
        snap.constellations.extend(c.constellations.iter().map(|k| {
            let level = st.constellation_level(k.id);
            let cost = k.cost_at(level);
            let maxed = k.is_maxed(level);
            ConstellationView {
                id: k.id,
                level,
                cost,
                affordable: !maxed && st.fragments_spendable() + 1e-9 >= cost,
                maxed,
            }
        }));
        snap.constellation_levels = st.constellation_levels_total();
        snap.challenges.clear();
        snap.challenges
            .extend(c.challenges.iter().map(|k| ChallengeView {
                id: k.id,
                unlocked: st.supernovas >= k.unlock_supernovas,
                done: st.challenge_done(k.id),
                active: st.challenge == Some(k.id),
            }));
        snap.active_challenge = st.challenge.and_then(|id| {
            let def = c.challenge(id)?;
            let elapsed = st.play_time - st.challenge_started;
            Some(ActiveChallengeView {
                id,
                progress: (st.total_run / def.goal).clamp(0.0, 1.0),
                elapsed,
                time_left: match def.restriction {
                    domain::Restriction::TimeLimit(l) => Some((l - elapsed).max(0.0)),
                    _ => None,
                },
            })
        });
        snap.comet_kind = st.comet.kind;
        snap.storm_multiplier = st.comet.boost();
        let parts = economy::production_breakdown(c, st);
        snap.permanent_multiplier = parts.iter().map(|(_, f)| f).product();
        snap.production_sources.clear();
        snap.production_sources.extend(
            parts
                .iter()
                .filter(|(_, f)| (f - 1.0).abs() > 1e-9)
                .map(|(p, f)| (p.label(), *f)),
        );
        snap.crunches = st.crunches;
        snap.crunch_available = st.can_big_crunch();
        snap.crunch_needed = domain::catalog::balance::CRUNCH_SUPERNOVAS;
        snap.crunch_production = st.crunch_production(c);
        snap.crunch_production_next = st.crunch_production_with(c, st.crunches + 1);
        snap.crunch_fragments = st.crunch_fragments();
        snap.lab_unlocked = st.lab_unlocked();
        snap.lab_needed = domain::catalog::balance::LAB_UNLOCK_CRUNCHES;
        snap.dark_matter_spendable = st.dark_matter_spendable();
        snap.lab.clear();
        snap.lab.extend(c.lab.iter().map(|l| {
            LabView {
                id: l.id,
                done: st.lab_done(l.id),
                paused: st.lab_is_paused(l.id),
                available: st.can_research(c, l.id),
                blocked: l
                    .requires
                    .and_then(|k| c.lab_by_key(k))
                    .is_some_and(|p| !st.lab_done(p)),
            }
        }));
        snap.universe = st.universe;
        snap.universes_unlocked = st.universes_unlocked();
        snap.universes.clear();
        snap.universes
            .extend(c.universes.iter().map(|u| UniverseView {
                id: u.id,
                current: st.universe == u.id,
                key: st.has_universe_key(u.id),
            }));
        snap.entropy = st.entropy_factor(c);
        snap.big_bangs = st.big_bangs;
        snap.particles = st.particles;
        snap.particles_spendable = st.particles_spendable();
        snap.particles_available = st.particles_available();
        snap.bigbang_available = st.can_big_bang();
        snap.bigbang_needed = domain::catalog::balance::BIGBANG_CRUNCHES;
        snap.particle_production = st.particle_production();
        snap.particle_fragments = st.particle_fragments();
        snap.forges.clear();
        snap.forges.extend(
            c.generators
                .iter()
                .filter(|g| g.is_primordial())
                .map(|g| ForgeView {
                    id: g.id,
                    forged: st.is_forged(g.id),
                    available: st.can_forge(c, g.id),
                }),
        );
        snap.attractor_stages = st.attractor;
        snap.attractor_build = st.attractor_build.and_then(|left| {
            let stage = c.attractor_stage(st.attractor)?;
            Some(ResearchBuildView {
                stage: st.attractor,
                remaining: left,
                total: stage.build_secs,
            })
        });
        snap.attractor_buildable = st.can_build_attractor(c);
        snap.attractor_awakened = st.attractor_awakened(c);
        snap.attractor_production = st.attractor_production(c);
        snap.starmap_unlocked = st.starmap_unlocked(c) && !st.map.systems.is_empty();
        snap.fleets_max = st.fleets_max();
        snap.explored = st.explored_count();
        snap.systems.clear();
        snap.systems
            .extend(st.map.systems.iter().map(|sys| SystemView {
                index: sys.index,
                name: sys.name.clone(),
                links: sys.links.clone(),
                column: sys.column,
                y: sys.y,
                kind: sys.kind,
                danger: sys.danger,
                explored: st.is_explored(sys.index),
                revealed: st.is_revealed(sys.index),
                available: st.can_launch_expedition(c, sys.index),
                chance: st.expedition_chance_in(c, sys.index),
                duration: st.expedition_duration(c, sys.index),
                reward_scale: sys.reward_scale(),
            }));
        snap.fleets.clear();
        snap.fleets.extend(st.fleets.iter().map(|f| FleetView {
            target: f.target,
            remaining: f.left,
            total: f.total,
        }));
        snap.map_production = st.map_bonus(domain::SystemKind::Nebula, 0.10);
        snap.map_fragments = st.map_bonus(domain::SystemKind::Cluster, 0.10);
        snap.relics.clear();
        snap.relics.extend(c.relics.iter().map(|r| RelicView {
            id: r.id,
            rarity: st.relic_rarity(r.id),
            equipped: st.is_equipped(r.id),
        }));
        snap.relic_slots = st.relic_slots();
        snap.relics_equipped = st.equipped.len();
        snap.shards = st.shards;
        snap.relic_production = st.relic_production(c);
        snap.pending_event = st.pending_event;
        snap.boosts.clear();
        snap.boosts.extend(st.boosts.iter().copied());
        snap.events_resolved = st.events_resolved;
        snap.convergence_ready = st.convergence_ready(c);
        snap.convergence = st
            .convergence
            .map(|left| (left, domain::cosmic::CONVERGENCE_SECS));
        snap.converged = st.converged;
        snap.auto_fleets = st.auto_fleets;
        snap.auto_fleets_unlocked = st.auto_fleets_unlocked();
        snap.history_long.clear();
        snap.history_long.extend(st.history.iter().copied());
        snap.best_production = st.best_production;
        snap.milestones = st.milestones;
        snap.color_mode = self.color_mode;
        snap.started_at = self.started_at;
        snap.megastructures_unlocked = st.megastructures_unlocked();
        let today = self.today();
        snap.daily_rules = domain::daily::rules_for(today).0;
        snap.daily = self.daily.as_ref().map(|d| DailyView {
            left: d.left,
            score: st.total_all,
        });
        // Record et récompense du jour : lus dans la partie principale.
        let main = self.daily.as_ref().map(|d| &*d.main).unwrap_or(st);
        snap.daily_unlocked = main.daily_unlocked();
        let same_day = main.daily_day == today;
        snap.daily_best = if same_day { main.daily_best } else { 0.0 };
        snap.daily_rewarded = same_day && main.daily_rewarded;
        snap.exotic = st.exotic;
        snap.grid.clear();
        snap.grid.extend(st.grid.iter().copied());
        snap.grid_effects = st.grid_effects();
        snap.structure_costs = domain::StructureKind::ALL.map(|k| st.structure_cost(k));
        snap.awakening_ready = st.awakening_ready();
        snap.awakening = st
            .awakening
            .map(|l| (l, domain::megastructure::AWAKENING_SECS));
        snap.awakened = st.awakened;
        snap.relic_sets = domain::RelicSet::ALL
            .iter()
            .map(|&s| {
                let n = st
                    .equipped
                    .iter()
                    .filter(|&&id| c.relic(id).is_some_and(|d| d.set == s))
                    .count();
                (s, n)
            })
            .collect();
        snap.research = st.research.and_then(|(id, left)| {
            let def = c.lab_node(id)?;
            Some(ResearchView {
                id,
                remaining: left,
                total: def.research_secs,
            })
        });
        snap.next_upgrade = st.next_locked_upgrade(c).and_then(|id| {
            let def = c.upgrade(id)?;
            let (current, needed) = match def.requirement {
                Requirement::GeneratorOwned { generator, count } => {
                    (st.owned_of(generator) as f64, count as f64)
                }
                Requirement::TotalRun(r) => (st.total_run, r),
            };
            Some(NextUpgradeView {
                id,
                current,
                needed,
            })
        });
        let generators_found = st.owned.iter().filter(|&&o| o > 0).count();
        let perks_owned = st.perks_owned();
        let achievements = st.achievements_unlocked();
        let challenges_done = st.challenges_done.iter().filter(|&&d| d).count();
        let lab_done = st.lab.iter().filter(|&&d| d).count();
        let found_total = c.generators.len()
            + c.achievements.len()
            + c.perks.len()
            + c.challenges.len()
            + c.lab.len();
        snap.progress = ProgressView {
            generators_found,
            generators_total: c.generators.len(),
            upgrades_bought: st.upgrades.iter().filter(|&&u| u).count(),
            upgrades_total: c.upgrades.len(),
            perks_owned,
            perks_total: c.perks.len(),
            completion: if found_total > 0 {
                (generators_found + achievements + perks_owned + challenges_done + lab_done) as f64
                    / found_total as f64
            } else {
                0.0
            },
            next_fragment_at: st.total_run_for_next_fragment(c),
        };

        snap.generators.clear();
        snap.teaser = None;
        for g in &c.generators {
            if st.is_visible(c, g.id) {
                let owned = st.owned_of(g.id);
                let quantity = st.purchase_quantity(c, g.id, self.buy_mode);
                let cost = st.generator_cost(c, g.id, quantity);
                let gen_prod = economy::generator_production(c, st, g.id);
                snap.generators.push(GeneratorView {
                    id: g.id,
                    owned,
                    production: gen_prod,
                    share: if production > 0.0 {
                        gen_prod / production
                    } else {
                        0.0
                    },
                    quantity,
                    cost,
                    affordable: st.stardust.can_afford(cost),
                });
            } else if snap.teaser.is_none() {
                snap.teaser = Some(TeaserView {
                    id: g.id,
                    remaining: (g.base_cost * domain::catalog::balance::REVEAL_RATIO
                        - st.total_run)
                        .max(0.0),
                    supernovas_needed: if st.supernovas < g.unlock_supernovas {
                        g.unlock_supernovas
                    } else {
                        0
                    },
                    particles_needed: if g.is_primordial() && !st.is_forged(g.id) {
                        g.particle_cost
                    } else {
                        0
                    },
                });
            }
        }

        snap.upgrades.clear();
        let upgrade_factor = st.upgrade_cost_factor(c);
        for u in &c.upgrades {
            if st.is_upgrade_available(c, u.id) {
                snap.upgrades.push(UpgradeView {
                    id: u.id,
                    cost: u.cost * upgrade_factor,
                    affordable: st.stardust.can_afford(u.cost * upgrade_factor),
                });
            }
        }
        snap.upgrades.sort_unstable_by(|a, b| {
            a.cost
                .partial_cmp(&b.cost)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.id.cmp(&b.id))
        });

        snap.owned.clear();
        snap.owned.extend_from_slice(&st.owned);
        snap.history.clear();
        snap.history.extend(self.history.iter().copied());
        snap.history_mix_width = c.generators.len();
        snap.history_mix.clear();
        for sample in &self.history_mix {
            snap.history_mix.extend_from_slice(sample);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::RepositoryError;
    use domain::{GeneratorId, Stardust};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    #[derive(Clone)]
    struct FakeClock(Rc<Cell<u64>>);
    impl Clock for FakeClock {
        fn now_unix_secs(&self) -> u64 {
            self.0.get()
        }
    }

    struct FakeRng(Vec<f64>, usize);
    impl RandomSource for FakeRng {
        fn next_f64(&mut self) -> f64 {
            let v = self.0[self.1 % self.0.len()];
            self.1 += 1;
            v
        }
    }

    #[derive(Clone, Default)]
    struct InMemoryRepository {
        slot: Rc<RefCell<Option<SaveData>>>,
        corrupted: Rc<Cell<bool>>,
        fail_writes: Rc<Cell<bool>>,
    }
    impl SaveRepository for InMemoryRepository {
        fn load(&self) -> Result<LoadOutcome, RepositoryError> {
            if self.corrupted.get() {
                return Ok(LoadOutcome::Corrupted {
                    backup: "save.json.bak".into(),
                });
            }
            Ok(match self.slot.borrow().clone() {
                Some(d) => LoadOutcome::Loaded(Box::new(d)),
                None => LoadOutcome::NotFound,
            })
        }
        fn save(&self, data: &SaveData) -> Result<(), RepositoryError> {
            if self.fail_writes.get() {
                return Err(RepositoryError("disque plein".into()));
            }
            *self.slot.borrow_mut() = Some(data.clone());
            Ok(())
        }
        fn location(&self) -> String {
            "mémoire".into()
        }
    }

    fn service(repo: &InMemoryRepository, clock: &FakeClock, rolls: Vec<f64>) -> GameService {
        GameService::new(
            Box::new(repo.clone()),
            Box::new(clock.clone()),
            Box::new(FakeRng(rolls, 0)),
        )
    }

    fn fixture() -> (InMemoryRepository, FakeClock) {
        (
            InMemoryRepository::default(),
            FakeClock(Rc::new(Cell::new(1_000_000))),
        )
    }

    #[test]
    fn new_game_when_no_save() {
        let (repo, clock) = fixture();
        let mut s = service(&repo, &clock, vec![0.5]);
        assert!(s.load().unwrap().is_empty());
        let snap = s.snapshot();
        assert_eq!(snap.stardust, 0.0);
        assert_eq!(snap.generators.len(), 1, "seule la sonde est visible");
        assert_eq!(snap.teaser.map(|t| t.id), Some(GeneratorId(1)));
    }

    #[test]
    fn harvest_then_purchase() {
        let (repo, clock) = fixture();
        let mut s = service(&repo, &clock, vec![0.5]);
        s.load().unwrap();
        for _ in 0..15 {
            s.handle(Command::Harvest);
        }
        let ev = s.handle(Command::BuyGenerator(GeneratorId(0)));
        assert!(ev.iter().any(|e| matches!(
            e,
            GameEvent::Domain(DomainEvent::Purchased { count: 1, .. })
        )));
        let snap = s.snapshot();
        assert_eq!(snap.generators[0].owned, 1);
        assert!(snap.production > 0.0);
    }

    #[test]
    fn buy_mode_cycles_and_changes_cost() {
        let (repo, clock) = fixture();
        let mut s = service(&repo, &clock, vec![0.5]);
        s.load().unwrap();
        let ev = s.handle(Command::CycleBuyMode);
        assert_eq!(ev, [GameEvent::BuyModeChanged(BuyMode::Ten)]);
        let snap = s.snapshot();
        assert_eq!(snap.generators[0].quantity, 10);
        assert!((snap.generators[0].cost - s.catalog().generators[0].cost_for(0, 10)).abs() < 1e-9);
    }

    #[test]
    fn save_load_round_trip_with_offline_gains() {
        let (repo, clock) = fixture();
        let mut s = service(&repo, &clock, vec![0.5]);
        s.load().unwrap();
        for _ in 0..200 {
            s.handle(Command::Harvest);
        }
        s.handle(Command::BuyGenerator(GeneratorId(0)));
        s.handle(Command::BuyGenerator(GeneratorId(1)));
        s.handle(Command::ToggleReducedEffects);
        assert_eq!(
            s.handle(Command::Save),
            [GameEvent::Saved { automatic: false }]
        );
        let before = s.snapshot();

        clock.0.set(clock.0.get() + 3600);
        let mut s2 = service(&repo, &clock, vec![0.5]);
        let ev = s2.load().unwrap();
        let after = s2.snapshot();
        assert_eq!(after.owned, before.owned);
        assert_eq!(after.clicks, 200);
        assert!(after.reduced_effects);
        let expected_gain = before.production * 3600.0 * 0.5;
        assert!((after.stardust - before.stardust - expected_gain).abs() < 1e-6);
        assert!(matches!(
            ev.as_slice(),
            [GameEvent::Domain(DomainEvent::OfflineGains { seconds, .. })] if *seconds == 3600.0
        ));
    }

    #[test]
    fn corrupted_save_starts_new_game() {
        let (repo, clock) = fixture();
        repo.corrupted.set(true);
        let mut s = service(&repo, &clock, vec![0.5]);
        let ev = s.load().unwrap();
        assert_eq!(
            ev,
            [GameEvent::SaveRecovered {
                backup: "save.json.bak".into()
            }]
        );
        assert_eq!(s.snapshot().stardust, 0.0);
    }

    #[test]
    fn save_failure_is_reported_not_fatal() {
        let (repo, clock) = fixture();
        repo.fail_writes.set(true);
        let mut s = service(&repo, &clock, vec![0.5]);
        s.load().unwrap();
        assert!(matches!(
            s.handle(Command::Save)[0],
            GameEvent::SaveFailed(_)
        ));
    }

    #[test]
    fn autosave_every_30s_and_history_sampling() {
        let (repo, clock) = fixture();
        let mut s = service(&repo, &clock, vec![0.5]);
        s.load().unwrap();
        let mut saves = 0;
        for _ in 0..(61 * 10) {
            saves += s
                .tick(0.1)
                .iter()
                .filter(|e| matches!(e, GameEvent::Saved { automatic: true }))
                .count();
        }
        assert_eq!(saves, 2);
        assert!(repo.slot.borrow().is_some());
        let snap = s.snapshot();
        assert_eq!(
            snap.history_mix.len(),
            snap.history.len() * snap.history_mix_width,
            "un échantillon par générateur à chaque seconde"
        );
        let h = snap.history.len();
        assert!((60..=61).contains(&h), "{h}");
    }

    /// Après un prestige, la production repart de presque rien : le graphique
    /// repart de zéro au lieu d'écraser la nouvelle courbe sous l'ancienne.
    #[test]
    fn a_prestige_clears_the_production_chart() {
        let (repo, clock) = fixture();
        let mut s = service(&repo, &clock, vec![0.5]);
        s.load().unwrap();
        let mut data = SaveData::from_state(s.state(), s.catalog(), 0, false);
        data.total_run = 4e8;
        data.total_all = 4e8;
        data.owned[0] = 50;
        *repo.slot.borrow_mut() = Some(data);
        s.load().unwrap();
        for _ in 0..30 {
            s.tick(0.1);
        }
        s.tick(5.0);
        assert!(!s.snapshot().history.is_empty());
        let ev = s.handle(Command::Supernova);
        assert!(ev
            .iter()
            .any(|e| matches!(e, GameEvent::Domain(DomainEvent::Supernova { .. }))));
        let snap = s.snapshot();
        assert!(snap.history.is_empty(), "{:?}", snap.history);
        assert!(snap.history_mix.is_empty());
    }

    #[test]
    fn comet_catch_and_supernova() {
        let (repo, clock) = fixture();
        // Tirages : 0.1 → Tempête solaire.
        let mut s = service(&repo, &clock, vec![0.1]);
        s.load().unwrap();
        let ev = s.tick(46.0);
        assert!(ev
            .iter()
            .any(|e| matches!(e, GameEvent::Domain(DomainEvent::CometSpawned { .. }))));
        assert!(s.snapshot().comet_progress.is_some());
        let ev = s.handle(Command::CatchComet);
        assert!(ev
            .iter()
            .any(|e| matches!(e, GameEvent::Domain(DomainEvent::CometCaught { .. }))));
        assert!(s.snapshot().storm_remaining > 0.0);

        let ev = s.handle(Command::Supernova);
        assert_eq!(ev, [GameEvent::Domain(DomainEvent::SupernovaUnavailable)]);

        let mut data = SaveData::from_state(s.state(), s.catalog(), 0, false);
        data.total_run = 4e8;
        data.total_all = 4e8;
        data.stardust = 1e8;
        *repo.slot.borrow_mut() = Some(data);
        s.load().unwrap();
        assert_eq!(
            s.snapshot().fragments_available,
            6.0,
            "floor(sqrt(4e8 / 1e7))"
        );
        let ev = s.handle(Command::Supernova);
        assert!(ev.iter().any(|e| matches!(
            e,
            GameEvent::Domain(DomainEvent::Supernova { fragments_gained, .. }) if *fragments_gained == 6.0
        )));
        let snap = s.snapshot();
        assert_eq!(snap.stardust, 0.0);
        assert_eq!(snap.fragments, 6.0);
        assert_eq!(snap.fragments_spendable, 6.0);
        // 6 fragments : 1 + 0.05 × ∛(6/5) (courbe en racine au-delà de 5).
        assert!((snap.fragment_bonus - 0.05 * (6.0f64 / 5.0).cbrt()).abs() < 1e-12);
        assert_eq!(snap.perks.len(), 8);
        assert!(snap.perks.iter().any(|p| p.affordable));
        // Nouveau cycle, 1 Supernova faite : le seuil passe de 1e7 à 1.25e7.
        assert!((snap.progress.next_fragment_at - 1.25e7).abs() < 1.0);
        let _ = Stardust::ZERO;

        // Constellation : achat puis aller-retour par la sauvegarde.
        let forge = s.catalog().constellation_by_key("const_forge").unwrap();
        s.handle(Command::BuyConstellation(forge));
        assert_eq!(s.snapshot().constellations[forge.index()].level, 1);
        assert_eq!(s.snapshot().fragments_spendable, 4.0);
        s.handle(Command::Save);
        s.load().unwrap();
        let snap = s.snapshot();
        assert_eq!(snap.constellations[forge.index()].level, 1);
        assert_eq!(snap.fragments_spendable, 4.0);

        // Défi : lancé, sauvegardé et rechargé en cours.
        let noclick = s.catalog().challenge_by_key("ch_noclick").unwrap();
        s.handle(Command::StartChallenge(noclick));
        assert_eq!(s.snapshot().active_challenge.map(|a| a.id), Some(noclick));
        s.handle(Command::Save);
        s.load().unwrap();
        assert_eq!(s.snapshot().active_challenge.map(|a| a.id), Some(noclick));
        s.handle(Command::AbandonChallenge);
        assert_eq!(s.snapshot().active_challenge, None);
    }

    #[test]
    fn upgrades_sorted_by_cost() {
        let (repo, clock) = fixture();
        let mut s = service(&repo, &clock, vec![0.5]);
        let mut data = SaveData::from_state(s.state(), s.catalog(), 0, false);
        data.owned = vec![10, 10, 0, 0, 0, 0, 0, 0, 0];
        data.total_run = 2e4;
        *repo.slot.borrow_mut() = Some(data);
        s.load().unwrap();
        let snap = s.snapshot();
        let costs: Vec<f64> = snap.upgrades.iter().map(|u| u.cost).collect();
        assert!(costs.len() >= 3);
        assert!(costs.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn outdated_saves_are_wiped_with_a_notice() {
        let (repo, clock) = fixture();
        let mut s = service(&repo, &clock, vec![0.5]);
        let mut old = SaveData::from_state(s.state(), s.catalog(), 0, true);
        old.version = SAVE_RESET_BELOW - 1;
        old.supernovas = 12;
        old.fragments = 99.0;
        *repo.slot.borrow_mut() = Some(old);
        let ev = s.load().unwrap();
        assert!(ev.contains(&GameEvent::SaveWiped));
        assert_eq!(s.snapshot().supernovas, 0);
        assert!(s.snapshot().reduced_effects, "les réglages sont gardés");
        // L'ancienne partie est écrasée tout de suite sur le disque.
        let saved = repo.slot.borrow().clone().unwrap();
        assert_eq!(saved.version, crate::SAVE_VERSION);
        assert_eq!(saved.supernovas, 0);
        // Au lancement suivant, rien n'est effacé.
        assert!(!s.load().unwrap().contains(&GameEvent::SaveWiped));
    }

    #[test]
    fn new_game_wipes_progress_but_keeps_settings() {
        let (repo, clock) = fixture();
        let mut s = service(&repo, &clock, vec![0.5]);
        s.load().unwrap();
        for _ in 0..30 {
            s.handle(Command::Harvest);
        }
        s.handle(Command::ToggleReducedEffects);
        let ev = s.handle(Command::CycleFrameRate);
        assert!(ev.contains(&GameEvent::FrameRateChanged(60)));
        let ev = s.handle(Command::CycleFrameRate);
        assert!(ev.contains(&GameEvent::FrameRateChanged(20)));
        let ev = s.handle(Command::NewGame);
        assert!(ev.contains(&GameEvent::NewGame));
        let snap = s.snapshot();
        assert_eq!(snap.stardust, 0.0);
        assert!(snap.reduced_effects);
        assert_eq!(snap.frame_rate, 20);
        // La partie effacée est écrasée sur le disque, réglages compris.
        let saved = repo.slot.borrow().clone().unwrap();
        assert_eq!(saved.clicks, 0);
        assert_eq!(saved.frame_rate, 20);
        // Datée : les appareils liés la préféreront à l'ancienne partie.
        assert!(saved.started_at > 0);
        assert_eq!(s.started_at(), saved.started_at);
        let mut again = service(&repo, &clock, vec![0.5]);
        again.load().unwrap();
        assert_eq!(again.snapshot().frame_rate, 20);
    }

    #[test]
    fn daily_challenge_swaps_and_restores_the_main_game() {
        let (repo, clock) = fixture();
        let mut s = service(&repo, &clock, vec![0.5]);
        s.load().unwrap();
        assert_eq!(s.handle(Command::StartDaily), [GameEvent::DailyUnavailable]);
        s.state.converged = true;
        s.state.fragments = 42.0;
        let ev = s.handle(Command::StartDaily);
        assert!(matches!(ev[0], GameEvent::DailyStarted { .. }));
        assert_eq!(s.snapshot().fragments, 0.0, "univers de poche vierge");
        assert!(s.snapshot().daily.is_some());
        for _ in 0..200 {
            s.handle(Command::Harvest);
        }
        // La sauvegarde pendant le défi garde la partie principale.
        s.save().unwrap();
        assert_eq!(repo.slot.borrow().as_ref().unwrap().fragments, 42.0);
        // Dix minutes plus tard, fin du défi et retour à la partie principale.
        let mut ended = None;
        for _ in 0..(10 * 60 + 5) {
            for e in s.tick(1.0) {
                if let GameEvent::DailyEnded { score, code, .. } = e {
                    ended = Some((score, code));
                }
            }
        }
        let (score, code) = ended.expect("fin du défi");
        assert!(score >= 200.0);
        assert!(code.starts_with("FS"));
        let snap = s.snapshot();
        assert!(snap.daily.is_none());
        assert_eq!(snap.fragments, 42.0);
        assert_eq!(snap.daily_best, score);
        assert!(snap.daily_rewarded);
    }
}
