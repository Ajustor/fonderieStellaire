//! Sauvegarde JSON sur disque : écriture atomique (`.tmp` puis renommage),
//! sauvegarde corrompue mise de côté en `.bak`.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use std::time::{SystemTime, UNIX_EPOCH};

use application::{LoadOutcome, RepositoryError, SaveData, SaveRepository};

use crate::save_crypto::{self, CryptoError};
use serde::{Deserialize, Serialize};

const APP_DIR: &str = "fonderie-stellaire";
const FILE_NAME: &str = "save.json";

/// Format sur disque. `#[serde(default)]` partout pour relire les anciennes versions.
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct SaveFile {
    version: u32,
    stardust: f64,
    total_run: f64,
    total_all: f64,
    owned: Vec<u64>,
    upgrades: Vec<String>,
    achievements: Vec<String>,
    fragments: f64,
    fragments_this_run: f64,
    supernovas: u32,
    clicks: u64,
    meteors_caught: u32,
    play_time: f64,
    last_saved: u64,
    started_at: u64,
    reduced_effects: bool,
    frame_rate: u32,
    perks: Vec<String>,
    fragments_spent: f64,
    autobuy: bool,
    /// Niveaux de Constellations : { "const_forge": 3, … }.
    constellations: BTreeMap<String, u32>,
    challenge: String,
    challenge_started: f64,
    challenges_done: Vec<String>,
    golden_caught: u32,
    black_caught: u32,
    crunches: u32,
    dark_matter_spent: u32,
    lab: Vec<String>,
    lab_paused: Vec<String>,
    research: String,
    research_left: f64,
    universe: String,
    universe_keys: Vec<String>,
    run_started: f64,
    big_bangs: u32,
    particles: u32,
    particles_spent: u32,
    forged: Vec<usize>,
    attractor: usize,
    attractor_left: f64,
    map_seed: u64,
    explored: Vec<usize>,
    fleets: Vec<(usize, f64, f64, bool)>,
    expeditions_launched: u32,
    fleets_loot: Vec<(f64, bool)>,
    relics: Vec<(String, u8)>,
    equipped: Vec<String>,
    shards: u32,
    event_timer: f64,
    pending_event: String,
    pending_left: f64,
    boosts: Vec<(u8, f64, f64)>,
    events_resolved: u32,
    convergence_left: f64,
    converged: bool,
    auto_fleets: bool,
    history: Vec<(f64, f32)>,
    best_production: f64,
    milestones: Vec<f64>,
    color_mode: u8,
    exotic: f64,
    exotic_total: f64,
    grid: Vec<i8>,
    daily_day: u64,
    daily_best: f64,
    daily_rewarded: bool,
    awakening_left: f64,
    awakened: bool,
}

impl From<&SaveData> for SaveFile {
    fn from(d: &SaveData) -> Self {
        SaveFile {
            version: d.version,
            stardust: d.stardust,
            total_run: d.total_run,
            total_all: d.total_all,
            owned: d.owned.clone(),
            upgrades: d.upgrades.clone(),
            achievements: d.achievements.clone(),
            fragments: d.fragments,
            fragments_this_run: d.fragments_this_run,
            supernovas: d.supernovas,
            clicks: d.clicks,
            meteors_caught: d.meteors_caught,
            play_time: d.play_time,
            last_saved: d.last_saved,
            started_at: d.started_at,
            reduced_effects: d.reduced_effects,
            frame_rate: d.frame_rate,
            perks: d.perks.clone(),
            fragments_spent: d.fragments_spent,
            autobuy: d.autobuy,
            constellations: d.constellations.iter().cloned().collect(),
            challenge: d.challenge.clone(),
            challenge_started: d.challenge_started,
            challenges_done: d.challenges_done.clone(),
            golden_caught: d.golden_caught,
            black_caught: d.black_caught,
            crunches: d.crunches,
            dark_matter_spent: d.dark_matter_spent,
            lab: d.lab.clone(),
            lab_paused: d.lab_paused.clone(),
            research: d.research.clone(),
            research_left: d.research_left,
            universe: d.universe.clone(),
            universe_keys: d.universe_keys.clone(),
            run_started: d.run_started,
            big_bangs: d.big_bangs,
            particles: d.particles,
            particles_spent: d.particles_spent,
            forged: d.forged.clone(),
            attractor: d.attractor,
            attractor_left: d.attractor_left,
            map_seed: d.map_seed,
            explored: d.explored.clone(),
            fleets: d.fleets.clone(),
            expeditions_launched: d.expeditions_launched,
            fleets_loot: d.fleets_loot.clone(),
            relics: d.relics.clone(),
            equipped: d.equipped.clone(),
            shards: d.shards,
            event_timer: d.event_timer,
            pending_event: d.pending_event.clone(),
            pending_left: d.pending_left,
            boosts: d.boosts.clone(),
            events_resolved: d.events_resolved,
            convergence_left: d.convergence_left,
            converged: d.converged,
            auto_fleets: d.auto_fleets,
            history: d.history.clone(),
            best_production: d.best_production,
            milestones: d.milestones.clone(),
            color_mode: d.color_mode,
            exotic: d.exotic,
            exotic_total: d.exotic_total,
            grid: d.grid.clone(),
            daily_day: d.daily_day,
            daily_best: d.daily_best,
            daily_rewarded: d.daily_rewarded,
            awakening_left: d.awakening_left,
            awakened: d.awakened,
        }
    }
}

