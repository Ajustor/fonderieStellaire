//! DTO de sauvegarde, en Rust pur (la sérialisation vit dans l'infrastructure).

use domain::{Catalog, GameState, Stardust};

pub const SAVE_VERSION: u32 = 2;
/// Les sauvegardes d'un format plus ancien sont effacées au chargement :
/// l'équilibrage a trop changé pour les garder (v0.9). Le joueur est prévenu.
pub const SAVE_RESET_BELOW: u32 = 2;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SaveData {
    pub version: u32,
    pub stardust: f64,
    pub total_run: f64,
    pub total_all: f64,
    /// Index = générateur.
    pub owned: Vec<u64>,
    /// Identifiants texte des améliorations achetées.
    pub upgrades: Vec<String>,
    /// Identifiants texte des succès débloqués.
    pub achievements: Vec<String>,
    pub fragments: f64,
    pub fragments_this_run: f64,
    pub supernovas: u32,
    pub clicks: u64,
    pub meteors_caught: u32,
    pub play_time: f64,
    /// Timestamp Unix (secondes) de la sauvegarde.
    pub last_saved: u64,
    /// Option « effets réduits ».
    pub reduced_effects: bool,
    /// Images par seconde choisies (0 : par défaut).
    pub frame_rate: u32,
    /// Identifiants texte des améliorations de l'Atelier achetées.
    pub perks: Vec<String>,
    pub fragments_spent: f64,
    /// Automate d'achat activé.
    pub autobuy: bool,
    /// Niveaux de Constellations (clé texte, niveau).
    pub constellations: Vec<(String, u32)>,
    /// Défi en cours (clé texte, vide sinon) et `play_time` de son lancement.
    pub challenge: String,
    pub challenge_started: f64,
    /// Clés des Défis réussis.
    pub challenges_done: Vec<String>,
    pub golden_caught: u32,
    pub black_caught: u32,
    /// Big Crunch déclenchés (matière noire).
    pub crunches: u32,
    pub dark_matter_spent: u32,
    /// Recherches terminées et automatismes en pause (clés texte).
    pub lab: Vec<String>,
    pub lab_paused: Vec<String>,
    /// Recherche en cours (clé, vide sinon) et secondes restantes.
    pub research: String,
    pub research_left: f64,
    /// Univers en cours (clé) et Clés obtenues.
    pub universe: String,
    pub universe_keys: Vec<String>,
    pub run_started: f64,
    /// Big Bang, particules, forges (index des générateurs) et Grand Attracteur.
    pub big_bangs: u32,
    pub particles: u32,
    pub particles_spent: u32,
    pub forged: Vec<usize>,
    pub attractor: usize,
    /// Secondes restantes de l'étape en construction (0 ou négatif : aucune).
    pub attractor_left: f64,
    /// Acte III : graine de la carte, systèmes explorés, flottes en route
    /// (cible, secondes restantes, durée, succès) et expéditions lancées.
    pub map_seed: u64,
    pub explored: Vec<usize>,
    pub fleets: Vec<(usize, f64, f64, bool)>,
    pub expeditions_launched: u32,
    /// Butin et prospection de chaque flotte (même ordre que `fleets`).
    pub fleets_loot: Vec<(f64, bool)>,
    /// Reliques : (clé, rareté), clés des équipées, Éclats.
    pub relics: Vec<(String, u8)>,
    pub equipped: Vec<String>,
    pub shards: u32,
    /// Événements cosmiques : délai, événement en attente (clé, secondes),
    /// bonus (0 production / 1 fragments, facteur, secondes), résolus.
    pub event_timer: f64,
    pub pending_event: String,
    pub pending_left: f64,
    pub boosts: Vec<(u8, f64, f64)>,
    pub events_resolved: u32,
    /// Convergence : secondes restantes (0 : aucune), achevée.
    pub convergence_left: f64,
    pub converged: bool,
    /// Pilote automatique, historique de production, record, grandes étapes.
    pub auto_fleets: bool,
    pub history: Vec<(f64, f32)>,
    pub best_production: f64,
    pub milestones: Vec<f64>,
    /// Couleurs : 0 normales, 1 daltonien, 2 contraste élevé.
    pub color_mode: u8,
}

