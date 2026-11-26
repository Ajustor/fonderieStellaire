use std::time::{SystemTime, UNIX_EPOCH};

use application::RandomSource;

/// Générateur pseudo-aléatoire xorshift64 (pas de crate `rand`).
#[derive(Clone, Debug)]
pub struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    pub fn new(seed: u64) -> Self {
        // L'état ne doit jamais valoir 0 ; on mélange la graine (splitmix64).
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        XorShift64 {
            state: if z == 0 { 0x2545_F491_4F6C_DD1D } else { z },
        }
    }

    /// Graine tirée de l'heure système.
    pub fn from_time() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x1234_5678);
        Self::new(nanos)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
}

impl RandomSource for XorShift64 {
    fn next_f64(&mut self) -> f64 {
        // 53 bits de mantisse → [0, 1).
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_in_range() {
        let mut a = XorShift64::new(42);
        let mut b = XorShift64::new(42);
        for _ in 0..1000 {
            let x = a.next_f64();
            assert_eq!(x, b.next_f64());
            assert!((0.0..1.0).contains(&x));
        }
    }

    #[test]
    fn zero_seed_is_fine_and_spread() {
        let mut r = XorShift64::new(0);
        let mean: f64 = (0..10_000).map(|_| r.next_f64()).sum::<f64>() / 10_000.0;
        assert!((mean - 0.5).abs() < 0.02, "{mean}");
    }
}
