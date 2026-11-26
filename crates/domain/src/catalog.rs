//! Données du jeu : générateurs, améliorations, succès et constantes d'équilibrage.

use crate::achievement::{AchievementCondition, AchievementDef, AchievementId};
use crate::constellation::{ConstellationDef, ConstellationEffect, ConstellationId};
use crate::generator::{GeneratorDef, GeneratorId};
use crate::perk::{PerkDef, PerkEffect, PerkId};
use crate::upgrade::{Effect, Requirement, UpgradeDef, UpgradeId};

/// Constantes d'équilibrage.
pub mod balance {
    /// Bonus global par fragment de Supernova (+10 %).
    pub const FRAGMENT_BONUS: f64 = 0.10;
    /// Bonus global par succès débloqué (+1 %).
    pub const ACHIEVEMENT_BONUS: f64 = 0.01;
    /// Diviseur de la formule des fragments : `floor(sqrt(total_run / 1e7))`.
    pub const FRAGMENT_DIVISOR: f64 = 1e7;
    /// Chaque Supernova augmente ce diviseur de 25 %.
    pub const FRAGMENT_SCALING: f64 = 0.25;
    /// Au-delà de ce nombre de fragments, leur bonus croît en racine carrée.
    pub const FRAGMENT_KNEE: f64 = 5.0;
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
const GENERATORS: [(&str, &str, char, f64, f64); 12] = [
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
    ("Trou blanc", "Trous blancs", '◎', 7.5e10, 1.6e6),
    (
        "Fabrique de quasars",
        "Fabriques de quasars",
        '✶',
        1.1e12,
        1.0e7,
    ),
    (
        "Moteur à multivers",
        "Moteurs à multivers",
        '▣',
        1.6e13,
        6.5e7,
    ),
];

/// Nombre de Supernovas nécessaires pour débloquer chaque générateur.
const GENERATOR_UNLOCKS: [u32; 12] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 5, 15];

