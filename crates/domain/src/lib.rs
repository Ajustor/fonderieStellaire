//! Couche domaine : règles du jeu pures et déterministes.
//!
//! Aucune dépendance externe, aucune I/O, aucune horloge, aucun aléa implicite :
//! le temps écoulé (`dt`) et les tirages aléatoires arrivent en paramètre.

pub mod achievement;
pub mod catalog;
pub mod challenge;
pub mod comet;
pub mod constellation;
pub mod cosmos;
pub mod economy;
pub mod events;
pub mod generator;
pub mod lab;
pub mod perk;
pub mod prestige;
pub mod stardust;
pub mod state;
pub mod universe;
pub mod upgrade;

pub use achievement::{AchievementCondition, AchievementContext, AchievementDef, AchievementId};
pub use catalog::Catalog;
pub use challenge::{ChallengeDef, ChallengeId, ChallengeReward, Restriction};
pub use comet::{CometKind, CometReward, CometState};
pub use constellation::{ConstellationDef, ConstellationEffect, ConstellationId};
pub use cosmos::{AttractorEffect, AttractorStage};
pub use events::{DomainEvent, PurchaseTarget};
pub use generator::{BuyMode, GeneratorDef, GeneratorId};
pub use lab::{LabDef, LabEffect, LabId};
pub use perk::{PerkDef, PerkEffect, PerkId};
pub use stardust::Stardust;
pub use state::GameState;
pub use universe::{KeyReward, UniverseDef, UniverseId, UniverseRules};
pub use upgrade::{Effect, Requirement, UpgradeDef, UpgradeId};