impl From<SaveFile> for SaveData {
    fn from(f: SaveFile) -> Self {
        SaveData {
            version: f.version,
            stardust: f.stardust,
            total_run: f.total_run,
            total_all: f.total_all,
            owned: f.owned,
            upgrades: f.upgrades,
            achievements: f.achievements,
            fragments: f.fragments,
            fragments_this_run: f.fragments_this_run,
            supernovas: f.supernovas,
            clicks: f.clicks,
            meteors_caught: f.meteors_caught,
            play_time: f.play_time,
            last_saved: f.last_saved,
            started_at: f.started_at,
            reduced_effects: f.reduced_effects,
            frame_rate: f.frame_rate,
            perks: f.perks,
            fragments_spent: f.fragments_spent,
            autobuy: f.autobuy,
            constellations: f.constellations.into_iter().collect(),
            challenge: f.challenge,
            challenge_started: f.challenge_started,
            challenges_done: f.challenges_done,
            golden_caught: f.golden_caught,
            black_caught: f.black_caught,
            crunches: f.crunches,
            dark_matter_spent: f.dark_matter_spent,
            lab: f.lab,
            lab_paused: f.lab_paused,
            research: f.research,
            research_left: f.research_left,
            universe: f.universe,
            universe_keys: f.universe_keys,
            run_started: f.run_started,
            big_bangs: f.big_bangs,
            particles: f.particles,
            particles_spent: f.particles_spent,
            forged: f.forged,
            attractor: f.attractor,
            attractor_left: f.attractor_left,
            map_seed: f.map_seed,
            explored: f.explored,
            fleets: f.fleets,
            expeditions_launched: f.expeditions_launched,
            fleets_loot: f.fleets_loot,
            relics: f.relics,
            equipped: f.equipped,
            shards: f.shards,
            event_timer: f.event_timer,
            pending_event: f.pending_event,
            pending_left: f.pending_left,
            boosts: f.boosts,
            events_resolved: f.events_resolved,
            convergence_left: f.convergence_left,
            converged: f.converged,
            auto_fleets: f.auto_fleets,
            history: f.history,
            best_production: f.best_production,
            milestones: f.milestones,
            color_mode: f.color_mode,
            exotic: f.exotic,
            exotic_total: f.exotic_total,
            grid: f.grid,
            daily_day: f.daily_day,
            daily_best: f.daily_best,
            daily_rewarded: f.daily_rewarded,
            awakening_left: f.awakening_left,
            awakened: f.awakened,
        }
    }
}

#[derive(Clone, Debug)]
pub struct JsonSaveRepository {
    path: PathBuf,
}

