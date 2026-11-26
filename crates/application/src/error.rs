use std::fmt;

use crate::ports::RepositoryError;

/// Erreurs remontées par la couche application.
#[derive(Debug)]
pub enum AppError {
    Repository(RepositoryError),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Repository(e) => write!(f, "sauvegarde : {e}"),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Repository(e) => Some(e),
        }
    }
}

impl From<RepositoryError> for AppError {
    fn from(e: RepositoryError) -> Self {
        AppError::Repository(e)
    }
}