impl SaveData {
    pub fn from_state(
        state: &GameState,
        catalog: &Catalog,
        now: u64,
        reduced_effects: bool,
    ) -> Self {
        SaveData {
            version: SAVE_VERSION,
            stardust: state.stardust.value(),
            total_run: state.total_run,
            total_all: state.total_all,
            owned: state.owned.clone(),
            upgrades: catalog
                .upgrades
                .iter()
                .filter(|u| state.has_upgrade(u.id))
                .map(|u| u.key.clone())
                .collect(),
            achievements: catalog
                .achievements
                .iter()
                .filter(|a| state.has_achievement(a.id))
                .map(|a| a.key.to_string())
                .collect(),
            fragments: state.fragments,
            fragments_this_run: state.fragments_this_run,
            supernovas: state.supernovas,
            clicks: state.clicks,
            meteors_caught: state.meteors_caught,
            play_time: state.play_time,
            last_saved: now,
            reduced_effects,
            frame_rate: 0,
            perks: catalog
                .perks
                .iter()
                .filter(|p| state.has_perk(p.id))
                .map(|p| p.key.to_string())
                .collect(),
            fragments_spent: state.fragments_spent,
            autobuy: state.autobuy,
            constellations: catalog
                .constellations
                .iter()
                .map(|c| (c.key.to_string(), state.constellation_level(c.id)))
                .filter(|(_, lvl)| *lvl > 0)
                .collect(),
            challenge: state
                .challenge
                .and_then(|id| catalog.challenge(id))
                .map(|c| c.key.to_string())
                .unwrap_or_default(),
            challenge_started: state.challenge_started,
            challenges_done: catalog
                .challenges
                .iter()
                .filter(|c| state.challenge_done(c.id))
                .map(|c| c.key.to_string())
                .collect(),
            golden_caught: state.golden_caught,
            black_caught: state.black_caught,
            crunches: state.crunches,
            dark_matter_spent: state.dark_matter_spent,
            lab: catalog
                .lab
                .iter()
                .filter(|l| state.lab_done(l.id))
                .map(|l| l.key.to_string())
                .collect(),
            lab_paused: catalog
                .lab
                .iter()
                .filter(|l| state.lab_is_paused(l.id))
                .map(|l| l.key.to_string())
                .collect(),
            research: state
                .research
                .and_then(|(id, _)| catalog.lab_node(id))
                .map(|l| l.key.to_string())
                .unwrap_or_default(),
            research_left: state.research.map(|(_, left)| left).unwrap_or(0.0),
            universe: catalog
                .universe(state.universe)
                .map(|u| u.key.to_string())
                .unwrap_or_default(),
            universe_keys: catalog
                .universes
                .iter()
                .filter(|u| state.has_universe_key(u.id))
                .map(|u| u.key.to_string())
                .collect(),
            run_started: state.run_started,
            big_bangs: state.big_bangs,
            particles: state.particles,
            particles_spent: state.particles_spent,
            forged: state
                .forged
                .iter()
                .enumerate()
                .filter(|(_, &f)| f)
                .map(|(i, _)| i)
                .collect(),
            attractor: state.attractor,
            attractor_left: state.attractor_build.unwrap_or(-1.0),
            map_seed: state.map_seed,
            explored: state
                .explored
                .iter()
                .enumerate()
                .filter(|(_, &e)| e)
                .map(|(i, _)| i)
                .collect(),
            fleets: state
                .fleets
                .iter()
                .map(|f| (f.target, f.left, f.total, f.success))
                .collect(),
            expeditions_launched: state.expeditions_launched,
            fleets_loot: state.fleets.iter().map(|f| (f.loot, f.prospect)).collect(),
            relics: catalog
                .relics
                .iter()
                .filter_map(|r| state.relic_rarity(r.id).map(|k| (r.key.to_string(), k)))
                .collect(),
            equipped: state
                .equipped
                .iter()
                .filter_map(|&id| catalog.relic(id).map(|r| r.key.to_string()))
                .collect(),
            shards: state.shards,
            event_timer: state.event_timer,
            pending_event: state
                .pending_event
                .and_then(|(i, _)| catalog.events.get(i))
                .map(|e| e.key.to_string())
                .unwrap_or_default(),
            pending_left: state.pending_event.map(|(_, l)| l).unwrap_or(0.0),
            boosts: state
                .boosts
                .iter()
                .map(|b| {
                    let kind = match b.kind {
                        domain::BoostKind::Production => 0,
                        domain::BoostKind::Fragments => 1,
                    };
                    (kind, b.factor, b.left)
                })
                .collect(),
            events_resolved: state.events_resolved,
            convergence_left: state.convergence.unwrap_or(0.0),
            converged: state.converged,
            auto_fleets: state.auto_fleets,
            history: state.history.clone(),
            best_production: state.best_production,
            milestones: state.milestones.to_vec(),
            color_mode: 0,
        }
    }

