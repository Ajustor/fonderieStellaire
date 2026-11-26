//! Données du jeu : générateurs, améliorations, succès et constantes d'équilibrage.

use crate::achievement::{AchievementCondition, AchievementDef, AchievementId};
use crate::generator::{GeneratorDef, GeneratorId};
use crate::upgrade::{Effect, Requirement, UpgradeDef, UpgradeId};

/// Constantes d'équilibrage.
pub mod balance {
    /// Bonus global par fragment de Supernova (+5 %).
    pub const FRAGMENT_BONUS: f64 = 0.05;
    /// Bonus global par succès débloqué (+1 %).
    pub const ACHIEVEMENT_BONUS: f64 = 0.01;
    /// Diviseur de la formule des fragments : `floor(sqrt(total_run / 1e8))`.
    pub const FRAGMENT_DIVISOR: f64 = 1e8;
    /// Plafond du temps hors-ligne pris en compte (12 h).
    pub const OFFLINE_CAP_SECS: f64 = 12.0 * 3600.0;
    /// Rendement de la production hors-ligne.
    pub const OFFLINE_RATE: f64 = 0.5;
    /// Un générateur se dévoile quand `total_run ≥ base_cost × REVEAL_RATIO`.
    pub const REVEAL_RATIO: f64 = 0.5;
    /// Gain de base d'un clic.
    pub const CLICK_BASE: f64 = 1.0;
    /// Intervalle minimal entre deux récoltes « touche maintenue » (10 par seconde).
    pub const HOLD_HARVEST_INTERVAL: f64 = 0.1;
}

/// Ligne de la table des générateurs : (nom, pluriel, symbole, coût, prod/s).
const GENERATORS: [(&str, &str, char, f64, f64); 9] = [
    ("Sonde", "Sondes", '✧', 15.0, 0.1),
    ("Drone collecteur", "Drones collecteurs", '◇', 100.0, 1.0),
    (
        "Moissonneur orbital",
        "Moissonneurs orbitaux",
        '◆',
        1_100.0,
        8.0,
    ),
    ("Station lunaire", "Stations lunaires", '○', 12_000.0, 47.0),
    ("Sphère de Dyson", "Sphères de Dyson", '●', 130_000.0, 260.0),
    ("Forge stellaire", "Forges stellaires", '▲', 1.4e6, 1_400.0),
    (
        "Usine à nébuleuses",
        "Usines à nébuleuses",
        '✦',
        2e7,
        7_800.0,
    ),
    (
        "Moteur quantique",
        "Moteurs quantiques",
        '◈',
        3.3e8,
        44_000.0,
    ),
    (
        "Tisseur de galaxies",
        "Tisseurs de galaxies",
        '★',
        5.1e9,
        260_000.0,
    ),
];

/// Ce que fait chaque générateur (affiché dans le détail de la sélection).
const GENERATOR_FLAVORS: [&str; 9] = [
    "Petite sonde autonome qui aspire la poussière en orbite basse.",
    "Essaim de drones qui ratissent les ceintures d'astéroïdes.",
    "Vaisseau-usine qui fauche les nuages de poussière interstellaire.",
    "Base lunaire qui raffine la poussière à grande échelle.",
    "Coque géante qui capte toute l'énergie d'une étoile.",
    "Fonderie plongée au cœur d'une étoile pour en couler la matière.",
    "Condense des nébuleuses entières en poussière d'étoile pure.",
    "Extrait la poussière des fluctuations du vide quantique.",
    "Tisse des galaxies neuves, fil d'étoile après fil d'étoile.",
];

/// Textes d'ambiance des niveaux d'amélioration par générateur.
const TIER_FLAVORS: [&str; 4] = [
    "Un blindage plus solide : moins de pannes, plus de récolte.",
    "Des réglages au micron près pour ne plus rien laisser filer.",
    "On pousse les machines bien au-delà de leurs limites.",
    "Une micro-singularité au cœur de chaque machine.",
];
const GLOBAL_FLAVORS: [&str; 4] = [
    "Courbe la lumière pour repérer les gisements les plus riches.",
    "Des voiles géantes canalisent le vent solaire vers la fonderie.",
    "Toutes les machines vibrent à l'unisson du cosmos.",
    "Le vide lui-même se met à produire.",
];
const CLICK_FLAVORS: [&str; 3] = [
    "Chaque geste ramasse un peu plus de poussière.",
    "Une poigne capable d'arracher la queue d'une comète.",
    "Tout ce que tu touches se change en poussière d'étoile.",
];
const HOLD_FLAVOR: &str = "Un gant motorisé qui récolte tant que tu gardes le doigt appuyé.";

