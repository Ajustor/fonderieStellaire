//! `GameService` : le cas d'usage principal. Reçoit des [`Command`], fait avancer
//! le temps, sauvegarde, et renvoie des événements à la présentation.

use std::collections::VecDeque;

use domain::{economy, BuyMode, Catalog, DomainEvent, GameState, Requirement};

use crate::commands::Command;
use crate::error::AppError;
use crate::ports::{Clock, LoadOutcome, RandomSource, SaveRepository};
use crate::save_dto::SaveData;
use crate::snapshot::{
    GameSnapshot, GeneratorView, NextUpgradeView, PerkView, ProgressView, TeaserView, UpgradeView,
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
}

pub struct GameService {
    catalog: Catalog,
    state: GameState,
    buy_mode: BuyMode,
    reduced_effects: bool,
    repo: Box<dyn SaveRepository>,
    clock: Box<dyn Clock>,
    rng: Box<dyn RandomSource>,
    history: VecDeque<f64>,
    /// Production de chaque générateur, échantillon par échantillon.
    history_mix: VecDeque<Vec<f64>>,
    sample_timer: f64,
    autosave_timer: f64,
    scratch: Vec<DomainEvent>,
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
            repo,
            clock,
            rng,
            history: VecDeque::with_capacity(HISTORY_LEN),
            history_mix: VecDeque::with_capacity(HISTORY_LEN),
            sample_timer: 0.0,
            autosave_timer: 0.0,
            scratch: Vec::new(),
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
            LoadOutcome::Loaded(data) => {
                self.state = data.to_state(&self.catalog);
                self.reduced_effects = data.reduced_effects;
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
        let data = SaveData::from_state(
            &self.state,
            &self.catalog,
            self.clock.now_unix_secs(),
            self.reduced_effects,
        );
        self.repo.save(&data)?;
        self.autosave_timer = 0.0;
        Ok(())
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
            Command::ToggleReducedEffects => {
                self.reduced_effects = !self.reduced_effects;
                out.push(GameEvent::ReducedEffectsChanged(self.reduced_effects));
            }
        }
        out.extend(ev.drain(..).map(GameEvent::Domain));
        self.scratch = ev;
        out
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
        let found_total = c.generators.len() + c.achievements.len() + c.perks.len();
        snap.progress = ProgressView {
            generators_found,
            generators_total: c.generators.len(),
            upgrades_bought: st.upgrades.iter().filter(|&&u| u).count(),
            upgrades_total: c.upgrades.len(),
            perks_owned,
            perks_total: c.perks.len(),
            completion: if found_total > 0 {
                (generators_found + achievements + perks_owned) as f64 / found_total as f64
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
                });
            }
        }

        snap.upgrades.clear();
        for u in &c.upgrades {
            if st.is_upgrade_available(c, u.id) {
                snap.upgrades.push(UpgradeView {
                    id: u.id,
                    cost: u.cost,
                    affordable: st.stardust.can_afford(u.cost),
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
                Some(d) => LoadOutcome::Loaded(d),
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

    #[test]
    fn comet_catch_and_supernova() {
        let (repo, clock) = fixture();
        // Tirages : 0.1 → Tempête solaire.
        let mut s = service(&repo, &clock, vec![0.1]);
        s.load().unwrap();
        let ev = s.tick(46.0);
        assert!(ev.contains(&GameEvent::Domain(DomainEvent::CometSpawned)));
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
        assert!((snap.fragment_bonus - 0.6).abs() < 1e-12);
        assert_eq!(snap.perks.len(), 8);
        assert!(snap.perks.iter().any(|p| p.affordable));
        // Nouveau cycle : le prochain fragment demande 1e7 récoltés.
        assert!((snap.progress.next_fragment_at - 1.0e7).abs() < 1.0);
        let _ = Stardust::ZERO;
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
}
