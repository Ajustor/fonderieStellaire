//! État des effets visuels de l'écran de jeu (vit uniquement dans la présentation).

use std::f64::consts::TAU;

use ratatui::layout::Rect;

use crate::fx::easing::Easing;
use crate::fx::particles::{Glyph, Particle};
use crate::fx::{FxRng, ParticleSystem, Rgb, Tween};
use crate::screens::supernova::NovaSequence;
use crate::theme;

/// « Jamais » pour les horodatages d'effets.
pub const NEVER: f64 = -1.0e9;
pub const MAX_PARTICLES: usize = 400;
const MAX_FLOATERS: usize = 24;
pub const FLOATER_LIFE: f64 = 1.2;

#[derive(Clone, Copy, Debug)]
pub struct Star {
    /// Position normalisée dans [0, 1).
    pub x: f64,
    pub y: f64,
    /// 0 = lointaine, 2 = proche.
    pub layer: u8,
    pub phase: f64,
    pub speed: f64,
}

/// Champ d'étoiles déterministe, en coordonnées normalisées (indépendant de la taille).
#[derive(Clone, Debug)]
pub struct StarField {
    pub stars: Vec<Star>,
}

impl StarField {
    pub fn new(n: usize, seed: u64) -> Self {
        let mut rng = FxRng::new(seed);
        let stars = (0..n)
            .map(|_| {
                let r = rng.f();
                Star {
                    x: rng.f(),
                    y: rng.f(),
                    layer: if r < 0.55 {
                        0
                    } else if r < 0.85 {
                        1
                    } else {
                        2
                    },
                    phase: rng.range(0.0, TAU),
                    speed: rng.range(0.8, 3.2),
                }
            })
            .collect();
        StarField { stars }
    }
}

/// Texte « +X » qui monte et s'estompe.
#[derive(Clone, Debug)]
pub struct Floater {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub born: f64,
    pub color: Rgb,
}

#[derive(Clone, Copy, Debug)]
pub struct ShootingStar {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub born: f64,
    pub life: f64,
}

/// Gerbe programmée (feux d'artifice), en coordonnées normalisées du ciel.
#[derive(Clone, Copy, Debug)]
pub struct PendingBurst {
    pub at: f64,
    pub fx: f64,
    pub fy: f64,
    pub palette: usize,
    pub n: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct CometVisual {
    pub y0: f64,
    pub amp: f64,
    /// Position écran de la tête lors de la dernière frame.
    pub head: Option<(f64, f64)>,
    pub trail_acc: f64,
}

pub struct Visuals {
    pub rng: FxRng,
    pub particles: ParticleSystem,
    pub floaters: Vec<Floater>,
    pub stars: StarField,
    pub counter: Tween<f64>,
    pub counter_ready: bool,
    pub counter_shown: f64,
    pub bump_at: f64,
    pub digits_prev: String,
    /// Instant du dernier changement de chaque chiffre (index depuis la droite).
    pub digit_changed: [f64; 40],
    pub gen_flash: [f64; 9],
    pub gen_fail: [f64; 9],
    pub gen_reveal: [f64; 9],
    pub bar_fill: [f64; 9],
    pub upg_flash: Vec<f64>,
    pub upg_fail: Vec<f64>,
    pub orbit_count: [usize; 9],
    pub orbit_pop: [f64; 9],
    pub orbit_ready: bool,
    pub comet: CometVisual,
    pub shooting: Vec<ShootingStar>,
    pub next_shooting: f64,
    pub bursts: Vec<PendingBurst>,
    pub sky_flash: f64,
    pub sky_flash_color: Rgb,
    pub cascade_start: f64,
    pub mode_flash: f64,
    pub nova: Option<NovaSequence>,
    pub sun_scale: f64,
}

impl Visuals {
    pub fn new(upgrade_count: usize, seed: u64) -> Self {
        Visuals {
            rng: FxRng::new(seed),
            particles: ParticleSystem::new(MAX_PARTICLES, seed ^ 0xABCD),
            floaters: Vec::with_capacity(MAX_FLOATERS),
            stars: StarField::new(320, 0x5747_4152),
            counter: Tween::idle(0.0),
            counter_ready: false,
            counter_shown: 0.0,
            bump_at: NEVER,
            digits_prev: String::new(),
            digit_changed: [NEVER; 40],
            gen_flash: [NEVER; 9],
            gen_fail: [NEVER; 9],
            gen_reveal: [NEVER; 9],
            bar_fill: [0.0; 9],
            upg_flash: vec![NEVER; upgrade_count],
            upg_fail: vec![NEVER; upgrade_count],
            orbit_count: [0; 9],
            orbit_pop: [NEVER; 9],
            orbit_ready: false,
            comet: CometVisual {
                y0: 3.0,
                amp: -1.5,
                head: None,
                trail_acc: 0.0,
            },
            shooting: Vec::with_capacity(4),
            next_shooting: 6.0,
            bursts: Vec::with_capacity(8),
            sky_flash: NEVER,
            sky_flash_color: theme::WHITE,
            cascade_start: NEVER,
            mode_flash: NEVER,
            nova: None,
            sun_scale: 1.0,
        }
    }

