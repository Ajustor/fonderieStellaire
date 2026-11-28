//! Vue en lecture seule de l'état du jeu, destinée à la présentation.

use domain::{
    BuyMode, ChallengeId, CometKind, ConstellationId, GeneratorId, LabId, PerkId, UniverseId,
    UpgradeId,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeneratorView {
    pub id: GeneratorId,
    pub owned: u64,
    /// Production effective (multiplicateur global inclus).
    pub production: f64,
    /// Part de la production totale (0 → 1).
    pub share: f64,
    /// Quantité achetée selon le mode courant.
    pub quantity: u64,
    /// Coût de ce lot.
    pub cost: f64,
    pub affordable: bool,
}

/// Le premier générateur encore caché (ligne « ??? »).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TeaserView {
    pub id: GeneratorId,
    /// Poussière restant à récolter avant le dévoilement.
    pub remaining: f64,
    /// Supernovas requises pour ce générateur (0 si aucune).
    pub supernovas_needed: u32,
    /// Générateur primordial pas encore forgé : particules nécessaires (0 sinon).
    pub particles_needed: u32,
}

/// Un générateur primordial et son état de forge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForgeView {
    pub id: GeneratorId,
    pub forged: bool,
    /// Forgeable maintenant (particules suffisantes, précédent forgé).
    pub available: bool,
}

/// Un système de la Carte stellaire, vu par la présentation.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemView {
    pub index: usize,
    pub name: String,
    /// Systèmes reliés.
    pub links: Vec<usize>,
    pub column: usize,
    /// Position verticale dans la colonne (0..1).
    pub y: f64,
    pub kind: domain::SystemKind,
    pub danger: u8,
    pub explored: bool,
    /// Visible (exploré ou relié à un système exploré).
    pub revealed: bool,
    /// Une flotte peut partir vers lui maintenant.
    pub available: bool,
    pub chance: f64,
    pub duration: f64,
    pub reward_scale: f64,
}

/// Une relique de la collection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RelicView {
    pub id: domain::RelicId,
    /// Meilleure rareté possédée (0 à 3), `None` si pas encore trouvée.
    pub rarity: Option<u8>,
    pub equipped: bool,
}

/// Défi du jour en cours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DailyView {
    /// Secondes restantes.
    pub left: f64,
    pub score: f64,
}

/// Une flotte en route.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FleetView {
    pub target: usize,
    pub remaining: f64,
    pub total: f64,
}

/// Étape du Grand Attracteur en construction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResearchBuildView {
    pub stage: usize,
    pub remaining: f64,
    pub total: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UpgradeView {
    pub id: UpgradeId,
    pub cost: f64,
    pub affordable: bool,
}

/// Une amélioration de l'Atelier stellaire.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PerkView {
    pub id: PerkId,
    pub owned: bool,
    pub affordable: bool,
}

/// Un niveau de Constellation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstellationView {
    pub id: ConstellationId,
    pub level: u32,
    /// Coût du niveau suivant, en fragments.
    pub cost: f64,
    pub affordable: bool,
    pub maxed: bool,
}

/// Un Défi et son état.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChallengeView {
    pub id: ChallengeId,
    /// Assez de Supernovas pour le lancer.
    pub unlocked: bool,
    pub done: bool,
    pub active: bool,
}

/// Le Défi en cours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActiveChallengeView {
    pub id: ChallengeId,
    /// Avancement vers l'objectif (0 → 1).
    pub progress: f64,
    pub elapsed: f64,
    /// Temps restant pour un Défi chronométré.
    pub time_left: Option<f64>,
}

/// Une recherche du Laboratoire.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabView {
    pub id: LabId,
    pub done: bool,
    pub paused: bool,
    /// Peut être lancée maintenant.
    pub available: bool,
    /// Son prérequis n'est pas encore terminé.
    pub blocked: bool,
}

/// Un univers parallèle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UniverseView {
    pub id: UniverseId,
    pub current: bool,
    pub key: bool,
}

/// La recherche en cours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResearchView {
    pub id: LabId,
    pub remaining: f64,
    pub total: f64,
}

