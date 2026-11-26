//! Acte III : les Reliques. Rapportées par les expéditions, elles forment une
//! collection de 16 objets (4 ensembles de 4), chacun possédé à sa meilleure
//! rareté. On en équipe quelques-unes ; 3 du même ensemble donnent un bonus
//! d'ensemble. Un doublon devient un Éclat (+1 % de production).

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RelicId(pub usize);

impl RelicId {
    pub fn index(self) -> usize {
        self.0
    }
}

/// Rareté, de 0 (commune) à 3 (légendaire).
pub const RARITY_NAMES: [&str; 4] = ["commune", "rare", "épique", "légendaire"];
/// Puissance de l'effet selon la rareté.
pub const RARITY_POWER: [f64; 4] = [1.0, 1.6, 2.5, 4.0];

/// Les quatre ensembles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelicSet {
    Forge,
    Voyager,
    Eclipse,
    Origins,
}

impl RelicSet {
    pub const ALL: [RelicSet; 4] = [
        RelicSet::Forge,
        RelicSet::Voyager,
        RelicSet::Eclipse,
        RelicSet::Origins,
    ];

    pub fn name(self) -> &'static str {
        match self {
            RelicSet::Forge => "Forgeron",
            RelicSet::Voyager => "Voyageur",
            RelicSet::Eclipse => "Éclipse",
            RelicSet::Origins => "Origines",
        }
    }

    /// Bonus quand 3 reliques de l'ensemble sont équipées.
    pub fn bonus_text(self) -> &'static str {
        match self {
            RelicSet::Forge => "production ×1,5",
            RelicSet::Voyager => "expéditions 25 % plus rapides et +10 % de succès",
            RelicSet::Eclipse => "hors-ligne +25 % de rendement",
            RelicSet::Origins => "production ×2",
        }
    }
}

/// Effet d'une relique équipée (valeur de base, avant la rareté).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RelicEffect {
    /// +x de production (additif entre reliques).
    Production(f64),
    /// +x de fragments.
    Fragments(f64),
    /// +x de la prod/s à chaque clic.
    Click(f64),
    /// Hors-ligne : +x heures de plafond.
    OfflineHours(f64),
    /// Hors-ligne : +x de rendement.
    OfflineRate(f64),
    /// Comètes : +x de fréquence.
    Comets(f64),
    /// Expéditions : −x de durée.
    ExpeditionSpeed(f64),
    /// Expéditions : +x de chance de succès.
    ExpeditionLuck(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct RelicDef {
    pub id: RelicId,
    pub key: &'static str,
    pub name: &'static str,
    pub set: RelicSet,
    pub effect: RelicEffect,
    pub flavor: &'static str,
}

impl RelicDef {
    /// Effet à une rareté donnée.
    pub fn effect_at(&self, rarity: u8) -> RelicEffect {
        let k = RARITY_POWER[(rarity as usize).min(3)];
        match self.effect {
            RelicEffect::Production(x) => RelicEffect::Production(x * k),
            RelicEffect::Fragments(x) => RelicEffect::Fragments(x * k),
            RelicEffect::Click(x) => RelicEffect::Click(x * k),
            RelicEffect::OfflineHours(x) => RelicEffect::OfflineHours(x * k),
            RelicEffect::OfflineRate(x) => RelicEffect::OfflineRate(x * k),
            RelicEffect::Comets(x) => RelicEffect::Comets(x * k),
            RelicEffect::ExpeditionSpeed(x) => RelicEffect::ExpeditionSpeed((x * k).min(0.5)),
            RelicEffect::ExpeditionLuck(x) => RelicEffect::ExpeditionLuck(x * k),
        }
    }
}

/// Description lisible d'un effet.
pub fn effect_text(e: RelicEffect) -> String {
    let pct = |x: f64| format!("{}%", (x * 100.0).round() as i64);
    match e {
        RelicEffect::Production(x) => format!("+{} de production", pct(x)),
        RelicEffect::Fragments(x) => format!("+{} de fragments", pct(x)),
        RelicEffect::Click(x) => format!("+{} de la prod/s par clic", pct(x)),
        RelicEffect::OfflineHours(x) => format!("+{} h de hors-ligne", x.round() as i64),
        RelicEffect::OfflineRate(x) => format!("+{} de rendement hors-ligne", pct(x)),
        RelicEffect::Comets(x) => format!("comètes +{} plus fréquentes", pct(x)),
        RelicEffect::ExpeditionSpeed(x) => format!("expéditions −{} de durée", pct(x)),
        RelicEffect::ExpeditionLuck(x) => format!("+{} de succès en expédition", pct(x)),
    }
}

/// Rareté tirée pour une relique trouvée (`roll` dans [0, 1)) : le danger du
/// système rend les reliques rares plus probables.
pub fn roll_rarity(roll: f64, danger: u8) -> u8 {
    let boost = 0.05 * danger.saturating_sub(1) as f64;
    let legendary = 0.02 + boost * 0.3;
    let epic = legendary + 0.10 + boost * 0.6;
    let rare = epic + 0.28 + boost;
    if roll < legendary {
        3
    } else if roll < epic {
        2
    } else if roll < rare {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rarity_scales_effects_and_danger_helps() {
        let d = RelicDef {
            id: RelicId(0),
            key: "k",
            name: "n",
            set: RelicSet::Forge,
            effect: RelicEffect::Production(0.2),
            flavor: "",
        };
        assert_eq!(d.effect_at(0), RelicEffect::Production(0.2));
        assert_eq!(d.effect_at(3), RelicEffect::Production(0.8));
        assert_eq!(roll_rarity(0.99, 1), 0);
        assert_eq!(roll_rarity(0.0, 1), 3);
        // Plus de danger : la même chance donne mieux.
        assert!(roll_rarity(0.25, 5) > roll_rarity(0.25, 1));
        assert_eq!(
            effect_text(RelicEffect::OfflineHours(4.0)),
            "+4 h de hors-ligne"
        );
    }
}