impl JsonSaveRepository {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        JsonSaveRepository { path: path.into() }
    }

    /// `dirs::data_dir()/fonderie-stellaire/save.json`
    /// (`%APPDATA%` sous Windows, `~/.local/share` sous Linux).
    pub fn default_location() -> Result<Self, RepositoryError> {
        let base = dirs::data_dir().ok_or_else(|| {
            RepositoryError("impossible de trouver le dossier de données utilisateur".into())
        })?;
        Ok(Self::new(base.join(APP_DIR).join(FILE_NAME)))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn sibling(&self, suffix: &str) -> PathBuf {
        let mut name = self.path.as_os_str().to_owned();
        name.push(suffix);
        PathBuf::from(name)
    }

    pub fn backup_path(&self) -> PathBuf {
        self.sibling(".bak")
    }

    /// Copie la sauvegarde (vérifiée lisible) dans `dest`.
    pub fn export_to(&self, dest: &Path) -> Result<(), RepositoryError> {
        match self.load()? {
            LoadOutcome::Loaded(_) => fs::copy(&self.path, dest)
                .map(|_| ())
                .map_err(|e| io_err(&format!("copie vers {}", dest.display()), &e)),
            LoadOutcome::NotFound => Err(RepositoryError("aucune sauvegarde à exporter".into())),
            LoadOutcome::Corrupted { backup } => Err(RepositoryError(format!(
                "sauvegarde illisible (mise de côté dans {backup})"
            ))),
            LoadOutcome::Tampered { backup } => Err(RepositoryError(format!(
                "sauvegarde modifiée à la main (mise de côté dans {backup})"
            ))),
        }
    }

    /// Reprend une sauvegarde exportée : le fichier est vérifié, puis la
    /// sauvegarde actuelle passe en `.bak`. Renvoie `true` si elle existait.
    pub fn import_from(&self, src: &Path) -> Result<bool, RepositoryError> {
        let data = match JsonSaveRepository::new(src).load()? {
            LoadOutcome::Loaded(d) => d,
            LoadOutcome::NotFound => {
                return Err(RepositoryError(format!("{} introuvable", src.display())))
            }
            LoadOutcome::Corrupted { .. } => {
                return Err(RepositoryError(format!(
                    "{} n'est pas une sauvegarde valide",
                    src.display()
                )))
            }
            LoadOutcome::Tampered { .. } => {
                return Err(RepositoryError(format!(
                    "{} a été modifiée à la main : refusée",
                    src.display()
                )))
            }
        };
        let backed_up = self.reset()?;
        self.save(&data)?;
        Ok(backed_up)
    }

    /// Contenu de la sauvegarde (vérifiée lisible), pour la partager (Android).
    pub fn export_bytes(&self) -> Result<Vec<u8>, RepositoryError> {
        let tmp = self.sibling(".export");
        self.export_to(&tmp)?;
        let bytes = fs::read(&tmp).map_err(|e| io_err("lecture de l'export", &e));
        let _ = fs::remove_file(&tmp);
        bytes
    }

    /// Comme `import_from`, à partir du contenu d'un fichier exporté.
    pub fn import_bytes(&self, bytes: &[u8]) -> Result<bool, RepositoryError> {
        let tmp = self.sibling(".import");
        fs::write(&tmp, bytes).map_err(|e| io_err("écriture de l'import", &e))?;
        let result = self.import_from(&tmp);
        let _ = fs::remove_file(&tmp);
        result
    }

    /// Met la sauvegarde existante de côté (`.bak`) pour repartir de zéro.
    /// Renvoie `true` si un fichier a été déplacé.
    pub fn reset(&self) -> Result<bool, RepositoryError> {
        if !self.path.exists() {
            return Ok(false);
        }
        fs::rename(&self.path, self.backup_path()).map_err(|e| io_err("mise de côté", &e))?;
        Ok(true)
    }
}

fn io_err(action: &str, e: &io::Error) -> RepositoryError {
    RepositoryError(format!("{action} impossible : {e}"))
}

