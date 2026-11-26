//! Ports : ce dont l'application a besoin du monde extérieur.
//! Implémentés par l'infrastructure (et par des fakes dans les tests).

use std::fmt;

use crate::save_dto::SaveData;

/// Résultat d'un chargement de sauvegarde.
#[derive(Clone, Debug, PartialEq)]
pub enum LoadOutcome {
    /// En `Box` : la sauvegarde est bien plus grosse que les autres variantes.
    Loaded(Box<SaveData>),
    /// Aucune sauvegarde : nouvelle partie.
    NotFound,
    /// Sauvegarde illisible, mise de côté dans `backup`.
    Corrupted { backup: String },
    /// Sauvegarde modifiée à la main (signature invalide), mise de côté.
    Tampered { backup: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryError(pub String);

impl fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RepositoryError {}

pub trait SaveRepository {
    fn load(&self) -> Result<LoadOutcome, RepositoryError>;
    fn save(&self, data: &SaveData) -> Result<(), RepositoryError>;
    /// Emplacement lisible par un humain (affiché dans l'aide).
    fn location(&self) -> String;
    /// Garde une copie de la sauvegarde actuelle avant qu'elle soit remplacée
    /// par celle d'un autre appareil.
    fn keep_backup(&self) -> Result<(), RepositoryError> {
        Ok(())
    }
}

pub trait Clock {
    /// Secondes écoulées depuis l'époque Unix.
    fn now_unix_secs(&self) -> u64;
}

pub trait RandomSource {
    /// Tirage uniforme dans [0, 1).
    fn next_f64(&mut self) -> f64;
}