    /// Reconstruit l'état. Les identifiants inconnus sont ignorés.
    pub fn to_state(&self, catalog: &Catalog) -> GameState {
        let mut s = GameState::new(catalog);
        s.stardust = Stardust::new(self.stardust);
        s.total_run = self.total_run;
        s.total_all = self.total_all.max(self.total_run);
        for (i, &o) in self.owned.iter().enumerate().take(s.owned.len()) {
            s.owned[i] = o;
        }
        for key in &self.upgrades {
            if let Some(id) = catalog.upgrade_by_key(key) {
                s.upgrades[id.index()] = true;
            }
        }
        for key in &self.achievements {
            if let Some(id) = catalog.achievement_by_key(key) {
                s.achievements[id.index()] = true;
            }
        }
        s.fragments = self.fragments;
        s.fragments_this_run = self.fragments_this_run;
        s.supernovas = self.supernovas;
        s.clicks = self.clicks;
        s.meteors_caught = self.meteors_caught;
        s.play_time = self.play_time;
        for key in &self.perks {
            if let Some(id) = catalog.perk_by_key(key) {
                s.perks[id.index()] = true;
            }
        }
        s.fragments_spent = self.fragments_spent;
        s.autobuy = self.autobuy;
        for (key, level) in &self.constellations {
            if let Some(id) = catalog.constellation_by_key(key) {
                s.constellations[id.index()] = *level;
            }
        }
        s.challenge = catalog.challenge_by_key(&self.challenge);
        s.challenge_started = self.challenge_started;
        for key in &self.challenges_done {
            if let Some(id) = catalog.challenge_by_key(key) {
                s.challenges_done[id.index()] = true;
            }
        }
        s.golden_caught = self.golden_caught;
        s.black_caught = self.black_caught;
        s.crunches = self.crunches;
        s.dark_matter_spent = self.dark_matter_spent;
        for key in &self.lab {
            if let Some(id) = catalog.lab_by_key(key) {
                s.lab[id.index()] = true;
            }
        }
        for key in &self.lab_paused {
            if let Some(id) = catalog.lab_by_key(key) {
                s.lab_paused[id.index()] = true;
            }
        }
        s.universe = catalog.universe_by_key(&self.universe).unwrap_or_default();
        for key in &self.universe_keys {
            if let Some(id) = catalog.universe_by_key(key) {
                s.universe_keys[id.index()] = true;
            }
        }
        s.run_started = self.run_started.min(self.play_time);
        s.big_bangs = self.big_bangs;
        s.particles = self.particles;
        s.particles_spent = self.particles_spent;
        s.forged.resize(catalog.generators.len(), false);
        for &i in &self.forged {
            if let Some(f) = s.forged.get_mut(i) {
                *f = true;
            }
        }
        s.attractor = self.attractor;
        s.map_seed = self.map_seed;
        if s.map_seed != 0 {
            s.map = domain::starmap::generate(s.map_seed);
            s.explored = vec![false; s.map.systems.len()];
            for &i in &self.explored {
                if let Some(e) = s.explored.get_mut(i) {
                    *e = true;
                }
            }
        }
        s.fleets = self
            .fleets
            .iter()
            .enumerate()
            .map(|(i, &(target, left, total, success))| {
                let (loot, prospect) = self.fleets_loot.get(i).copied().unwrap_or((0.5, false));
                domain::state::Expedition {
                    target,
                    left,
                    total,
                    success,
                    loot,
                    prospect,
                }
            })
            .collect();
        s.relics.resize(catalog.relics.len(), None);
        for (key, rarity) in &self.relics {
            if let Some(id) = catalog.relic_by_key(key) {
                s.relics[id.index()] = Some(*rarity);
            }
        }
        s.equipped = self
            .equipped
            .iter()
            .filter_map(|k| catalog.relic_by_key(k))
            .collect();
        s.shards = self.shards;
        s.event_timer = self.event_timer;
        s.pending_event = catalog
            .events
            .iter()
            .find(|e| e.key == self.pending_event)
            .map(|e| (e.index, self.pending_left));
        s.boosts = self
            .boosts
            .iter()
            .map(|&(kind, factor, left)| domain::Boost {
                kind: if kind == 1 {
                    domain::BoostKind::Fragments
                } else {
                    domain::BoostKind::Production
                },
                factor,
                left,
            })
            .collect();
        s.events_resolved = self.events_resolved;
        s.convergence = (self.convergence_left > 0.0).then_some(self.convergence_left);
        s.converged = self.converged;
        s.auto_fleets = self.auto_fleets;
        s.history = self.history.clone();
        s.best_production = self.best_production;
        for (m, v) in s.milestones.iter_mut().zip(&self.milestones) {
            *m = *v;
        }
        s.expeditions_launched = self.expeditions_launched;
        // 0 (valeur par défaut) ou négatif : aucun chantier en cours.
        s.attractor_build = (self.attractor_left > 0.0).then_some(self.attractor_left);
        s.research = catalog
            .lab_by_key(&self.research)
            .map(|id| (id, self.research_left.max(0.0)));
        s.normalize(catalog);
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let c = Catalog::standard();
        let mut s = GameState::new(&c);
        s.stardust = Stardust::new(1234.5);
        s.total_run = 5000.0;
        s.total_all = 9000.0;
        s.owned[0] = 12;
        s.owned[3] = 2;
        s.upgrades[c.upgrade_by_key("gen0_t0").unwrap().index()] = true;
        s.achievements[c.achievement_by_key("first").unwrap().index()] = true;
        s.fragments = 3.0;
        s.supernovas = 1;
        s.clicks = 42;
        s.meteors_caught = 2;
        s.play_time = 99.0;
        let dto = SaveData::from_state(&s, &c, 1_700_000_000, true);
        assert_eq!(dto.upgrades, ["gen0_t0"]);
        assert_eq!(dto.achievements, ["first"]);
        assert!(dto.reduced_effects);
        let back = dto.to_state(&c);
        assert_eq!(back.stardust, s.stardust);
        assert_eq!(back.owned, s.owned);
        assert_eq!(back.upgrades, s.upgrades);
        assert_eq!(back.achievements, s.achievements);
        assert_eq!(back.fragments, 3.0);
        assert_eq!(back.clicks, 42);
    }