    pub fn set_reduced(&mut self, reduced: bool) {
        self.particles.set_reduced(reduced);
    }

    /// Valeur affichée par le gros compteur : roule vers `target`.
    pub fn counter_value(&mut self, now: f64, target: f64) -> f64 {
        if !self.counter_ready {
            self.counter = Tween::idle(target);
            self.counter_ready = true;
        }
        self.counter.retarget(target);
        self.counter_shown = self.counter.value(now).max(0.0);
        self.counter_shown
    }

    /// Lance un roulement du compteur (300 ms, ease_out_cubic) depuis la valeur affichée.
    pub fn roll_counter(&mut self, now: f64) {
        let from = self.counter_shown;
        self.counter = Tween::new(from, from, now, 0.3, Easing::OutCubic);
    }

    /// Roulement + éclat lumineux du compteur.
    pub fn bump_counter(&mut self, now: f64) {
        self.roll_counter(now);
        self.bump_at = now;
    }

    /// Mémorise quels chiffres ont changé depuis la frame précédente.
    pub fn track_digits(&mut self, digits: &str, now: f64) {
        if digits == self.digits_prev {
            return;
        }
        let mut old = self.digits_prev.chars().rev();
        for (i, c) in digits.chars().rev().enumerate() {
            let previous = old.next();
            if previous != Some(c) && i < self.digit_changed.len() {
                self.digit_changed[i] = now;
            }
        }
        self.digits_prev.clear();
        self.digits_prev.push_str(digits);
    }

    pub fn add_floater(&mut self, text: String, x: f64, y: f64, now: f64, color: Rgb) {
        if self.floaters.len() >= MAX_FLOATERS {
            self.floaters.remove(0);
        }
        self.floaters.push(Floater {
            text,
            x,
            y,
            born: now,
            color,
        });
    }

    /// Point aléatoire dans une zone (coordonnées écran).
    pub fn random_point(&mut self, area: Rect) -> (f64, f64) {
        let x = area.x as f64 + self.rng.range(0.15, 0.7) * area.width as f64;
        let y = area.y as f64 + self.rng.range(0.25, 0.85) * area.height as f64;
        (x, y)
    }

    /// Feu d'artifice : 2 à 3 gerbes colorées décalées dans le temps.
    pub fn fireworks(&mut self, now: f64) {
        let n = 2 + self.rng.below(2);
        for k in 0..n {
            let burst = PendingBurst {
                at: now + k as f64 * 0.25,
                fx: self.rng.range(0.1, 0.65),
                fy: self.rng.range(0.15, 0.6),
                palette: self.rng.below(theme::FIREWORKS.len()),
                n: 26,
            };
            self.bursts.push(burst);
        }
    }

