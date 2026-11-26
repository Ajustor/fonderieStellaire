//! Succès : définitions et conditions.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AchievementId(pub usize);

impl AchievementId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AchievementCondition {
    /// Au moins N générateurs possédés au total.
    GeneratorsOwned(u64),
    /// Au moins N exemplaires d'un même générateur.
    SameGenerator(u64),
    /// Au moins un exemplaire de chacun des N premiers générateurs.
    AllTypes(usize),
    TotalAll(f64),
    Clicks(u64),
    CometsCaught(u32),
    Supernovas(u32),
    /// Production par seconde minimale.
    Production(f64),
    /// Au moins `count` exemplaires du générateur n° `index`.
    OwnGenerator {
        index: usize,
        count: u64,
    },
    /// Fragments gagnés au total.
    Fragments(f64),
    /// Améliorations de l'Atelier achetées.
    PerksOwned(usize),
    /// Toutes les améliorations de l'Atelier.
    AllPerks,
    /// Niveaux de Constellations au total.
    ConstellationLevels(u32),
    GoldenComets(u32),
    BlackComets(u32),
    ChallengesDone(usize),
    /// Big Crunch déclenchés.
    Crunches(u32),
    /// Recherches du Laboratoire terminées.
    LabResearch(usize),
    /// Clés d'univers obtenues.
    UniverseKeys(usize),
}

/// Données nécessaires pour évaluer une condition de succès.
#[derive(Clone, Copy, Debug)]
pub struct AchievementContext<'a> {
    pub owned: &'a [u64],
    pub total_all: f64,
    pub clicks: u64,
    pub comets: u32,
    pub supernovas: u32,
    pub production: f64,
    pub fragments: f64,
    pub perks_owned: usize,
    pub perks_total: usize,
    pub constellation_levels: u32,
    pub golden_comets: u32,
    pub black_comets: u32,
    pub challenges_done: usize,
    pub crunches: u32,
    pub lab_done: usize,
    pub universe_keys: usize,
}

impl AchievementCondition {
    pub fn is_met(&self, ctx: &AchievementContext<'_>) -> bool {
        match *self {
            AchievementCondition::GeneratorsOwned(n) => ctx.owned.iter().sum::<u64>() >= n,
            AchievementCondition::SameGenerator(n) => ctx.owned.iter().any(|&o| o >= n),
            AchievementCondition::AllTypes(n) => {
                ctx.owned.len() >= n && ctx.owned.iter().take(n).all(|&o| o >= 1)
            }
            AchievementCondition::TotalAll(v) => ctx.total_all >= v,
            AchievementCondition::Clicks(n) => ctx.clicks >= n,
            AchievementCondition::CometsCaught(n) => ctx.comets >= n,
            AchievementCondition::Supernovas(n) => ctx.supernovas >= n,
            AchievementCondition::Production(p) => ctx.production >= p,
            AchievementCondition::OwnGenerator { index, count } => {
                ctx.owned.get(index).copied().unwrap_or(0) >= count
            }
            AchievementCondition::Fragments(f) => ctx.fragments >= f,
            AchievementCondition::PerksOwned(n) => ctx.perks_owned >= n,
            AchievementCondition::AllPerks => {
                ctx.perks_total > 0 && ctx.perks_owned >= ctx.perks_total
            }
            AchievementCondition::ConstellationLevels(n) => ctx.constellation_levels >= n,
            AchievementCondition::GoldenComets(n) => ctx.golden_comets >= n,
            AchievementCondition::BlackComets(n) => ctx.black_comets >= n,
            AchievementCondition::ChallengesDone(n) => ctx.challenges_done >= n,
            AchievementCondition::Crunches(n) => ctx.crunches >= n,
            AchievementCondition::LabResearch(n) => ctx.lab_done >= n,
            AchievementCondition::UniverseKeys(n) => ctx.universe_keys >= n,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AchievementDef {
    pub id: AchievementId,
    /// Identifiant texte stable, utilisé par la sauvegarde.
    pub key: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub condition: AchievementCondition,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(owned: &[u64]) -> AchievementContext<'_> {
        AchievementContext {
            owned,
            total_all: 0.0,
            clicks: 0,
            comets: 0,
            supernovas: 0,
            production: 0.0,
            fragments: 0.0,
            perks_owned: 0,
            perks_total: 0,
            constellation_levels: 0,
            golden_comets: 0,
            black_comets: 0,
            challenges_done: 0,
            crunches: 0,
            lab_done: 0,
            universe_keys: 0,
        }
    }

    #[test]
    fn generator_conditions() {
        assert!(!AchievementCondition::GeneratorsOwned(1).is_met(&ctx(&[0, 0])));
        assert!(AchievementCondition::GeneratorsOwned(1).is_met(&ctx(&[0, 1])));
        assert!(AchievementCondition::SameGenerator(100).is_met(&ctx(&[3, 100])));
        assert!(!AchievementCondition::SameGenerator(100).is_met(&ctx(&[99, 99])));
        assert!(AchievementCondition::AllTypes(3).is_met(&ctx(&[1, 2, 3])));
        assert!(!AchievementCondition::AllTypes(3).is_met(&ctx(&[1, 0, 3])));
        assert!(!AchievementCondition::AllTypes(3).is_met(&ctx(&[])));
    }

    #[test]
    fn counter_conditions() {
        let mut c = ctx(&[]);
        c.total_all = 1e6;
        c.clicks = 100;
        c.comets = 1;
        c.supernovas = 1;
        c.production = 1e6;
        assert!(AchievementCondition::TotalAll(1e6).is_met(&c));
        assert!(!AchievementCondition::TotalAll(1e9).is_met(&c));
        assert!(AchievementCondition::Clicks(100).is_met(&c));
        assert!(!AchievementCondition::Clicks(1000).is_met(&c));
        assert!(AchievementCondition::CometsCaught(1).is_met(&c));
        assert!(!AchievementCondition::CometsCaught(10).is_met(&c));
        assert!(AchievementCondition::Supernovas(1).is_met(&c));
        assert!(AchievementCondition::Production(1e6).is_met(&c));
    }
}
