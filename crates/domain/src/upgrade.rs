//! Améliorations : identifiants, effets et conditions de déblocage.

use crate::generator::GeneratorId;

/// Identifiant d'une amélioration (index dans le catalogue). L'identifiant
/// texte stable (`gen3_t2`, `global_1`…) est porté par [`UpgradeDef::key`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UpgradeId(pub usize);

impl UpgradeId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    /// Multiplie la production d'un générateur.
    GeneratorMultiplier { generator: GeneratorId, factor: f64 },
    /// Multiplie toute la production.
    GlobalMultiplier(f64),
    /// Ajoute une fraction de la prod/s à chaque clic (0.01 = +1 %).
    ClickPercent(f64),
    /// Maintenir la touche de récolte récolte en continu.
    HoldHarvest,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Requirement {
    GeneratorOwned { generator: GeneratorId, count: u64 },
    TotalRun(f64),
}

impl Requirement {
    pub fn is_met(&self, owned: &[u64], total_run: f64) -> bool {
        match *self {
            Requirement::GeneratorOwned { generator, count } => {
                owned.get(generator.index()).copied().unwrap_or(0) >= count
            }
            Requirement::TotalRun(min) => total_run >= min,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UpgradeDef {
    pub id: UpgradeId,
    /// Identifiant texte stable, utilisé par la sauvegarde.
    pub key: String,
    pub name: String,
    pub description: String,
    pub cost: f64,
    pub requirement: Requirement,
    pub effect: Effect,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requirements() {
        let r = Requirement::GeneratorOwned {
            generator: GeneratorId(1),
            count: 10,
        };
        assert!(!r.is_met(&[50, 9], 0.0));
        assert!(r.is_met(&[0, 10], 0.0));
        assert!(!r.is_met(&[0], 0.0));
        let t = Requirement::TotalRun(1e4);
        assert!(!t.is_met(&[], 9_999.0));
        assert!(t.is_met(&[], 1e4));
    }
}
