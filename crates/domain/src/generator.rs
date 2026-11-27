//! Générateurs : définition, coûts, achat multiple et paliers.

/// Identifiant d'un générateur (index dans le catalogue).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GeneratorId(pub usize);

impl GeneratorId {
    pub fn index(self) -> usize {
        self.0
    }
}

/// Croissance du coût à chaque exemplaire acheté.
pub const COST_GROWTH: f64 = 1.15;

/// Nombres d'exemplaires qui doublent la production du générateur (cumulatif).
pub const MILESTONES: [u64; 7] = [25, 50, 100, 200, 300, 400, 500];

#[derive(Clone, Debug, PartialEq)]
pub struct GeneratorDef {
    pub id: GeneratorId,
    pub name: &'static str,
    pub plural: &'static str,
    pub symbol: char,
    pub base_cost: f64,
    pub base_prod: f64,
    /// Ce que fait ce générateur, en une phrase.
    pub description: &'static str,
    /// Supernovas nécessaires pour qu'il apparaisse (0 = dès le début).
    pub unlock_supernovas: u32,
    /// Générateur primordial : particules à dépenser pour le forger (0 = aucun).
    pub particle_cost: u32,
}

impl GeneratorDef {
    /// Vrai pour les générateurs primordiaux (forgés avec des particules).
    pub fn is_primordial(&self) -> bool {
        self.particle_cost > 0
    }
}

impl GeneratorDef {
    /// Coût du prochain exemplaire quand on en possède `owned`.
    pub fn unit_cost(&self, owned: u64) -> f64 {
        self.base_cost * COST_GROWTH.powf(owned as f64)
    }

    /// Coût de `n` exemplaires : `base × 1.15^owned × (1.15^n − 1) / 0.15`.
    pub fn cost_for(&self, owned: u64, n: u64) -> f64 {
        if n == 0 {
            return 0.0;
        }
        self.unit_cost(owned) * (COST_GROWTH.powf(n as f64) - 1.0) / (COST_GROWTH - 1.0)
    }

    /// Nombre maximal d'exemplaires achetables avec `stardust`.
    pub fn max_affordable(&self, owned: u64, stardust: f64) -> u64 {
        let first = self.unit_cost(owned);
        if !stardust.is_finite() || stardust < first {
            return 0;
        }
        let raw = (stardust * (COST_GROWTH - 1.0) / first + 1.0).ln() / COST_GROWTH.ln();
        let mut n = if raw.is_finite() && raw > 0.0 {
            raw.floor().min(1e6) as u64
        } else {
            0
        };
        // Corrige les erreurs d'arrondi flottant dans les deux sens.
        while n > 0 && self.cost_for(owned, n) > stardust {
            n -= 1;
        }
        while n < 1_000_000 && self.cost_for(owned, n + 1) <= stardust {
            n += 1;
        }
        n
    }
}

/// Nombre de paliers atteints pour `owned` exemplaires.
pub fn milestones_reached(owned: u64) -> u32 {
    MILESTONES.iter().filter(|&&m| owned >= m).count() as u32
}

/// Prochain palier au-dessus de `owned` (aucun au-delà de 500).
pub fn next_milestone(owned: u64) -> Option<u64> {
    MILESTONES.into_iter().find(|&m| m > owned)
}

/// Paliers franchis en passant de `before` à `after` exemplaires.
pub fn milestones_crossed(before: u64, after: u64) -> impl Iterator<Item = u64> {
    MILESTONES
        .into_iter()
        .filter(move |&m| before < m && after >= m)
}

/// Mode d'achat : ×1 → ×10 → ×25 → Palier → MAX.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BuyMode {
    #[default]
    One,
    Ten,
    TwentyFive,
    /// Jusqu'au prochain palier de bonus (25, 50, 100… 500 exemplaires) ;
    /// au-delà du dernier, un par un.
    NextMilestone,
    Max,
}

impl BuyMode {
    pub fn next(self) -> Self {
        match self {
            BuyMode::One => BuyMode::Ten,
            BuyMode::Ten => BuyMode::TwentyFive,
            BuyMode::TwentyFive => BuyMode::NextMilestone,
            BuyMode::NextMilestone => BuyMode::Max,
            BuyMode::Max => BuyMode::One,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            BuyMode::One => "×1",
            BuyMode::Ten => "×10",
            BuyMode::TwentyFive => "×25",
            BuyMode::NextMilestone => "PALIER",
            BuyMode::Max => "MAX",
        }
    }

