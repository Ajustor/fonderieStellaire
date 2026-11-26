//! Acte IV : les Mégastructures. Une grille où l'on bâtit, avec de la matière
//! exotique (qui s'accumule en temps réel), des structures dont l'effet dépend
//! de leurs voisines.

/// Taille de la grille.
pub const GRID_W: usize = 9;
pub const GRID_H: usize = 5;
pub const CELLS: usize = GRID_W * GRID_H;
/// Matière exotique produite d'office, par heure.
pub const BASE_EXOTIC_PER_HOUR: f64 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StructureKind {
    Collector,
    Relay,
    WorldForge,
    Archive,
    Beacon,
    Core,
}

impl StructureKind {
    pub const ALL: [StructureKind; 6] = [
        StructureKind::Collector,
        StructureKind::Relay,
        StructureKind::WorldForge,
        StructureKind::Archive,
        StructureKind::Beacon,
        StructureKind::Core,
    ];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&k| k == self).unwrap_or(0)
    }

    pub fn from_index(i: usize) -> Option<StructureKind> {
        Self::ALL.get(i).copied()
    }

    pub fn symbol(self) -> char {
        match self {
            StructureKind::Collector => '◇',
            StructureKind::Relay => '◆',
            StructureKind::WorldForge => '▲',
            StructureKind::Archive => '★',
            StructureKind::Beacon => '◎',
            StructureKind::Core => '✶',
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            StructureKind::Collector => "Collecteur",
            StructureKind::Relay => "Relais",
            StructureKind::WorldForge => "Forge-monde",
            StructureKind::Archive => "Archive",
            StructureKind::Beacon => "Phare",
            StructureKind::Core => "Cœur stellaire",
        }
    }

    /// Ce que fait la structure, et comment ses voisines la renforcent.
    pub fn description(self) -> &'static str {
        match self {
            StructureKind::Collector => {
                "+1 matière exotique par heure ; chaque Relais voisin la renforce de 25 %."
            }
            StructureKind::Relay => {
                "Aucun effet propre : renforce de 25 % chaque structure voisine."
            }
            StructureKind::WorldForge => "+25 % de production ; ×1,5 si elle touche un Collecteur.",
            StructureKind::Archive => "+20 % de fragments ; ×1,5 si elle touche une Forge-monde.",
            StructureKind::Beacon => {
                "Expéditions −4 % de durée et +2 % de succès ; ×2 au bord de la grille."
            }
            StructureKind::Core => {
                "Unique. Production +50 % pour chaque type de structure différent qui le touche."
            }
        }
    }

    /// Coût de base en matière exotique et croissance par exemplaire déjà bâti.
    pub fn cost(self, built: usize) -> f64 {
        let (base, growth) = match self {
            StructureKind::Collector => (3.0, 1.5),
            StructureKind::Relay => (8.0, 1.7),
            StructureKind::WorldForge => (6.0, 1.6),
            StructureKind::Archive => (6.0, 1.6),
            StructureKind::Beacon => (5.0, 1.6),
            StructureKind::Core => (60.0, 1.0),
        };
        base * f64::powi(growth, built as i32)
    }

    pub fn unique(self) -> bool {
        self == StructureKind::Core
    }
}

/// Voisines (haut, bas, gauche, droite) d'une case.
pub fn neighbors(cell: usize) -> impl Iterator<Item = usize> {
    let (x, y) = ((cell % GRID_W) as i32, (cell / GRID_W) as i32);
    [(0, -1), (0, 1), (-1, 0), (1, 0)]
        .into_iter()
        .filter_map(move |(dx, dy)| {
            let (nx, ny) = (x + dx, y + dy);
            (nx >= 0 && ny >= 0 && (nx as usize) < GRID_W && (ny as usize) < GRID_H)
                .then(|| ny as usize * GRID_W + nx as usize)
        })
}

pub fn on_edge(cell: usize) -> bool {
    let (x, y) = (cell % GRID_W, cell / GRID_W);
    x == 0 || y == 0 || x + 1 == GRID_W || y + 1 == GRID_H
}

/// Effets cumulés de la grille.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct GridEffects {
    pub exotic_per_hour: f64,
    pub production: f64,
    pub fragments: f64,
    pub expedition_speed: f64,
    pub expedition_luck: f64,
}

