use domain::{ChallengeId, ConstellationId, GeneratorId, LabId, PerkId, UpgradeId};

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
    /// Un niveau de Constellation.
    BuyConstellation(ConstellationId),
    /// Lance un Défi (relance le cycle).
    StartChallenge(ChallengeId),
    /// Abandonne le Défi en cours.
    AbandonChallenge,
    /// Big Crunch : remise à zéro du niveau Supernova contre de la matière noire.
    BigCrunch,
    /// Lance une recherche au Laboratoire.
    StartResearch(LabId),
    /// Met en pause / relance un automatisme du Laboratoire.
    ToggleLab(LabId),
}
