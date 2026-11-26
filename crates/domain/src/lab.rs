//! Laboratoire de matière noire : des recherches payées en matière noire, qui
//! durent un temps réel (une à la fois, et elles avancent même jeu fermé).
//! La matière noire dépensée garde son bonus de production.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LabId(pub usize);

impl LabId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LabEffect {
    /// Déclenche la Supernova dès qu'elle rapporte au moins autant de
    /// fragments qu'on en a déjà (5 minimum). Activable.
    AutoSupernova,
    /// Dépense les fragments libres en Constellations. Activable.
    AutoConstellations,
    /// Capture les comètes normales. Activable.
    CometNet,
    /// L'Atelier est conservé après un Big Crunch.
    KeepAtelier,
    /// Les Défis réussis sont conservés après un Big Crunch.
    KeepChallenges,
    /// Chaque Big Crunch démarre avec N Supernovas déjà faites.
    HeadStart(u32),
    /// Production multipliée.
    Production(f64),
    /// Fragments multipliés.
    Fragments(f64),
    /// Hors-ligne : rendement et plafond (secondes).
    Offline { rate: f64, cap_secs: f64 },
    /// Fraction de la prod/s ajoutée à chaque clic.
    Click(f64),
    /// Comètes rares plus fréquentes (facteur sur leurs probabilités).
    RareComets(f64),
}

impl LabEffect {
    /// Les automatismes peuvent être mis en pause.
    pub fn is_toggleable(self) -> bool {
        matches!(
            self,
            LabEffect::AutoSupernova | LabEffect::AutoConstellations | LabEffect::CometNet
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LabDef {
    pub id: LabId,
    /// Identifiant texte stable, utilisé par la sauvegarde.
    pub key: &'static str,
    pub name: &'static str,
    /// Ce que ça fait, en clair.
    pub description: &'static str,
    pub symbol: char,
    /// Coût en matière noire.
    pub cost: u32,
    /// Durée de la recherche, en secondes de temps réel.
    pub research_secs: f64,
    /// Recherche à terminer avant celle-ci (clé).
    pub requires: Option<&'static str>,
    pub effect: LabEffect,
}
