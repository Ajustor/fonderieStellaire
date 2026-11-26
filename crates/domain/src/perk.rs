//! Atelier stellaire : améliorations permanentes achetées avec les fragments
//! de Supernova. Elles survivent à toutes les Supernovas.
//!
//! Dépenser des fragments ne retire **pas** leur bonus de production : le bonus
//! dépend du total de fragments gagnés, l'Atelier du nombre de fragments libres.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PerkId(pub usize);

impl PerkId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PerkEffect {
    /// Chaque nouveau cycle commence avec ces générateurs.
    StartingKit { probes: u64, drones: u64 },
    /// Pourcentage de la prod/s ajouté à chaque récolte manuelle.
    ClickBoost(f64),
    /// Multiplie l'intervalle entre deux comètes (< 1 = plus fréquentes).
    CometMagnet(f64),
    /// Multiplie la durée des tempêtes solaires.
    LongStorm(f64),
    /// Multiplie le coût de tous les générateurs (< 1 = moins cher).
    GeneratorDiscount(f64),
    /// Production hors-ligne : rendement et plafond (secondes).
    OfflineMastery { rate: f64, cap_secs: f64 },
    /// Achète automatiquement des générateurs (activable).
    AutoBuyer,
    /// Multiplie les fragments gagnés à chaque Supernova.
    RichNova(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PerkDef {
    pub id: PerkId,
    /// Identifiant texte stable, utilisé par la sauvegarde.
    pub key: &'static str,
    pub name: &'static str,
    /// Ce que fait l'amélioration, en clair.
    pub description: &'static str,
    pub symbol: char,
    /// Coût en fragments.
    pub cost: f64,
    pub effect: PerkEffect,
}