/// Niveaux d'amélioration par générateur : (nom, possédés requis, ×coût de base, facteur).
const GENERATOR_TIERS: [(&str, u64, f64, f64); 4] = [
    ("Alliage renforcé", 10, 15.0, 2.0),
    ("Calibrage fin", 25, 120.0, 2.0),
    ("Surcharge", 50, 1_500.0, 3.0),
    ("Singularité", 100, 40_000.0, 3.0),
];

/// Améliorations globales : (nom, total_run requis, coût, facteur).
const GLOBAL_UPGRADES: [(&str, f64, f64, f64); 4] = [
    ("Lentilles gravitationnelles", 1e4, 5e4, 1.5),
    ("Vent solaire", 1e6, 5e6, 1.5),
    ("Harmonie cosmique", 1e9, 5e9, 2.0),
    ("Éveil du vide", 1e12, 5e12, 2.0),
];

/// Améliorations de clic : (nom, total_run requis, coût, fraction de prod/s).
const CLICK_UPGRADES: [(&str, f64, f64, f64); 3] = [
    ("Gants d'astronaute", 500.0, 1_000.0, 0.01),
    ("Main de Titan", 1e5, 5e5, 0.02),
    ("Toucher divin", 1e8, 5e8, 0.05),
];

/// Amélioration « maintenir Espace pour récolter » : (nom, total_run requis, coût).
const HOLD_UPGRADE: (&str, f64, f64) = ("Récolte continue", 150.0, 300.0);

const ACHIEVEMENTS: [(&str, &str, &str, AchievementCondition); 12] = [
    (
        "first",
        "Premier pas",
        "Acheter un générateur",
        AchievementCondition::GeneratorsOwned(1),
    ),
    (
        "hundred",
        "Centurion",
        "100 exemplaires d'un même générateur",
        AchievementCondition::SameGenerator(100),
    ),
    (
        "all_types",
        "Collectionneur",
        "Au moins un générateur de chaque type",
        AchievementCondition::AllTypes,
    ),
    (
        "m1",
        "Millionnaire",
        "Récolter 1 million de poussières au total",
        AchievementCondition::TotalAll(1e6),
    ),
    (
        "b1",
        "Milliardaire",
        "Récolter 1 milliard de poussières au total",
        AchievementCondition::TotalAll(1e9),
    ),
    (
        "t1",
        "Magnat cosmique",
        "Récolter 1 billion de poussières au total",
        AchievementCondition::TotalAll(1e12),
    ),
    (
        "clicks100",
        "Doigts agiles",
        "Récolter 100 fois à la main",
        AchievementCondition::Clicks(100),
    ),
    (
        "clicks1000",
        "Tendinite stellaire",
        "Récolter 1 000 fois à la main",
        AchievementCondition::Clicks(1_000),
    ),
    (
        "comet1",
        "Chasseur de comètes",
        "Capturer une comète",
        AchievementCondition::CometsCaught(1),
    ),
    (
        "comet10",
        "Astronome confirmé",
        "Capturer 10 comètes",
        AchievementCondition::CometsCaught(10),
    ),
    (
        "nova1",
        "Supernova !",
        "Déclencher une Supernova",
        AchievementCondition::Supernovas(1),
    ),
    (
        "prod1m",
        "Usine infernale",
        "Produire 1 million par seconde",
        AchievementCondition::Production(1e6),
    ),
];

/// Toutes les données statiques du jeu.
#[derive(Clone, Debug, PartialEq)]
pub struct Catalog {
    pub generators: Vec<GeneratorDef>,
    pub upgrades: Vec<UpgradeDef>,
    pub achievements: Vec<AchievementDef>,
}

