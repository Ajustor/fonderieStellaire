//! DTO de sauvegarde, en Rust pur (la sérialisation vit dans l'infrastructure).

use domain::{Catalog, GameState, Stardust};

pub const SAVE_VERSION: u32 = 1;

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
    /// Identifiants texte des améliorations de l'Atelier achetées.
    pub perks: Vec<String>,
    pub fragments_spent: f64,
    /// Automate d'achat activé.
    pub autobuy: bool,
    /// Niveaux de Constellations (clé texte, niveau).
    pub constellations: Vec<(String, u32)>,
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
        assert_eq!(s.owned.len(), 12);
        assert_eq!(s.stardust.value(), 0.0);
        assert!(s.upgrades.iter().all(|&u| !u));
    }
}
