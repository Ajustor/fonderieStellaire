//! Événements émis par le domaine. La présentation les traduit en effets visuels
//! et en lignes de journal ; le domaine ne sait rien des animations.

use crate::achievement::AchievementId;
use crate::challenge::{ChallengeId, Restriction};
use crate::comet::{CometKind, CometReward};
use crate::constellation::ConstellationId;
use crate::generator::GeneratorId;
use crate::lab::LabId;
use crate::perk::PerkId;
use crate::universe::UniverseId;
use crate::upgrade::UpgradeId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PurchaseTarget {
    Generator(GeneratorId),
    Upgrade(UpgradeId),
    /// Amélioration de l'Atelier stellaire.
    Perk(PerkId),
    /// Niveau de Constellation.
    Constellation(ConstellationId),
    /// Recherche du Laboratoire.
    Lab(LabId),
    /// Forge d'un générateur primordial.
    Forge(GeneratorId),
    /// Étape du Grand Attracteur.
    Attractor(usize),
    /// Expédition vers un système.
    Expedition(usize),
    /// Équiper une relique.
    Relic(crate::relic::RelicId),
    /// Lancer la Convergence.
    Convergence,
    /// Bâtir sur une case de la grille des Mégastructures.
    Structure(usize),
}

/// Ce que devient une relique trouvée.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelicOutcome {
    /// Nouvelle dans la collection.
    New,
    /// Déjà possédée, mais à une rareté plus basse : elle est améliorée.
    Upgraded,
    /// Doublon : changée en Éclat (+1 % de production).
    Shard,
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
    CometSpawned {
        kind: CometKind,
    },
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
    /// Une comète noire ratée a volé de la poussière.
    CometStole {
        amount: f64,
    },
    /// Un Défi commence (le cycle a été relancé).
    ChallengeStarted {
        challenge: ChallengeId,
    },
    /// Objectif atteint : la récompense est acquise pour toujours.
    ChallengeCompleted {
        challenge: ChallengeId,
    },
    /// Temps écoulé (Défi chronométré).
    ChallengeFailed {
        challenge: ChallengeId,
    },
    ChallengeAbandoned {
        challenge: ChallengeId,
    },
    /// Action interdite par le Défi en cours.
    ChallengeBlocked {
        restriction: Restriction,
    },
    /// Défi impossible à lancer (verrouillé, déjà réussi, ou un autre en cours).
    ChallengeUnavailable {
        challenge: ChallengeId,
    },
    /// Big Crunch : tout le niveau Supernova repart de zéro contre de la matière noire.
    BigCrunch {
        crunches: u32,
    },
    /// Nouvel univers après un Big Crunch.
    UniverseEntered {
        universe: UniverseId,
    },
    /// Objectif d'un univers atteint : sa Clé est acquise pour toujours.
    UniverseKeyEarned {
        universe: UniverseId,
    },
    /// Big Bang : les Big Crunch repartent de zéro contre des particules.
    BigBang {
        particles_gained: u32,
        total_particles: u32,
    },
    /// Big Bang demandé trop tôt.
    BigBangUnavailable {
        crunches: u32,
        needed: u32,
    },
    /// Un générateur primordial est forgé (pour toujours).
    GeneratorForged {
        generator: GeneratorId,
    },
    /// La construction d'une étape du Grand Attracteur commence.
    AttractorStageStarted {
        stage: usize,
    },
    /// Une étape du Grand Attracteur est achevée.
    AttractorStageDone {
        stage: usize,
    },
    /// La dernière étape est achevée : le Grand Attracteur s'éveille.
    AttractorAwakened,
    /// Acte III : la Carte stellaire s'ouvre (carte générée).
    StarMapOpened,
    /// Une flotte part en expédition.
    ExpeditionLaunched {
        system: usize,
    },
    /// Une flotte rentre ; en cas de succès, le système est exploré.
    ExpeditionReturned {
        system: usize,
        success: bool,
        /// Gain immédiat (Anomalie), 0 sinon.
        stardust: f64,
    },
    /// Tous les systèmes sont explorés.
    StarMapCompleted,
    /// Une relique rapportée d'expédition.
    RelicFound {
        relic: crate::relic::RelicId,
        rarity: u8,
        outcome: RelicOutcome,
    },
    /// Un événement cosmique se présente.
    CosmicEventStarted {
        event: usize,
    },
    /// Le joueur a choisi ; `success` vaut faux si le risque a mal tourné.
    CosmicEventResolved {
        event: usize,
        choice: usize,
        success: bool,
    },
    /// Personne n'a répondu à temps.
    CosmicEventExpired {
        event: usize,
    },
    /// La Convergence commence (48 h).
    ConvergenceStarted,
    /// Fin de l'Acte III.
    ConvergenceDone,
    /// Acte IV : les Mégastructures s'ouvrent.
    MegastructuresOpened,
    /// Une structure est bâtie.
    StructurePlaced {
        cell: usize,
        kind: crate::megastructure::StructureKind,
    },
    /// Une structure est démolie.
    StructureRemoved {
        cell: usize,
    },
    /// Pilote automatique des flottes activé ou coupé.
    AutoFleetsToggled {
        enabled: bool,
    },
    /// Relique équipée ou rangée.
    RelicToggled {
        relic: crate::relic::RelicId,
        equipped: bool,
    },
    /// Big Crunch demandé trop tôt.
    BigCrunchUnavailable {
        supernovas: u32,
        needed: u32,
    },
    /// Une recherche du Laboratoire commence (matière noire payée).
    LabResearchStarted {
        node: LabId,
    },
    /// Une recherche est terminée : son effet est actif.
    LabResearchDone {
        node: LabId,
    },
    /// Un automatisme du Laboratoire est mis en pause ou relancé.
    LabToggled {
        node: LabId,
        enabled: bool,
    },
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
