use domain::{ChallengeId, ConstellationId, GeneratorId, LabId, PerkId, UniverseId, UpgradeId};

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
    /// Big Crunch vers un univers (l'origine tant que les univers sont fermés).
    BigCrunch(UniverseId),
    /// Lance une recherche au Laboratoire.
    StartResearch(LabId),
    /// Met en pause / relance un automatisme du Laboratoire.
    ToggleLab(LabId),
    /// Big Bang : les Big Crunch repartent de zéro contre des particules.
    BigBang,
    /// Forge un générateur primordial.
    Forge(GeneratorId),
    /// Lance la prochaine étape du Grand Attracteur.
    BuildAttractor,
    /// Images par seconde : 30 → 60 → 20 → 15 → 30.
    CycleFrameRate,
    /// Efface la partie et repart de zéro (les réglages sont gardés).
    NewGame,
    /// Envoie une flotte explorer un système de la Carte stellaire.
    LaunchExpedition(usize),
    /// Équipe ou range une relique.
    ToggleRelic(domain::RelicId),
    /// Répond à l'événement cosmique en attente (index du choix).
    ResolveEvent(usize),
    /// Lance la Convergence.
    StartConvergence,
    /// Active / coupe le pilote automatique des flottes.
    ToggleAutoFleets,
    /// Couleurs : normales → daltonien → contraste élevé.
    CycleColorMode,
    /// Bâtit une mégastructure sur une case.
    PlaceStructure(usize, domain::StructureKind),
    /// Démolit la mégastructure d'une case.
    Demolish(usize),
}
