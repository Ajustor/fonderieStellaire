use domain::{GeneratorId, UpgradeId};

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
}
