//! Sauvegarde JSON sur disque : écriture atomique (`.tmp` puis renommage),
//! sauvegarde corrompue mise de côté en `.bak`.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use application::{LoadOutcome, RepositoryError, SaveData, SaveRepository};
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
    reduced_effects: bool,
    perks: Vec<String>,
    fragments_spent: f64,
    autobuy: bool,
    /// Niveaux de Constellations : { "const_forge": 3, … }.
    constellations: BTreeMap<String, u32>,
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
            reduced_effects: d.reduced_effects,
            perks: d.perks.clone(),
            fragments_spent: d.fragments_spent,
            autobuy: d.autobuy,
            constellations: d.constellations.iter().cloned().collect(),
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
            reduced_effects: f.reduced_effects,
            perks: f.perks,
            fragments_spent: f.fragments_spent,
            autobuy: f.autobuy,
            constellations: f.constellations.into_iter().collect(),
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
        match serde_json::from_str::<SaveFile>(&text) {
            Ok(file) => Ok(LoadOutcome::Loaded(file.into())),
            Err(_) => {
                let backup = self.backup_path();
                fs::rename(&self.path, &backup).map_err(|e| io_err("mise de côté", &e))?;
                Ok(LoadOutcome::Corrupted {
                    backup: backup.display().to_string(),
                })
            }
        }
    }

    fn save(&self, data: &SaveData) -> Result<(), RepositoryError> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).map_err(|e| io_err("création du dossier", &e))?;
        }
        let json = serde_json::to_string_pretty(&SaveFile::from(data))
            .map_err(|e| RepositoryError(format!("sérialisation impossible : {e}")))?;
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
            version: 1,
            stardust: 12.5,
            owned: vec![3, 1],
            upgrades: vec!["gen0_t0".into()],
            last_saved: 99,
            reduced_effects: true,
            ..SaveData::default()
        };
        repo.save(&data).unwrap();
        assert!(!repo.sibling(".tmp").exists(), "le .tmp est renommé");
        assert_eq!(repo.load().unwrap(), LoadOutcome::Loaded(data));
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
}