impl Catalog {
    /// Construit le catalogue standard, de manière déterministe.
    pub fn standard() -> Self {
        let generators: Vec<GeneratorDef> = GENERATORS
            .iter()
            .enumerate()
            .map(
                |(i, &(name, plural, symbol, base_cost, base_prod))| GeneratorDef {
                    id: GeneratorId(i),
                    name,
                    plural,
                    symbol,
                    base_cost,
                    base_prod,
                    description: GENERATOR_FLAVORS.get(i).copied().unwrap_or(""),
                },
            )
            .collect();

        let mut upgrades = Vec::new();
        let mut push = |key: String,
                        name: String,
                        description: String,
                        flavor: &str,
                        cost,
                        requirement,
                        effect| {
            let id = UpgradeId(upgrades.len());
            upgrades.push(UpgradeDef {
                id,
                key,
                name,
                description,
                flavor: flavor.to_string(),
                cost,
                requirement,
                effect,
            });
        };

        for g in &generators {
            for (t, &(tier_name, count, cost_mult, factor)) in GENERATOR_TIERS.iter().enumerate() {
                push(
                    format!("gen{}_t{}", g.id.index(), t),
                    format!("{tier_name} · {}", g.plural),
                    format!("×{} production des {}", fmt_factor(factor), g.plural),
                    TIER_FLAVORS[t],
                    g.base_cost * cost_mult,
                    Requirement::GeneratorOwned {
                        generator: g.id,
                        count,
                    },
                    Effect::GeneratorMultiplier {
                        generator: g.id,
                        factor,
                    },
                );
            }
        }
        for (i, &(name, req, cost, factor)) in GLOBAL_UPGRADES.iter().enumerate() {
            push(
                format!("global_{i}"),
                name.to_string(),
                format!("×{} production globale", fmt_factor(factor)),
                GLOBAL_FLAVORS[i],
                cost,
                Requirement::TotalRun(req),
                Effect::GlobalMultiplier(factor),
            );
        }
        for (i, &(name, req, cost, pct)) in CLICK_UPGRADES.iter().enumerate() {
            push(
                format!("click_{i}"),
                name.to_string(),
                format!("+{}% de la prod/s par clic", (pct * 100.0).round()),
                CLICK_FLAVORS[i],
                cost,
                Requirement::TotalRun(req),
                Effect::ClickPercent(pct),
            );
        }
        let (name, req, cost) = HOLD_UPGRADE;
        push(
            "hold_0".to_string(),
            name.to_string(),
            format!(
                "Maintenir [Espace] : {} récoltes/s",
                (1.0 / balance::HOLD_HARVEST_INTERVAL).round()
            ),
            HOLD_FLAVOR,
            cost,
            Requirement::TotalRun(req),
            Effect::HoldHarvest,
        );

        let achievements = ACHIEVEMENTS
            .iter()
            .enumerate()
            .map(|(i, &(key, name, description, condition))| AchievementDef {
                id: AchievementId(i),
                key,
                name,
                description,
                condition,
            })
            .collect();

        Catalog {
            generators,
            upgrades,
            achievements,
        }
    }

    pub fn generator(&self, id: GeneratorId) -> Option<&GeneratorDef> {
        self.generators.get(id.index())
    }

    pub fn upgrade(&self, id: UpgradeId) -> Option<&UpgradeDef> {
        self.upgrades.get(id.index())
    }

    pub fn achievement(&self, id: AchievementId) -> Option<&AchievementDef> {
        self.achievements.get(id.index())
    }

    pub fn upgrade_by_key(&self, key: &str) -> Option<UpgradeId> {
        self.upgrades.iter().find(|u| u.key == key).map(|u| u.id)
    }

    pub fn achievement_by_key(&self, key: &str) -> Option<AchievementId> {
        self.achievements
            .iter()
            .find(|a| a.key == key)
            .map(|a| a.id)
    }
}

fn fmt_factor(f: f64) -> String {
    if f.fract() == 0.0 {
        format!("{}", f as i64)
    } else {
        format!("{f}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_sizes_and_keys() {
        let c = Catalog::standard();
        assert_eq!(c.generators.len(), 9);
        assert_eq!(c.upgrades.len(), 9 * 4 + 4 + 3 + 1);
        assert!(c.upgrade_by_key("hold_0").is_some());
        assert_eq!(c.achievements.len(), 12);
        assert!(c.upgrade_by_key("gen3_t2").is_some());
        assert!(c.upgrade_by_key("global_1").is_some());
        assert!(c.upgrade_by_key("click_0").is_some());
        assert!(c.achievement_by_key("prod1m").is_some());
        let mut keys: Vec<_> = c.upgrades.iter().map(|u| u.key.clone()).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), c.upgrades.len(), "clés uniques");
    }

    #[test]
    fn upgrade_names_and_descriptions() {
        let c = Catalog::standard();
        let u = &c.upgrades[0];
        assert_eq!(u.name, "Alliage renforcé · Sondes");
        assert_eq!(u.description, "×2 production des Sondes");
        assert_eq!(u.cost, 15.0 * 15.0);
        let g = c.upgrade(c.upgrade_by_key("global_0").unwrap()).unwrap();
        assert_eq!(g.description, "×1.5 production globale");
    }

    #[test]
    fn everything_is_described() {
        let c = Catalog::standard();
        assert!(c.generators.iter().all(|g| !g.description.is_empty()));
        assert!(c
            .upgrades
            .iter()
            .all(|u| !u.flavor.is_empty() && !u.description.is_empty()));
        assert!(c.achievements.iter().all(|a| !a.description.is_empty()));
    }

    #[test]
    fn catalog_is_deterministic() {
        assert_eq!(Catalog::standard(), Catalog::standard());
    }
}
