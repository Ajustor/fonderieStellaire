//! Couche infrastructure : la seule qui touche au disque, à l'horloge système
//! et à `serde`. Implémente les ports de la couche application.

pub mod github_updater;
pub mod json_repository;
pub mod system_clock;
pub mod xorshift;

pub use github_updater::GitHubUpdater;
pub use json_repository::JsonSaveRepository;
pub use system_clock::SystemClock;
pub use xorshift::XorShift64;
