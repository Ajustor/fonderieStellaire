use domain::{GeneratorId, PerkId, UpgradeId};

/// Intentions de jeu envoyées par la présentation au [`crate::GameService`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Harvest,
    /// Récolte issue d'une touche maintenue (nécessite « Récolte continue »).
    HarvestHeld,
    BuyGenerator(GeneratorId),
    BuyUpgrade(UpgradeId),
    CycleBuyMode,
    CatchComet,
    Supernova,
    Save,
    ToggleReducedEffects,
    /// Achète toutes les améliorations abordables.
    BuyAllUpgrades,
    /// Achat à l'Atelier stellaire.
    BuyPerk(PerkId),
    /// Active / arrête l'automate d'achat.
    ToggleAutoBuy,
}
