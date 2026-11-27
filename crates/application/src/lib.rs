//! Couche application : cas d'usage et ports.
//!
//! Orchestre le domaine, ne connaît ni le terminal ni le système de fichiers :
//! elle ne parle au monde extérieur qu'à travers les traits de [`ports`].

pub mod commands;
pub mod error;
pub mod game_service;
pub mod leaderboard;
pub mod ports;
pub mod save_dto;
pub mod simulator;
pub mod snapshot;
pub mod sync;
pub mod updates;

pub use commands::Command;
pub use error::AppError;
pub use game_service::{GameEvent, GameService, AUTOSAVE_INTERVAL, HISTORY_LEN};
pub use leaderboard::{
    Board, Entry, LbError, LeaderboardBackend, LeaderboardEvent, LeaderboardService, ScoreCard,
    Standings,
};
pub use ports::{Clock, LoadOutcome, RandomSource, RepositoryError, SaveRepository};
pub use save_dto::{SaveData, SAVE_RESET_BELOW, SAVE_VERSION};
pub use snapshot::{
    ActiveChallengeView, ChallengeView, ConstellationView, DailyView, FleetView, ForgeView,
    GameSnapshot, GeneratorView, LabView, NextUpgradeView, PerkView, ProgressView,
    ResearchBuildView, ResearchView, SystemView, TeaserView, UniverseView, UpgradeView,
};
pub use sync::{
    PushOutcome, RemoteSave, SyncBackend, SyncError, SyncEvent, SyncService, SyncState,
};
pub use updates::{ReleaseInfo, UpdateError, UpdateEvent, UpdateProvider, UpdateService};