impl SaveRepository for JsonSaveRepository {
    fn load(&self) -> Result<LoadOutcome, RepositoryError> {
        let text = match fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(LoadOutcome::NotFound),
            Err(e) if e.kind() == io::ErrorKind::InvalidData => String::new(),
            Err(e) => return Err(io_err("lecture", &e)),
        };
        match decode_save(&text) {
            Ok(data) => Ok(LoadOutcome::Loaded(Box::new(data))),
            Err(e) => {
                let backup = self.backup_path();
                fs::rename(&self.path, &backup).map_err(|e| io_err("mise de côté", &e))?;
                let backup = backup.display().to_string();
                Ok(match e {
                    DecodeError::Tampered => LoadOutcome::Tampered { backup },
                    DecodeError::Unreadable => LoadOutcome::Corrupted { backup },
                })
            }
        }
    }

    fn save(&self, data: &SaveData) -> Result<(), RepositoryError> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).map_err(|e| io_err("création du dossier", &e))?;
        }
        let json = encode_save(data)?;
        let tmp = self.sibling(".tmp");
        {
            let mut f = fs::File::create(&tmp).map_err(|e| io_err("écriture", &e))?;
            f.write_all(json.as_bytes())
                .map_err(|e| io_err("écriture", &e))?;
            f.sync_all().map_err(|e| io_err("écriture", &e))?;
        }
        fs::rename(&tmp, &self.path).map_err(|e| io_err("renommage", &e))?;
        Ok(())
    }

    fn location(&self) -> String {
        self.path.display().to_string()
    }

    fn keep_backup(&self) -> Result<(), RepositoryError> {
        match fs::copy(&self.path, self.backup_path()) {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(io_err("copie de sauvegarde", &e)),
        }
    }
}

/// Sauvegarde illisible : modifiée à la main, ou pas une sauvegarde.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    Tampered,
    Unreadable,
}

/// Texte d'une sauvegarde : JSON chiffré et signé (une retouche à la main est
/// détectée au chargement).
pub fn encode_save(data: &SaveData) -> Result<String, RepositoryError> {
    let json = serde_json::to_string(&SaveFile::from(data))
        .map_err(|e| RepositoryError(format!("sérialisation impossible : {e}")))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
        ^ data.last_saved.rotate_left(17);
    Ok(save_crypto::encode(&json, nonce))
}

