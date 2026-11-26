//! Petit générateur pseudo-aléatoire pour les effets visuels (purement cosmétique,
//! indépendant de l'aléa du jeu) et fonctions de hachage déterministes.

#[derive(Clone, Debug)]
pub struct FxRng(u64);

impl FxRng {
    pub fn new(seed: u64) -> Self {
        FxRng(hash64(seed) | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniforme dans [0, 1).
    pub fn f(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.f()
    }

    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
}

/// Hachage 64 bits (splitmix64).
pub fn hash64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Hachage de trois entiers vers [0, 1).
pub fn hash3(a: i64, b: i64, c: i64) -> f64 {
    let h = hash64((a as u64).wrapping_mul(0x9E37_79B1) ^ hash64((b as u64) ^ hash64(c as u64)));
    (h >> 11) as f64 / (1u64 << 53) as f64
}