    /// Explosion radiale (capture de comète, onde de choc…).
    pub fn explode(&mut self, x: f64, y: f64, n: usize, speed: f64, palette: &[Rgb]) {
        self.particles.burst(x, y, n, speed, palette);
    }

    /// Étincelles qui partent d'une ligne (achat).
    pub fn line_sparks(&mut self, x0: f64, x1: f64, y: f64, color: Rgb) {
        let palette = [color, color.glow(0.5), theme::WHITE];
        for _ in 0..10 {
            let x = self.rng.range(x0, x1);
            let vx = self.rng.range(-6.0, 6.0);
            let vy = self.rng.range(-7.0, -2.0);
            let c = palette[self.rng.below(palette.len())];
            let life = self.rng.range(0.4, 0.8);
            self.particles.spawn(Particle {
                x,
                y,
                vx,
                vy,
                life,
                gravity: 14.0,
                friction: 1.0,
                color: c,
                glyph: Glyph::Spark,
                ..Particle::default()
            });
        }
    }

    /// Avance les effets autonomes (particules, gerbes programmées, étoiles filantes).
    pub fn update(&mut self, dt: f64, now: f64, sky: Rect, reduced: bool) {
        self.particles.set_reduced(reduced);
        self.particles.update(dt);
        self.floaters.retain(|f| now - f.born < FLOATER_LIFE);

        // Gerbes programmées.
        let mut i = 0;
        while i < self.bursts.len() {
            if self.bursts[i].at <= now {
                let b = self.bursts.swap_remove(i);
                if !sky.is_empty() {
                    let x = sky.x as f64 + b.fx * sky.width as f64;
                    let y = sky.y as f64 + b.fy * sky.height as f64;
                    let palette = theme::FIREWORKS[b.palette % theme::FIREWORKS.len()];
                    self.particles.burst(x, y, b.n, 16.0, &palette);
                }
            } else {
                i += 1;
            }
        }

        // Étoiles filantes décoratives (toutes les 5 à 15 s).
        self.shooting.retain(|s| now - s.born < s.life);
        if now >= self.next_shooting && !sky.is_empty() {
            let w = sky.width as f64;
            let h = sky.height as f64;
            let star = ShootingStar {
                x: self.rng.range(0.35, 1.0) * w,
                y: self.rng.range(0.0, 0.35) * h,
                vx: -self.rng.range(26.0, 40.0),
                vy: self.rng.range(5.0, 9.0),
                born: now,
                life: self.rng.range(0.5, 0.8),
            };
            self.shooting.push(star);
            self.next_shooting = now + self.rng.range(5.0, 15.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_rolls_towards_target() {
        let mut v = Visuals::new(4, 1);
        assert_eq!(v.counter_value(0.0, 100.0), 100.0);
        v.roll_counter(1.0);
        let mid = v.counter_value(1.1, 200.0);
        assert!(mid > 100.0 && mid < 200.0, "{mid}");
        assert_eq!(v.counter_value(1.5, 200.0), 200.0);
    }

    #[test]
    fn digit_tracking() {
        let mut v = Visuals::new(0, 1);
        v.track_digits("123", 1.0);
        v.track_digits("124", 2.0);
        assert_eq!(v.digit_changed[0], 2.0);
        assert_eq!(v.digit_changed[1], 1.0);
    }

    #[test]
    fn fireworks_fire_later() {
        let mut v = Visuals::new(0, 1);
        v.fireworks(0.0);
        assert!(v.bursts.len() >= 2);
        let sky = Rect::new(0, 0, 40, 10);
        v.update(0.01, 0.0, sky, false);
        let first = v.particles.alive_count();
        assert!(first > 0);
        v.update(0.01, 1.0, sky, false);
        assert!(v.bursts.is_empty());
        assert!(v.particles.alive_count() > first);
    }
}