/// Relit une sauvegarde chiffrée (depuis la v0.12) ou une ancienne en clair.
pub fn decode_save(text: &str) -> Result<SaveData, DecodeError> {
    let json = match save_crypto::decode(text) {
        Ok(json) => json,
        Err(CryptoError::NotEncrypted) => text.to_string(),
        Err(CryptoError::Tampered) => return Err(DecodeError::Tampered),
    };
    serde_json::from_str::<SaveFile>(&json)
        .map(Into::into)
        .map_err(|_| DecodeError::Unreadable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_repo(tag: &str) -> (PathBuf, JsonSaveRepository) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("fonderie-test-{tag}-{nanos}"));
        let repo = JsonSaveRepository::new(dir.join("save.json"));
        (dir, repo)
    }

    #[test]
    fn missing_file_is_new_game() {
        let (_dir, repo) = temp_repo("missing");
        assert_eq!(repo.load().unwrap(), LoadOutcome::NotFound);
    }

    #[test]
    fn save_then_load_round_trip() {
        let (dir, repo) = temp_repo("roundtrip");
        let data = SaveData {
            version: application::SAVE_VERSION,
            stardust: 12.5,
            owned: vec![3, 1],
            upgrades: vec!["gen0_t0".into()],
            last_saved: 99,
            started_at: 1_700_000_000,
            reduced_effects: true,
            ..SaveData::default()
        };
        repo.save(&data).unwrap();
        assert!(!repo.sibling(".tmp").exists(), "le .tmp est renommé");
        assert_eq!(repo.load().unwrap(), LoadOutcome::Loaded(Box::new(data)));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn old_saves_with_missing_fields_load() {
        let (dir, repo) = temp_repo("old");
        fs::create_dir_all(&dir).unwrap();
        fs::write(repo.path(), r#"{"stardust": 5.0, "clicks": 3}"#).unwrap();
        match repo.load().unwrap() {
            LoadOutcome::Loaded(d) => {
                assert_eq!(d.stardust, 5.0);
                assert_eq!(d.clicks, 3);
                assert!(d.owned.is_empty());
            }
            other => panic!("{other:?}"),
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn corrupted_file_moved_to_bak() {
        let (dir, repo) = temp_repo("corrupt");
        fs::create_dir_all(&dir).unwrap();
        fs::write(repo.path(), "{ pas du json").unwrap();
        assert!(matches!(
            repo.load().unwrap(),
            LoadOutcome::Corrupted { .. }
        ));
        assert!(!repo.path().exists());
        assert!(repo.backup_path().exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn reset_moves_save_aside() {
        let (dir, repo) = temp_repo("reset");
        assert!(!repo.reset().unwrap());
        repo.save(&SaveData::default()).unwrap();
        assert!(repo.reset().unwrap());
        assert!(!repo.path().exists());
        assert!(repo.backup_path().exists());
        let _ = fs::remove_dir_all(dir);
    }

    /// Durée d'une sauvegarde réelle sur ce disque (antivirus compris).
    /// `cargo test --release -p infrastructure save_timing -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn save_timing() {
        let (dir, repo) = temp_repo("timing");
        let data = SaveData {
            version: application::SAVE_VERSION,
            owned: vec![250; 12],
            upgrades: (0..56).map(|i| format!("u{i}")).collect(),
            achievements: (0..30).map(|i| format!("a{i}")).collect(),
            ..SaveData::default()
        };
        let mut times = Vec::new();
        for _ in 0..20 {
            let t = std::time::Instant::now();
            repo.save(&data).unwrap();
            times.push(t.elapsed().as_secs_f64() * 1e3);
        }
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!(
            "sauvegarde : médiane {:.1} ms, pire {:.1} ms",
            times[10], times[19]
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn export_then_import_round_trip() {
        let (dir, repo) = temp_repo("export");
        let file = dir.join("copie.json");
        assert!(repo.export_to(&file).is_err(), "rien à exporter");
        let data = SaveData {
            version: application::SAVE_VERSION,
            clicks: 77,
            ..SaveData::default()
        };
        repo.save(&data).unwrap();
        repo.export_to(&file).unwrap();
        repo.save(&SaveData::default()).unwrap();
        assert!(repo.import_from(&file).unwrap(), "l'actuelle passe en .bak");
        match repo.load().unwrap() {
            LoadOutcome::Loaded(d) => assert_eq!(d.clicks, 77),
            other => panic!("{other:?}"),
        }
        assert!(repo.import_from(&dir.join("absent.json")).is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn export_then_import_bytes_round_trip() {
        let (dir, repo) = temp_repo("export-bytes");
        assert!(repo.export_bytes().is_err(), "rien à exporter");
        let data = SaveData {
            version: application::SAVE_VERSION,
            clicks: 42,
            ..SaveData::default()
        };
        repo.save(&data).unwrap();
        let bytes = repo.export_bytes().unwrap();
        repo.save(&SaveData::default()).unwrap();
        assert!(repo.import_bytes(&bytes).unwrap());
        match repo.load().unwrap() {
            LoadOutcome::Loaded(d) => assert_eq!(d.clicks, 42),
            other => panic!("{other:?}"),
        }
        assert!(repo.import_bytes(b"pas une sauvegarde").is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn saves_are_encrypted_and_edits_are_refused() {
        let (dir, repo) = temp_repo("crypto");
        let data = SaveData {
            version: application::SAVE_VERSION,
            fragments: 12.0,
            ..SaveData::default()
        };
        repo.save(&data).unwrap();
        let raw = fs::read_to_string(repo.path()).unwrap();
        assert!(raw.starts_with(save_crypto::MAGIC));
        assert!(!raw.contains("fragments"), "illisible en clair");
        // Une retouche : la sauvegarde est refusée et mise de côté.
        let mut bytes = raw.into_bytes();
        let i = bytes.len() / 2;
        bytes[i] = if bytes[i] == b'A' { b'B' } else { b'A' };
        fs::write(repo.path(), bytes).unwrap();
        match repo.load().unwrap() {
            LoadOutcome::Tampered { backup } => assert!(backup.ends_with(".bak")),
            other => panic!("{other:?}"),
        }
        assert!(!repo.path().exists());
        let _ = fs::remove_dir_all(dir);
    }
}