/// Ce que fait chaque générateur (affiché dans le détail de la sélection).
const GENERATOR_FLAVORS: [&str; 12] = [
    "Petite sonde autonome qui aspire la poussière en orbite basse.",
    "Essaim de drones qui ratissent les ceintures d'astéroïdes.",
    "Vaisseau-usine qui fauche les nuages de poussière interstellaire.",
    "Base lunaire qui raffine la poussière à grande échelle.",
    "Coque géante qui capte toute l'énergie d'une étoile.",
    "Fonderie plongée au cœur d'une étoile pour en couler la matière.",
    "Condense des nébuleuses entières en poussière d'étoile pure.",
    "Extrait la poussière des fluctuations du vide quantique.",
    "Tisse des galaxies neuves, fil d'étoile après fil d'étoile.",
    "Recrache la matière qu'avalent les trous noirs, en poussière d'étoile.",
    "Allume des quasars pour en récolter la lumière condensée.",
    "Puise la poussière dans des univers parallèles.",
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

const ACHIEVEMENTS: [(&str, &str, &str, AchievementCondition); 28] = [
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
        AchievementCondition::AllTypes(9),
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
    (
        "dyson1",
        "Ingénieur stellaire",
        "Posséder une Sphère de Dyson",
        AchievementCondition::OwnGenerator { index: 4, count: 1 },
    ),
    (
        "weaver1",
        "Architecte cosmique",
        "Posséder un Tisseur de galaxies",
        AchievementCondition::OwnGenerator { index: 8, count: 1 },
    ),
    (
        "fivehundred",
        "Industriel",
        "500 exemplaires d'un même générateur",
        AchievementCondition::SameGenerator(500),
    ),
    (
        "q1",
        "Empereur galactique",
        "Récolter 1 billiard de poussières au total",
        AchievementCondition::TotalAll(1e15),
    ),
    (
        "clicks10k",
        "Main de fer",
        "Récolter 10 000 fois à la main",
        AchievementCondition::Clicks(10_000),
    ),
    (
        "comet50",
        "Seigneur des comètes",
        "Capturer 50 comètes",
        AchievementCondition::CometsCaught(50),
    ),
    (
        "nova5",
        "Phénix",
        "Déclencher 5 Supernovas",
        AchievementCondition::Supernovas(5),
    ),
    (
        "nova25",
        "Éternel retour",
        "Déclencher 25 Supernovas",
        AchievementCondition::Supernovas(25),
    ),
    (
        "prod1b",
        "Forge divine",
        "Produire 1 milliard par seconde",
        AchievementCondition::Production(1e9),
    ),
    (
        "frag100",
        "Cœur d'étoile",
        "Gagner 100 fragments au total",
        AchievementCondition::Fragments(100.0),
    ),
    (
        "perk1",
        "Apprenti artisan",
        "Acheter une amélioration à l'Atelier stellaire",
        AchievementCondition::PerksOwned(1),
    ),
    (
        "allperks",
        "Maître de l'Atelier",
        "Acheter toutes les améliorations de l'Atelier",
        AchievementCondition::AllPerks,
    ),
    (
        "whitehole1",
        "Fontaine cosmique",
        "Posséder un Trou blanc",
        AchievementCondition::OwnGenerator { index: 9, count: 1 },
    ),
    (
        "multiverse1",
        "Voyageur des mondes",
        "Posséder un Moteur à multivers",
        AchievementCondition::OwnGenerator {
            index: 11,
            count: 1,
        },
    ),
    (
        "const1",
        "Première étoile",
        "Monter une Constellation d'un niveau",
        AchievementCondition::ConstellationLevels(1),
    ),
    (
        "const50",
        "Cartographe du ciel",
        "Atteindre 50 niveaux de Constellations au total",
        AchievementCondition::ConstellationLevels(50),
    ),
];

/// Constellations : (clé, nom, description d'un niveau, symbole, coût de base,
/// croissance du coût, niveau max, effet par niveau).
#[allow(clippy::type_complexity)]
const CONSTELLATIONS: [(
    &str,
    &str,
    &str,
    char,
    f64,
    f64,
    Option<u32>,
    ConstellationEffect,
); 6] = [
    (
        "const_forge",
        "Le Forgeron",
        "+20 % de production par niveau",
        '▲',
        2.0,
        1.6,
        None,
        ConstellationEffect::Production(0.20),
    ),
    (
        "const_hand",
        "La Main",
        "+1 % de la prod/s par clic, par niveau",
        '•',
        1.0,
        1.5,
        None,
        ConstellationEffect::Click(0.01),
    ),
    (
        "const_comet",
        "La Comète",
        "Tempête solaire +1×, Pluie d'étoiles +25 %, par niveau",
        '●',
        2.0,
        1.7,
        Some(20),
        ConstellationEffect::Comets(0.25),
    ),
    (
        "const_hourglass",
        "Le Sablier",
        "+2 h de production hors-ligne par niveau",
        '○',
        1.0,
        1.5,
        Some(12),
        ConstellationEffect::OfflineHours(2.0),
    ),
    (
        "const_nova",
        "La Nova",
        "+10 % de fragments à chaque Supernova, par niveau",
        '★',
        5.0,
        2.0,
        None,
        ConstellationEffect::Fragments(0.10),
    ),
    (
        "const_builder",
        "Le Bâtisseur",
        "Générateurs 3 % moins chers par niveau",
        '◇',
        3.0,
        1.8,
        Some(15),
        ConstellationEffect::Discount(0.97),
    ),
];

/// Atelier stellaire : (clé, nom, description, symbole, coût en fragments, effet).
const PERKS: [(&str, &str, &str, char, f64, PerkEffect); 8] = [
    (
        "perk_start",
        "Héritage",
        "Chaque nouveau cycle commence avec 10 Sondes et 5 Drones",
        '✧',
        1.0,
        PerkEffect::StartingKit {
            probes: 10,
            drones: 5,
        },
    ),
    (
        "perk_click",
        "Poigne stellaire",
        "+2 % de la prod/s à chaque récolte manuelle",
        '•',
        1.0,
        PerkEffect::ClickBoost(0.02),
    ),
    (
        "perk_comet",
        "Aimant à comètes",
        "Les comètes passent 40 % plus souvent",
        '●',
        2.0,
        PerkEffect::CometMagnet(0.6),
    ),
    (
        "perk_storm",
        "Tempête prolongée",
        "Les tempêtes solaires durent 60 s au lieu de 30",
        '✦',
        2.0,
        PerkEffect::LongStorm(2.0),
    ),
    (
        "perk_discount",
        "Ingénierie frugale",
        "Tous les générateurs coûtent 20 % de moins",
        '◇',
        3.0,
        PerkEffect::GeneratorDiscount(0.8),
    ),
    (
        "perk_offline",
        "Résonance",
        "Hors-ligne : production à 100 % (au lieu de 50 %), jusqu'à 24 h",
        '○',
        3.0,
        PerkEffect::OfflineMastery {
            rate: 1.0,
            cap_secs: 24.0 * 3600.0,
        },
    ),
    (
        "perk_auto",
        "Automate",
        "Achète chaque seconde le générateur le plus rentable ([T] pour l'arrêter)",
        '▲',
        5.0,
        PerkEffect::AutoBuyer,
    ),
    (
        "perk_nova",
        "Nova dorée",
        "+50 % de fragments à chaque Supernova",
        '★',
        8.0,
        PerkEffect::RichNova(1.5),
    ),
];

/// Toutes les données statiques du jeu.
#[derive(Clone, Debug, PartialEq)]
pub struct Catalog {
    pub generators: Vec<GeneratorDef>,
    pub upgrades: Vec<UpgradeDef>,
    pub achievements: Vec<AchievementDef>,
    pub perks: Vec<PerkDef>,
    pub constellations: Vec<ConstellationDef>,
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
                    unlock_supernovas: GENERATOR_UNLOCKS.get(i).copied().unwrap_or(0),
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

        let perks = PERKS
            .iter()
            .enumerate()
            .map(
                |(i, &(key, name, description, symbol, cost, effect))| PerkDef {
                    id: PerkId(i),
                    key,
                    name,
                    description,
                    symbol,
                    cost,
                    effect,
                },
            )
            .collect();

        let constellations = CONSTELLATIONS
            .iter()
            .enumerate()
            .map(
                |(i, &(key, name, description, symbol, base_cost, growth, max_level, effect))| {
                    ConstellationDef {
                        id: ConstellationId(i),
                        key,
                        name,
                        description,
                        symbol,
                        base_cost,
                        growth,
                        max_level,
                        effect,
                    }
                },
            )
            .collect();

        Catalog {
            generators,
            upgrades,
            achievements,
            perks,
            constellations,
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

    pub fn perk(&self, id: PerkId) -> Option<&PerkDef> {
        self.perks.get(id.index())
    }

    pub fn constellation(&self, id: ConstellationId) -> Option<&ConstellationDef> {
        self.constellations.get(id.index())
    }

    pub fn constellation_by_key(&self, key: &str) -> Option<ConstellationId> {
        self.constellations
            .iter()
            .find(|c| c.key == key)
            .map(|c| c.id)
    }

    pub fn perk_by_key(&self, key: &str) -> Option<PerkId> {
        self.perks.iter().find(|p| p.key == key).map(|p| p.id)
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
        assert_eq!(c.generators.len(), 12);
        assert_eq!(c.upgrades.len(), 12 * 4 + 4 + 3 + 1);
        assert!(c.upgrade_by_key("hold_0").is_some());
        assert_eq!(c.achievements.len(), 28);
        assert_eq!(c.constellations.len(), 6);
        assert_eq!(c.generators[11].unlock_supernovas, 15);
        assert_eq!(c.perks.len(), 8);
        assert!(c.perk_by_key("perk_auto").is_some());
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