/// Renfort des Relais voisins d'une case : ×1,25 par Relais.
fn relay_boost(grid: &[Option<StructureKind>], cell: usize) -> f64 {
    1.25f64.powi(
        neighbors(cell)
            .filter(|&n| grid.get(n).copied().flatten() == Some(StructureKind::Relay))
            .count() as i32,
    )
}

fn touches(grid: &[Option<StructureKind>], cell: usize, kind: StructureKind) -> bool {
    neighbors(cell).any(|n| grid.get(n).copied().flatten() == Some(kind))
}

/// Effet propre d'une case (pour l'affichage) et total de la grille.
pub fn cell_strength(grid: &[Option<StructureKind>], cell: usize) -> f64 {
    let Some(kind) = grid.get(cell).copied().flatten() else {
        return 0.0;
    };
    let boost = relay_boost(grid, cell);
    match kind {
        StructureKind::Collector => 1.0 * boost,
        StructureKind::Relay => 0.0,
        StructureKind::WorldForge => {
            0.25 * boost
                * if touches(grid, cell, StructureKind::Collector) {
                    1.5
                } else {
                    1.0
                }
        }
        StructureKind::Archive => {
            0.20 * boost
                * if touches(grid, cell, StructureKind::WorldForge) {
                    1.5
                } else {
                    1.0
                }
        }
        StructureKind::Beacon => boost * if on_edge(cell) { 2.0 } else { 1.0 },
        StructureKind::Core => {
            let mut kinds: Vec<StructureKind> = neighbors(cell)
                .filter_map(|n| grid.get(n).copied().flatten())
                .collect();
            kinds.sort_by_key(|k| k.index());
            kinds.dedup();
            0.5 * kinds.len() as f64 * boost
        }
    }
}

pub fn effects(grid: &[Option<StructureKind>]) -> GridEffects {
    let mut e = GridEffects {
        exotic_per_hour: BASE_EXOTIC_PER_HOUR,
        production: 1.0,
        fragments: 1.0,
        expedition_speed: 0.0,
        expedition_luck: 0.0,
    };
    let mut forge = 0.0;
    let mut core = 1.0;
    for (cell, kind) in grid.iter().enumerate() {
        let Some(kind) = kind else { continue };
        let s = cell_strength(grid, cell);
        match kind {
            StructureKind::Collector => e.exotic_per_hour += s,
            StructureKind::Relay => {}
            StructureKind::WorldForge => forge += s,
            StructureKind::Archive => e.fragments += s,
            StructureKind::Beacon => {
                e.expedition_speed += 0.04 * s;
                e.expedition_luck += 0.02 * s;
            }
            StructureKind::Core => core += s,
        }
    }
    e.production = (1.0 + forge) * core;
    e.expedition_speed = e.expedition_speed.min(0.5);
    e
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> Vec<Option<StructureKind>> {
        vec![None; CELLS]
    }

    #[test]
    fn neighbors_and_edges() {
        assert_eq!(neighbors(0).count(), 2);
        assert_eq!(neighbors(GRID_W + 1).count(), 4);
        assert!(on_edge(0) && !on_edge(GRID_W + 1));
    }

    #[test]
    fn adjacency_bonuses() {
        let mut g = grid();
        let c = GRID_W + 2;
        g[c] = Some(StructureKind::Collector);
        assert_eq!(effects(&g).exotic_per_hour, 2.0);
        g[c + 1] = Some(StructureKind::Relay);
        assert_eq!(effects(&g).exotic_per_hour, 1.0 + 1.25);
        // Une Forge-monde qui touche le Collecteur : ×1,5.
        g[c - 1] = Some(StructureKind::WorldForge);
        assert!((effects(&g).production - 1.375).abs() < 1e-9);
        // Le Cœur : +50 % par type différent autour de lui.
        let core = c + GRID_W;
        g[core] = Some(StructureKind::Core);
        g[core - 1] = Some(StructureKind::Archive);
        // Autour du Cœur : Collecteur (haut) et Archive (gauche) → ×2.
        let e = effects(&g);
        assert!(
            (e.production - 1.375 * 2.0).abs() < 1e-9,
            "{}",
            e.production
        );
        assert_eq!(StructureKind::Collector.cost(2), 3.0 * 1.5 * 1.5);
    }
}