/// Prochaine amélioration encore verrouillée et où on en est de sa condition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NextUpgradeView {
    pub id: UpgradeId,
    /// Valeur actuelle (exemplaires possédés ou poussière récoltée ce cycle).
    pub current: f64,
    /// Valeur requise.
    pub needed: f64,
}

/// Où en est le joueur, globalement.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ProgressView {
    /// Générateurs possédés au moins une fois dans ce cycle.
    pub generators_found: usize,
    pub generators_total: usize,
    pub upgrades_bought: usize,
    pub upgrades_total: usize,
    pub perks_owned: usize,
    pub perks_total: usize,
    /// Complétion globale (0 → 1) : générateurs, succès et Atelier.
    pub completion: f64,
    /// `total_run` à atteindre pour le prochain fragment.
    pub next_fragment_at: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GameSnapshot {
    pub stardust: f64,
    pub production: f64,
    pub click_value: f64,
    pub total_run: f64,
    pub total_all: f64,
    pub fragments: f64,
    pub fragments_available: f64,
    /// Bonus de production des fragments (0.6 = +60 %).
    pub fragment_bonus: f64,
    pub supernovas: u32,
    pub clicks: u64,
    pub meteors_caught: u32,
    pub play_time: f64,
    /// Début de la partie (dernier « Nouvelle partie », secondes Unix ; 0 :
    /// jamais).
    pub started_at: u64,
    pub achievements_unlocked: usize,
    pub achievements_total: usize,
    /// Succès débloqués, indexés comme le catalogue.
    pub achievements: Vec<bool>,
    pub buy_mode: BuyMode,
    pub storm_remaining: f64,
    /// Avancement de la comète active (0 → 1).
    pub comet_progress: Option<f64>,
    /// Générateurs visibles, dans l'ordre du catalogue.
    pub generators: Vec<GeneratorView>,
    pub teaser: Option<TeaserView>,
    /// Améliorations débloquées et non achetées, triées par coût croissant.
    pub upgrades: Vec<UpgradeView>,
    /// Améliorations interdites par la règle en cours (Défi, univers, défi du
    /// jour) : la liste reste vide.
    pub upgrades_forbidden: bool,
    /// Exemplaires possédés de chaque générateur (visibles ou non).
    pub owned: Vec<u64>,
    /// Production par seconde échantillonnée chaque seconde (la plus ancienne d'abord).
    pub history: Vec<f64>,
    /// Production de chaque générateur pour chaque échantillon de `history`,
    /// à plat : l'échantillon `i` occupe `[i * width, (i + 1) * width)`.
    pub history_mix: Vec<f64>,
    /// Nombre de générateurs par échantillon de `history_mix`.
    pub history_mix_width: usize,
    pub reduced_effects: bool,
    /// Images par seconde (60, 30, 20 ou 15).
    pub frame_rate: u32,
    /// Sons activés dans les options.
    pub sound_on: bool,
    /// Volume des sons en % (20 à 100).
    pub volume: u8,
    /// « Récolte continue » achetée : maintenir Espace récolte en continu.
    pub hold_harvest: bool,
    /// Fragments disponibles pour l'Atelier.
    pub fragments_spendable: f64,
    /// Atelier stellaire, dans l'ordre du catalogue.
    pub perks: Vec<PerkView>,
    /// Constellations, dans l'ordre du catalogue.
    pub constellations: Vec<ConstellationView>,
    /// Défis, dans l'ordre du catalogue.
    pub challenges: Vec<ChallengeView>,
    pub active_challenge: Option<ActiveChallengeView>,
    /// Sorte de la comète active.
    pub comet_kind: CometKind,
    /// Multiplicateur de la tempête (ou frénésie dorée) en cours.
    pub storm_multiplier: f64,
    /// Détail du multiplicateur global (hors tempête) : sources actives
    /// (facteur ≠ 1), avec leur nom.
    pub production_sources: Vec<(&'static str, f64)>,
    /// Produit des sources (hors tempête).
    pub permanent_multiplier: f64,
    /// Big Crunch déclenchés (matière noire).
    pub crunches: u32,
    /// Big Crunch possible maintenant.
    pub crunch_available: bool,
    /// Supernovas nécessaires au Big Crunch.
    pub crunch_needed: u32,
    /// Multiplicateurs actuels de la matière noire (production, fragments).
    pub crunch_production: f64,
    /// Multiplicateur après le prochain Big Crunch.
    pub crunch_production_next: f64,
    pub crunch_fragments: f64,
    /// Laboratoire de matière noire.
    pub lab_unlocked: bool,
    pub lab_needed: u32,
    pub dark_matter_spendable: u32,
    pub lab: Vec<LabView>,
    pub research: Option<ResearchView>,
    /// Univers parallèles.
    pub universe: UniverseId,
    pub universes_unlocked: bool,
    pub universes: Vec<UniverseView>,
    /// Facteur d'entropie du cycle (1 = aucune perte).
    pub entropy: f64,
    pub constellation_levels: u32,
    pub autobuy_unlocked: bool,
    pub autobuy: bool,
    pub next_upgrade: Option<NextUpgradeView>,
    pub progress: ProgressView,
    /// Big Bang et particules primordiales.
    pub big_bangs: u32,
    pub particles: u32,
    pub particles_spendable: u32,
    /// Particules que rapporterait un Big Bang maintenant (0 si impossible).
    pub particles_available: u32,
    pub bigbang_available: bool,
    pub bigbang_needed: u32,
    pub particle_production: f64,
    pub particle_fragments: f64,
    /// Générateurs primordiaux.
    pub forges: Vec<ForgeView>,
    /// Grand Attracteur : étapes achevées, construction en cours.
    pub attractor_stages: usize,
    pub attractor_build: Option<ResearchBuildView>,
    pub attractor_buildable: bool,
    pub attractor_awakened: bool,
    pub attractor_production: f64,
    /// Acte III : Carte stellaire et expéditions.
    pub starmap_unlocked: bool,
    pub systems: Vec<SystemView>,
    pub fleets: Vec<FleetView>,
    pub fleets_max: usize,
    /// Systèmes explorés (hors fonderie).
    pub explored: usize,
    pub map_production: f64,
    pub map_fragments: f64,
    /// Reliques : collection, emplacements, Éclats, ensembles (reliques
    /// équipées par ensemble).
    pub relics: Vec<RelicView>,
    pub relic_slots: usize,
    pub relics_equipped: usize,
    pub shards: u32,
    pub relic_production: f64,
    pub relic_sets: Vec<(domain::RelicSet, usize)>,
    /// Événement cosmique en attente : (index, secondes pour répondre).
    pub pending_event: Option<(usize, f64)>,
    /// Bonus temporaires en cours.
    pub boosts: Vec<domain::Boost>,
    pub events_resolved: u32,
    /// Convergence : prête, en cours (restant, total), achevée.
    pub convergence_ready: bool,
    pub convergence: Option<(f64, f64)>,
    pub converged: bool,
    /// Pilote automatique des flottes.
    pub auto_fleets: bool,
    pub auto_fleets_unlocked: bool,
    /// Historique de production sur toute la partie (temps de jeu, log10).
    pub history_long: Vec<(f64, f32)>,
    pub best_production: f64,
    /// Temps de jeu des grandes étapes (0 : pas encore).
    pub milestones: [f64; 6],
    /// Couleurs : 0 normales, 1 daltonien, 2 contraste élevé.
    pub color_mode: u8,
    /// Acte IV : Mégastructures.
    pub megastructures_unlocked: bool,
    pub exotic: f64,
    pub grid: Vec<Option<domain::StructureKind>>,
    pub grid_effects: domain::GridEffects,
    /// Coût actuel de chaque type (dans l'ordre de `StructureKind::ALL`).
    pub structure_costs: [f64; 6],
    /// Défi du jour : règles du jour, défi en cours, record et récompense.
    pub daily_unlocked: bool,
    pub daily_rules: Vec<&'static str>,
    pub daily: Option<DailyView>,
    pub daily_best: f64,
    pub daily_rewarded: bool,
    /// Éveil du Cosmos : prêt, en cours (restant, total), achevé.
    pub awakening_ready: bool,
    pub awakening: Option<(f64, f64)>,
    pub awakened: bool,
}
