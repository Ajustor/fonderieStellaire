//! Couche infrastructure : la seule qui touche au disque, à l'horloge système
//! et à `serde`. Implémente les ports de la couche application.

#[cfg(feature = "audio")]
pub mod audio_output;
#[cfg(feature = "updater")]
pub mod github_updater;
pub mod json_repository;
#[cfg(feature = "sync")]
pub mod relay_leaderboard;
#[cfg(feature = "sync")]
pub mod relay_sync;
pub mod save_crypto;
pub mod system_clock;
pub mod xorshift;

#[cfg(feature = "audio")]
pub use audio_output::AudioOutput;
#[cfg(feature = "updater")]
pub use github_updater::{ApkReleases, GitHubUpdater};
pub use json_repository::JsonSaveRepository;
#[cfg(feature = "sync")]
pub use relay_leaderboard::RelayLeaderboard;
#[cfg(feature = "sync")]
pub use relay_sync::RelaySync;
pub use system_clock::SystemClock;
pub use xorshift::XorShift64;
