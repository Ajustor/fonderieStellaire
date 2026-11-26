//! Vue en lecture seule de l'état du jeu, destinée à la présentation.

use domain::{
    BuyMode, ChallengeId, CometKind, ConstellationId, GeneratorId, LabId, PerkId, UpgradeId,
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
    /// Big Crunch déclenchés (matière noire).
    pub crunches: u32,
    /// Big Crunch possible maintenant.
    pub crunch_available: bool,
    /// Supernovas nécessaires au Big Crunch.
    pub crunch_needed: u32,
    /// Multiplicateurs actuels de la matière noire (production, fragments).
    pub crunch_production: f64,
    pub crunch_fragments: f64,
    /// Laboratoire de matière noire.
    pub lab_unlocked: bool,
    pub lab_needed: u32,
    pub dark_matter_spendable: u32,
    pub lab: Vec<LabView>,
    pub research: Option<ResearchView>,
    pub constellation_levels: u32,
    pub autobuy_unlocked: bool,
    pub autobuy: bool,
    pub next_upgrade: Option<NextUpgradeView>,
    pub progress: ProgressView,
}
