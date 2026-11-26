//! Vue en lecture seule de l'état du jeu, destinée à la présentation.

use domain::{BuyMode, GeneratorId, UpgradeId};

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
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UpgradeView {
    pub id: UpgradeId,
    pub cost: f64,
    pub affordable: bool,
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
}
