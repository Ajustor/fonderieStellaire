//! Couleurs RGB : interpolation, dégradés, HSL, assombrissement et halo.

use ratatui::style::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// Couleur supposée du texte par défaut du terminal (pour les cellules `Reset`).
pub const DEFAULT_FG: Rgb = Rgb(200, 200, 210);
/// Couleur supposée du fond du terminal.
pub const DEFAULT_BG: Rgb = Rgb(0, 0, 0);

impl Rgb {
    pub fn color(self) -> Color {
        Color::Rgb(self.0, self.1, self.2)
    }

    /// Convertit une couleur ratatui ; `None` pour les couleurs non RGB.
    pub fn from_color(c: Color) -> Option<Rgb> {
        match c {
            Color::Rgb(r, g, b) => Some(Rgb(r, g, b)),
            Color::White => Some(Rgb(255, 255, 255)),
            Color::Black => Some(Rgb(0, 0, 0)),
            Color::Gray => Some(Rgb(170, 170, 170)),
            Color::DarkGray => Some(Rgb(90, 90, 90)),
            _ => None,
        }
    }

    pub fn lerp(self, other: Rgb, t: f64) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        let m = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round() as u8;
        Rgb(m(self.0, other.0), m(self.1, other.1), m(self.2, other.2))
    }

    /// Assombrit : `factor = 0` → inchangé, `1` → noir.
    pub fn dim(self, factor: f64) -> Rgb {
        self.lerp(Rgb(0, 0, 0), factor)
    }

    /// Éclaircit vers le blanc : `factor = 0` → inchangé, `1` → blanc.
    pub fn glow(self, factor: f64) -> Rgb {
        self.lerp(Rgb(255, 255, 255), factor)
    }

    /// Multiplie la luminosité (peut saturer à 255).
    pub fn scale(self, k: f64) -> Rgb {
        let m = |a: u8| (a as f64 * k).round().clamp(0.0, 255.0) as u8;
        Rgb(m(self.0), m(self.1), m(self.2))
    }

    pub fn luma(self) -> f64 {
        (0.2126 * self.0 as f64 + 0.7152 * self.1 as f64 + 0.0722 * self.2 as f64) / 255.0
    }
}

/// Dégradé à arrêts régulièrement espacés, `t ∈ [0, 1]`.
pub fn gradient(stops: &[Rgb], t: f64) -> Rgb {
    match stops.len() {
        0 => DEFAULT_FG,
        1 => stops[0],
        n => {
            let t = t.clamp(0.0, 1.0) * (n - 1) as f64;
            let i = (t.floor() as usize).min(n - 2);
            stops[i].lerp(stops[i + 1], t - i as f64)
        }
    }
}

/// Dégradé cyclique (le dernier arrêt rejoint le premier) : `t` est pris modulo 1.
pub fn gradient_cyclic(stops: &[Rgb], t: f64) -> Rgb {
    let n = stops.len();
    if n == 0 {
        return DEFAULT_FG;
    }
    let t = t.rem_euclid(1.0) * n as f64;
    let i = (t.floor() as usize) % n;
    stops[i].lerp(stops[(i + 1) % n], t - t.floor())
}

/// HSL → RGB, avec `h` en degrés, `s` et `l` dans [0, 1].
pub fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Rgb {
    let h = h.rem_euclid(360.0) / 360.0;
    let s = s.clamp(0.0, 1.0);
    let l = l.clamp(0.0, 1.0);
    if s == 0.0 {
        let v = (l * 255.0).round() as u8;
        return Rgb(v, v, v);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let hue = |mut t: f64| {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    let c = |v: f64| (v * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgb(c(hue(h + 1.0 / 3.0)), c(hue(h)), c(hue(h - 1.0 / 3.0)))
}

/// RGB → HSL (`h` en degrés).
pub fn rgb_to_hsl(c: Rgb) -> (f64, f64, f64) {
    let r = c.0 as f64 / 255.0;
    let g = c.1 as f64 / 255.0;
    let b = c.2 as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < f64::EPSILON {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, s, l)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_endpoints() {
        let a = Rgb(0, 100, 200);
        let b = Rgb(200, 100, 0);
        assert_eq!(a.lerp(b, 0.0), a);
        assert_eq!(a.lerp(b, 1.0), b);
        assert_eq!(a.lerp(b, 0.5), Rgb(100, 100, 100));
    }

    #[test]
    fn gradients() {
        let stops = [Rgb(0, 0, 0), Rgb(100, 0, 0), Rgb(100, 100, 0)];
        assert_eq!(gradient(&stops, 0.0), stops[0]);
        assert_eq!(gradient(&stops, 0.5), stops[1]);
        assert_eq!(gradient(&stops, 1.0), stops[2]);
        assert_eq!(gradient_cyclic(&stops, 1.0), stops[0]);
        assert_eq!(gradient_cyclic(&stops, -1.0 / 3.0), stops[2]);
    }

    #[test]
    fn hsl_round_trip() {
        for c in [
            Rgb(255, 0, 0),
            Rgb(10, 200, 120),
            Rgb(90, 90, 140),
            Rgb(255, 200, 80),
        ] {
            let (h, s, l) = rgb_to_hsl(c);
            let back = hsl_to_rgb(h, s, l);
            for (x, y) in [(c.0, back.0), (c.1, back.1), (c.2, back.2)] {
                assert!((x as i32 - y as i32).abs() <= 1, "{c:?} → {back:?}");
            }
        }
    }

    #[test]
    fn dim_and_glow() {
        assert_eq!(Rgb(100, 100, 100).dim(1.0), Rgb(0, 0, 0));
        assert_eq!(Rgb(100, 100, 100).glow(1.0), Rgb(255, 255, 255));
        assert_eq!(Rgb(10, 20, 30).dim(0.0), Rgb(10, 20, 30));
    }
}
