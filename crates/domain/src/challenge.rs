//! Défis : un cycle joué avec une contrainte. Réussir l'objectif rapporte une
//! récompense permanente. Lancer un Défi relance le cycle (comme une Supernova).

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChallengeId(pub usize);

impl ChallengeId {
    pub fn index(self) -> usize {
        self.0
    }
}

/// Règle imposée pendant le Défi.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Restriction {
    /// Aucune récolte manuelle.
    NoClick,
    /// Aucune amélioration.
    NoUpgrades,
    /// Aucune comète.
    NoComets,
    /// Objectif à atteindre en moins de N secondes.
    TimeLimit(f64),
    /// Seuls les N premiers générateurs.
    OnlyFirstGenerators(usize),
}

/// Récompense permanente d'un Défi réussi.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChallengeReward {
    /// Production globale multipliée.
    Production(f64),
    /// Coût des améliorations multiplié.
    UpgradeDiscount(f64),
    /// Intervalle entre les comètes multiplié.
    CometFrequency(f64),
    /// Fragments en plus à chaque Supernova.
    BonusFragments(f64),
    /// Production des `count` premiers générateurs multipliée.
    EarlyGenerators { count: usize, factor: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChallengeDef {
    pub id: ChallengeId,
    /// Identifiant texte stable, utilisé par la sauvegarde.
    pub key: &'static str,
    pub name: &'static str,
    /// La contrainte, en clair.
    pub rule: &'static str,
    /// La récompense, en clair.
    pub reward_text: &'static str,
    pub symbol: char,
    /// Poussière à récolter dans le cycle pour réussir.
    pub goal: f64,
    /// Supernovas nécessaires pour le débloquer.
    pub unlock_supernovas: u32,
    pub restriction: Restriction,
    pub reward: ChallengeReward,
}