    #[test]
    fn tolerant_to_unknown_or_missing_data() {
        let c = Catalog::standard();
        let dto = SaveData {
            owned: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            upgrades: vec!["n_existe_pas".into()],
            stardust: f64::NAN,
            ..SaveData::default()
        };
        let s = dto.to_state(&c);
        assert_eq!(s.owned.len(), 15);
        assert_eq!(s.stardust.value(), 0.0);
        assert!(s.upgrades.iter().all(|&u| !u));
    }

    #[test]
    fn big_bang_and_attractor_round_trip() {
        let c = Catalog::standard();
        let mut st = GameState::new(&c);
        st.big_bangs = 2;
        st.particles = 7;
        st.particles_spent = 3;
        st.forged[12] = true;
        st.attractor = 2;
        st.attractor_build = Some(1234.0);
        let back = SaveData::from_state(&st, &c, 0, false).to_state(&c);
        assert_eq!(back.big_bangs, 2);
        assert_eq!(back.particles, 7);
        assert_eq!(back.particles_spent, 3);
        assert!(back.forged[12] && !back.forged[13]);
        assert_eq!(back.attractor, 2);
        assert_eq!(back.attractor_build, Some(1234.0));
        // Sans chantier : rien n'est inventé au rechargement.
        st.attractor_build = None;
        let back = SaveData::from_state(&st, &c, 0, false).to_state(&c);
        assert_eq!(back.attractor_build, None);
        assert_eq!(SaveData::default().to_state(&c).attractor_build, None);
    }
}
