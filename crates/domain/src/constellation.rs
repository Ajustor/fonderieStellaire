//! Constellations : améliorations à niveaux payées en fragments, sans fin.
//! Chaque niveau coûte plus cher que le précédent (`base × croissance^niveau`) :
//! les fragments gardent toujours une utilité, même une fois l'Atelier complet.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConstellationId(pub usize);

impl ConstellationId {
    pub fn index(self) -> usize {
        self.0
    }
}

/// Effet **par niveau**.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConstellationEffect {
    /// +x production globale par niveau (0.20 = +20 %).
    Production(f64),
    /// +x de la prod/s ajoutée à chaque clic par niveau.
    Click(f64),
    /// Tempête solaire +1 multiplicateur et Pluie d'étoiles +x par niveau.
    Comets(f64),
    /// Heures de hors-ligne en plus par niveau.
    OfflineHours(f64),
    /// +x fragments gagnés par niveau (0.10 = +10 %).
    Fragments(f64),
    /// Coût des générateurs × x par niveau (0.97 = −3 %).
    Discount(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConstellationDef {
    pub id: ConstellationId,
    /// Identifiant texte stable, utilisé par la sauvegarde.
    pub key: &'static str,
    pub name: &'static str,
    /// Ce que fait un niveau, en clair.
    pub description: &'static str,
    pub symbol: char,
    pub base_cost: f64,
    pub growth: f64,
    /// Niveau maximal (`None` = sans limite).
    pub max_level: Option<u32>,
    pub effect: ConstellationEffect,
}

impl ConstellationDef {
    /// Coût en fragments du niveau suivant quand on est au niveau `level`.
    pub fn cost_at(&self, level: u32) -> f64 {
        (self.base_cost * self.growth.powi(level as i32)).ceil()
    }

    pub fn is_maxed(&self, level: u32) -> bool {
        self.max_level.is_some_and(|m| level >= m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(max: Option<u32>) -> ConstellationDef {
        ConstellationDef {
            id: ConstellationId(0),
            key: "c",
            name: "C",
            description: "",
            symbol: '✦',
            base_cost: 2.0,
            growth: 1.6,
            max_level: max,
            effect: ConstellationEffect::Production(0.2),
        }
    }

    #[test]
    fn costs_grow_and_round_up() {
        let d = def(None);
        assert_eq!(d.cost_at(0), 2.0);
        assert_eq!(d.cost_at(1), 4.0); // 3.2 arrondi au-dessus
        assert!(d.cost_at(10) > d.cost_at(9));
        assert!(!d.is_maxed(1000));
        assert!(def(Some(3)).is_maxed(3));
    }
}
