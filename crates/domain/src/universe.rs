//! Univers parallèles : à chaque Big Crunch, on choisit les règles du cycle
//! suivant. Atteindre l'objectif d'un univers rapporte sa Clé (bonus permanent).

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UniverseId(pub usize);

impl UniverseId {
    /// L'univers d'origine (règles normales).
    pub const ORIGIN: UniverseId = UniverseId(0);

    pub fn index(self) -> usize {
        self.0
    }
}

/// Règles d'un univers (multiplicateurs ; 1.0 = inchangé).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UniverseRules {
    /// Coût des générateurs.
    pub cost: f64,
    /// Production.
    pub production: f64,
    /// Gains hors-ligne.
    pub offline: f64,
    /// Intervalle entre deux comètes.
    pub comet_interval: f64,
    /// Durée des tempêtes solaires.
    pub storm_duration: f64,
    /// Fragments gagnés.
    pub fragments: f64,
    /// Améliorations interdites.
    pub no_upgrades: bool,
    /// Perte de production par minute passée dans le cycle (0.01 = −1 %/min).
    pub entropy_per_min: f64,
}

impl UniverseRules {
    pub const NORMAL: UniverseRules = UniverseRules {
        cost: 1.0,
        production: 1.0,
        offline: 1.0,
        comet_interval: 1.0,
        storm_duration: 1.0,
        fragments: 1.0,
        no_upgrades: false,
        entropy_per_min: 0.0,
    };
}

/// Plancher de l'entropie : la production ne descend pas sous 10 %.
pub const ENTROPY_FLOOR: f64 = 0.10;

/// Bonus permanent d'une Clé d'univers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyReward {
    /// Coût des générateurs multiplié.
    GeneratorDiscount(f64),
    /// Gains hors-ligne multipliés, plafond augmenté (secondes).
    Offline { factor: f64, extra_cap_secs: f64 },
    /// Pluie d'étoiles multipliée, comètes rares plus fréquentes.
    Comets { shower: f64, rare: f64 },
    /// Production multipliée.
    Production(f64),
    /// Fragments multipliés.
    Fragments(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct UniverseDef {
    pub id: UniverseId,
    /// Identifiant texte stable, utilisé par la sauvegarde.
    pub key: &'static str,
    pub name: &'static str,
    /// Les règles, en clair.
    pub rules_text: &'static str,
    pub symbol: char,
    pub rules: UniverseRules,
    /// Supernovas à atteindre dans cet univers pour gagner sa Clé (0 = aucune Clé).
    pub key_goal: u32,
    pub key_name: &'static str,
    pub key_reward: Option<KeyReward>,
    /// Le bonus de la Clé, en clair.
    pub reward_text: &'static str,
}