    /// Étiquette courte (boutons étroits du mode mobile).
    pub fn short_label(self) -> &'static str {
        match self {
            BuyMode::NextMilestone => "PAL.",
            other => other.label(),
        }
    }

    /// Quantité à acheter dans ce mode. En MAX, au moins 1 (pour l'affichage du coût).
    pub fn quantity(self, def: &GeneratorDef, owned: u64, stardust: f64) -> u64 {
        match self {
            BuyMode::One => 1,
            BuyMode::Ten => 10,
            BuyMode::TwentyFive => 25,
            BuyMode::NextMilestone => next_milestone(owned).map_or(1, |m| m - owned),
            BuyMode::Max => def.max_affordable(owned, stardust).max(1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe() -> GeneratorDef {
        GeneratorDef {
            id: GeneratorId(0),
            name: "Sonde",
            plural: "Sondes",
            symbol: '✧',
            base_cost: 15.0,
            base_prod: 0.1,
            description: "",
            unlock_supernovas: 0,
            particle_cost: 0,
        }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
    }

    #[test]
    fn unit_cost_grows_by_15_percent() {
        let g = probe();
        assert!(close(g.unit_cost(0), 15.0));
        assert!(close(g.unit_cost(1), 17.25));
        assert!(close(g.unit_cost(10), 15.0 * 1.15f64.powi(10)));
    }

    #[test]
    fn batch_cost_equals_sum_of_units() {
        let g = probe();
        for owned in [0u64, 3, 42] {
            for n in [1u64, 10, 25] {
                let sum: f64 = (0..n).map(|k| g.unit_cost(owned + k)).sum();
                assert!(close(g.cost_for(owned, n), sum), "owned={owned} n={n}");
            }
        }
        assert_eq!(g.cost_for(5, 0), 0.0);
    }

    #[test]
    fn max_affordable_is_exact() {
        let g = probe();
        assert_eq!(g.max_affordable(0, 14.9), 0);
        assert_eq!(g.max_affordable(0, 15.0), 1);
        for budget in [15.0, 100.0, 1234.5, 1e6, 1e12] {
            let n = g.max_affordable(7, budget);
            assert!(g.cost_for(7, n) <= budget);
            assert!(g.cost_for(7, n + 1) > budget);
        }
    }

    #[test]
    fn max_mode_shows_at_least_one() {
        let g = probe();
        assert_eq!(BuyMode::Max.quantity(&g, 0, 0.0), 1);
        assert_eq!(BuyMode::Max.quantity(&g, 0, 32.25), 2);
        assert_eq!(BuyMode::Ten.quantity(&g, 0, 0.0), 10);
    }

    #[test]
    fn buy_mode_cycles() {
        let mut m = BuyMode::One;
        let mut labels = Vec::new();
        for _ in 0..6 {
            labels.push(m.label());
            m = m.next();
        }
        assert_eq!(labels, ["×1", "×10", "×25", "PALIER", "MAX", "×1"]);
    }

    #[test]
    fn next_milestone_mode_fills_up_to_the_bonus() {
        let g = probe();
        let q = |owned| BuyMode::NextMilestone.quantity(&g, owned, 0.0);
        assert_eq!(q(0), 25);
        assert_eq!(q(24), 1);
        assert_eq!(q(25), 25, "25 → 50");
        assert_eq!(q(130), 70, "130 → 200");
        assert_eq!(q(499), 1);
        assert_eq!(q(500), 1, "plus de palier : un par un");
        assert_eq!(next_milestone(500), None);
    }

    #[test]
    fn milestones() {
        assert_eq!(milestones_reached(24), 0);
        assert_eq!(milestones_reached(25), 1);
        assert_eq!(milestones_reached(100), 3);
        assert_eq!(milestones_reached(10_000), 7);
        let crossed: Vec<_> = milestones_crossed(20, 60).collect();
        assert_eq!(crossed, [25, 50]);
    }
}
