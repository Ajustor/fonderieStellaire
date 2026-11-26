//! Value noise 2D et fBm, assez léger pour être évalué par cellule à 30 FPS.

use super::rng::hash3;

fn smooth(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

/// Value noise 2D dans [0, 1].
pub fn value_noise(x: f64, y: f64, seed: i64) -> f64 {
    let x0 = x.floor();
    let y0 = y.floor();
    let fx = smooth(x - x0);
    let fy = smooth(y - y0);
    let (ix, iy) = (x0 as i64, y0 as i64);
    let a = hash3(ix, iy, seed);
    let b = hash3(ix + 1, iy, seed);
    let c = hash3(ix, iy + 1, seed);
    let d = hash3(ix + 1, iy + 1, seed);
    let top = a + (b - a) * fx;
    let bottom = c + (d - c) * fx;
    top + (bottom - top) * fy
}

/// Bruit fractal (somme d'octaves), normalisé dans [0, 1].
pub fn fbm(x: f64, y: f64, octaves: u32, seed: i64) -> f64 {
    let mut sum = 0.0;
    let mut amp = 1.0;
    let mut norm = 0.0;
    let mut freq = 1.0;
    for o in 0..octaves.max(1) {
        sum += amp * value_noise(x * freq, y * freq, seed + o as i64 * 97);
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_and_determinism() {
        for i in 0..200 {
            let x = i as f64 * 0.37;
            let y = i as f64 * 0.11;
            let v = fbm(x, y, 3, 7);
            assert!((0.0..=1.0).contains(&v));
            assert_eq!(v, fbm(x, y, 3, 7));
        }
    }

    #[test]
    fn continuous() {
        let a = value_noise(3.5, 2.5, 1);
        let b = value_noise(3.5001, 2.5, 1);
        assert!((a - b).abs() < 1e-3);
    }

    #[test]
    fn lattice_points_match_hash() {
        assert_eq!(value_noise(2.0, 5.0, 3), hash3(2, 5, 3));
    }
}
