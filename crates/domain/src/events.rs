//! Événements émis par le domaine. La présentation les traduit en effets visuels
//! et en lignes de journal ; le domaine ne sait rien des animations.

use crate::achievement::AchievementId;
use crate::comet::CometReward;
use crate::constellation::ConstellationId;
use crate::generator::GeneratorId;
use crate::perk::PerkId;
use crate::upgrade::UpgradeId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PurchaseTarget {
    Generator(GeneratorId),
    Upgrade(UpgradeId),
    /// Amélioration de l'Atelier stellaire.
    Perk(PerkId),
    /// Niveau de Constellation.
    Constellation(ConstellationId),
}

#[derive(Clone, Debug, PartialEq)]
pub enum DomainEvent {
    /// Récolte manuelle.
    Harvested {
        amount: f64,
    },
    /// Achat de générateurs réussi.
    Purchased {
        generator: GeneratorId,
        count: u64,
        cost: f64,
    },
    /// Achat d'amélioration réussi.
    UpgradePurchased {
        upgrade: UpgradeId,
        cost: f64,
    },
    /// Récolte « touche maintenue » tentée sans l'amélioration « Récolte continue ».
    HoldHarvestLocked,
    /// Achat refusé (pas assez de poussière, ou élément verrouillé).
    PurchaseFailed {
        target: PurchaseTarget,
    },
    /// Un générateur a atteint un palier (production ×2).
    MilestoneReached {
        generator: GeneratorId,
        count: u64,
    },
    /// Un nouveau générateur devient visible.
    GeneratorRevealed {
        generator: GeneratorId,
    },
    AchievementUnlocked {
        achievement: AchievementId,
    },
    CometSpawned,
    /// La comète a fini sa traversée sans être capturée.
    CometMissed,
    CometCaught {
        reward: CometReward,
    },
    SolarStormEnded,
    Supernova {
        fragments_gained: f64,
        total_fragments: f64,
    },
    /// Supernova demandée alors qu'aucun fragment n'est gagnable.
    SupernovaUnavailable,
    /// « Tout acheter » : nombre d'améliorations achetées (0 = rien d'abordable).
    BulkUpgrades {
        count: usize,
    },
    /// Amélioration de l'Atelier achetée avec des fragments.
    PerkPurchased {
        perk: PerkId,
    },
    /// L'automate d'achat a été activé ou arrêté.
    AutoBuyChanged {
        enabled: bool,
    },
    /// Automate demandé sans l'avoir acheté à l'Atelier.
    AutoBuyLocked,
    /// Une Constellation gagne un niveau.
    ConstellationUpgraded {
        constellation: ConstellationId,
        level: u32,
    },
    OfflineGains {
        seconds: f64,
        amount: f64,
    },
}
