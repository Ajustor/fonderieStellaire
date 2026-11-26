//! Acte III : la Carte stellaire. Une carte générée à partir d'une graine
//! (propre à chaque partie) : des systèmes reliés par des routes, en colonnes
//! de plus en plus lointaines et dangereuses. On les explore en envoyant des
//! flottes en expédition, en temps réel.

/// Type d'un système : ce que son exploration rapporte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemKind {
    /// Production permanente.
    Nebula,
    /// Fragments permanents.
    Cluster,
    /// Hors-ligne : plafond plus long.
    BlackHole,
    /// Comètes plus fréquentes.
    Pulsar,
    /// Une flotte de plus.
    Civilization,
    /// Gain immédiat : des heures de production d'un coup.
    Anomaly,
}

impl SystemKind {
    pub fn symbol(self) -> char {
        match self {
            SystemKind::Nebula => '✦',
            SystemKind::Cluster => '★',
            SystemKind::BlackHole => '●',
            SystemKind::Pulsar => '◎',
            SystemKind::Civilization => '▲',
            SystemKind::Anomaly => '◈',
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SystemKind::Nebula => "Nébuleuse",
            SystemKind::Cluster => "Amas d'étoiles",
            SystemKind::BlackHole => "Trou noir",
            SystemKind::Pulsar => "Pulsar",
            SystemKind::Civilization => "Civilisation",
            SystemKind::Anomaly => "Anomalie",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StarSystem {
    pub index: usize,
    pub name: String,
    /// Colonne (0 = la fonderie) et position dans la colonne (0..1).
    pub column: usize,
    pub y: f64,
    pub kind: SystemKind,
    /// Danger, de 1 à 5 : durée, risque et récompense.
    pub danger: u8,
    /// Systèmes reliés (dans les deux sens).
    pub links: Vec<usize>,
}

impl StarSystem {
    /// Durée d'une expédition vers ce système (secondes).
    pub fn duration(&self) -> f64 {
        (0.5 + 1.5 * self.danger as f64 + 0.25 * self.column as f64) * 3600.0
    }

    /// Multiplicateur de récompense selon le danger (1 → ×1, 5 → ×3).
    pub fn reward_scale(&self) -> f64 {
        0.5 + 0.5 * self.danger as f64
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StarMap {
    pub systems: Vec<StarSystem>,
    pub columns: usize,
}

/// Nombre de colonnes de la carte (la fonderie comprise).
pub const COLUMNS: usize = 8;
/// Systèmes par colonne (hors fonderie).
const PER_COLUMN: [usize; COLUMNS] = [1, 3, 4, 4, 4, 4, 3, 2];
/// Flottes au départ ; chaque Civilisation explorée en ajoute une.
pub const BASE_FLEETS: usize = 2;

const SYLLABLES: [&str; 24] = [
    "Ka", "Vel", "Or", "Zé", "Thal", "Ny", "Ar", "Sé", "Lu", "Mor", "Iri", "Dra", "phy", "ris",
    "on", "ta", "mé", "lax", "nor", "ven", "ka", "dis", "tra", "lis",
];
const GREEK: [&str; 8] = ["α", "β", "γ", "δ", "ε", "ζ", "η", "θ"];

/// Hasard déterministe (splitmix64) : même graine, même carte.
struct Mix(u64);

impl Mix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn f(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

fn name(rng: &mut Mix) -> String {
    let parts = 2 + rng.below(2);
    let mut s = String::new();
    for i in 0..parts {
        let syl = SYLLABLES[rng.below(SYLLABLES.len())];
        if i == 0 {
            s.push_str(syl);
        } else {
            s.push_str(&syl.to_lowercase());
        }
    }
    format!("{s} {}", GREEK[rng.below(GREEK.len())])
}

/// Génère la carte d'une partie.
pub fn generate(seed: u64) -> StarMap {
    let mut rng = Mix(seed ^ 0xC0FF_EE00_D15C_0000);
    let mut systems: Vec<StarSystem> = Vec::new();
    let mut columns: Vec<Vec<usize>> = Vec::new();
    // Types garantis sur la carte : deux Civilisations (flottes), le reste tiré.
    let mut civ_columns = [2usize, 5];
    for (col, &count) in PER_COLUMN.iter().enumerate() {
        let mut ids = Vec::new();
        for k in 0..count {
            let index = systems.len();
            let y = (k as f64 + 0.5 + (rng.f() - 0.5) * 0.5) / count as f64;
            let (kind, danger, name) = if col == 0 {
                (SystemKind::Nebula, 0, "Fonderie Stellaire".to_string())
            } else {
                let kind = if civ_columns.contains(&col) && k == 0 {
                    civ_columns.iter_mut().for_each(|c| {
                        if *c == col {
                            *c = usize::MAX;
                        }
                    });
                    SystemKind::Civilization
                } else {
                    match rng.below(10) {
                        0..=2 => SystemKind::Nebula,
                        3..=4 => SystemKind::Cluster,
                        5..=6 => SystemKind::Pulsar,
                        7 => SystemKind::BlackHole,
                        _ => SystemKind::Anomaly,
                    }
                };
                let base = 1 + (col - 1) * 4 / (COLUMNS - 1);
                let danger = (base + rng.below(2)).clamp(1, 5) as u8;
                (kind, danger, name(&mut rng))
            };
            systems.push(StarSystem {
                index,
                name,
                column: col,
                y,
                kind,
                danger,
                links: Vec::new(),
            });
            ids.push(index);
        }
        columns.push(ids);
    }
    // Routes : chaque système est relié à au moins un de la colonne suivante,
    // et chaque système (hors fonderie) a au moins un lien vers la précédente.
    let link = |systems: &mut Vec<StarSystem>, a: usize, b: usize| {
        if !systems[a].links.contains(&b) {
            systems[a].links.push(b);
            systems[b].links.push(a);
        }
    };
    for col in 0..COLUMNS - 1 {
        let (here, next) = (&columns[col], &columns[col + 1]);
        for &a in here {
            // Le plus proche en hauteur, plus parfois un second.
            let ya = systems[a].y;
            let mut sorted = next.clone();
            sorted.sort_by(|&x, &y| {
                (systems[x].y - ya)
                    .abs()
                    .partial_cmp(&(systems[y].y - ya).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            link(&mut systems, a, sorted[0]);
            if sorted.len() > 1 && (col == 0 || rng.f() < 0.45) {
                link(&mut systems, a, sorted[1]);
            }
            if col == 0 {
                for &b in next.iter() {
                    link(&mut systems, a, b);
                }
            }
        }
        for &b in next {
            let has_back = systems[b].links.iter().any(|l| systems[*l].column == col);
            if !has_back {
                let yb = systems[b].y;
                let nearest = *here
                    .iter()
                    .min_by(|&&x, &&y| {
                        (systems[x].y - yb)
                            .abs()
                            .partial_cmp(&(systems[y].y - yb).abs())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .unwrap_or(&here[0]);
                link(&mut systems, nearest, b);
            }
        }
    }
    StarMap {
        systems,
        columns: COLUMNS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_is_deterministic_connected_and_sized() {
        let a = generate(42);
        assert_eq!(a, generate(42));
        assert_ne!(a, generate(43));
        assert_eq!(a.systems.len(), PER_COLUMN.iter().sum::<usize>());
        assert_eq!(a.systems[0].column, 0);
        // Tout est atteignable depuis la fonderie.
        let mut seen = vec![false; a.systems.len()];
        let mut stack = vec![0];
        while let Some(i) = stack.pop() {
            if std::mem::replace(&mut seen[i], true) {
                continue;
            }
            stack.extend(a.systems[i].links.iter().copied());
        }
        assert!(seen.iter().all(|&s| s), "carte connexe");
        let civs = a
            .systems
            .iter()
            .filter(|s| s.kind == SystemKind::Civilization)
            .count();
        assert_eq!(civs, 2, "deux flottes à gagner");
        assert!(a.systems[1..].iter().all(|s| (1..=5).contains(&s.danger)));
    }

    #[test]
    fn danger_sets_duration_and_reward() {
        let m = generate(7);
        let s = &m.systems[1];
        assert!(s.duration() >= 3600.0);
        let far = m.systems.last().unwrap();
        assert!(far.duration() > s.duration());
        assert!(far.reward_scale() >= s.reward_scale());
    }
}
