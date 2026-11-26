//! Système de particules à pool fixe (aucune allocation après la création).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::color::Rgb;
use super::rng::FxRng;

/// Glyphes successifs selon l'âge d'une étincelle.
const SPARK_GLYPHS: [char; 4] = ['✦', '+', '·', '.'];
/// Couleur vers laquelle les particules s'estompent (fond supposé sombre).
const FADE_TO: Rgb = Rgb(14, 14, 26);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Glyph {
    /// `✦ → + → · → .` selon l'âge.
    Spark,
    /// Toujours le même caractère.
    Fixed(char),
}

#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub age: f64,
    pub life: f64,
    pub gravity: f64,
    /// Frottement : fraction de vitesse perdue par seconde.
    pub friction: f64,
    pub color: Rgb,
    pub glyph: Glyph,
    pub alive: bool,
}

impl Default for Particle {
    fn default() -> Self {
        Particle {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            age: 0.0,
            life: 1.0,
            gravity: 0.0,
            friction: 0.0,
            color: Rgb(255, 255, 255),
            glyph: Glyph::Spark,
            alive: false,
        }
    }
}

impl Particle {
    pub fn ratio(&self) -> f64 {
        (self.age / self.life).clamp(0.0, 1.0)
    }

    pub fn current_glyph(&self) -> char {
        match self.glyph {
            Glyph::Fixed(c) => c,
            Glyph::Spark => {
                let i = (self.ratio() * SPARK_GLYPHS.len() as f64) as usize;
                SPARK_GLYPHS[i.min(SPARK_GLYPHS.len() - 1)]
            }
        }
    }

    pub fn current_color(&self) -> Rgb {
        self.color.lerp(FADE_TO, self.ratio().powf(1.3))
    }
}

pub struct ParticleSystem {
    pool: Vec<Particle>,
    next: usize,
    /// 1 = normal, 4 = effets réduits (une particule sur quatre).
    divisor: usize,
    skip: usize,
    rng: FxRng,
}

impl ParticleSystem {
    pub fn new(capacity: usize, seed: u64) -> Self {
        ParticleSystem {
            pool: vec![Particle::default(); capacity.max(1)],
            next: 0,
            divisor: 1,
            skip: 0,
            rng: FxRng::new(seed),
        }
    }

    pub fn capacity(&self) -> usize {
        self.pool.len()
    }

    pub fn set_reduced(&mut self, reduced: bool) {
        self.divisor = if reduced { 4 } else { 1 };
    }

    pub fn alive_count(&self) -> usize {
        self.pool.iter().filter(|p| p.alive).count()
    }

    pub fn clear(&mut self) {
        self.pool.iter_mut().for_each(|p| p.alive = false);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Particle> {
        self.pool.iter().filter(|p| p.alive)
    }

    /// Ajoute une particule (écrase la plus ancienne si le pool est plein).
    /// En mode réduit, seule une particule sur `divisor` est réellement émise.
    pub fn spawn(&mut self, mut p: Particle) {
        self.skip = (self.skip + 1) % self.divisor;
        if self.skip != 0 {
            return;
        }
        p.alive = true;
        p.age = 0.0;
        let n = self.pool.len();
        // Cherche un emplacement libre à partir du curseur, sinon recycle.
        let mut idx = self.next;
        for k in 0..n {
            let i = (self.next + k) % n;
            if !self.pool[i].alive {
                idx = i;
                break;
            }
        }
        self.pool[idx] = p;
        self.next = (idx + 1) % n;
    }

    /// Gerbe radiale.
    pub fn burst(&mut self, x: f64, y: f64, n: usize, speed: f64, palette: &[Rgb]) {
        for _ in 0..n {
            let a = self.rng.range(0.0, std::f64::consts::TAU);
            let v = speed * self.rng.range(0.35, 1.0);
            let color = palette[self
                .rng
                .below(palette.len().max(1))
                .min(palette.len().saturating_sub(1))];
            let life = self.rng.range(0.5, 1.1);
            self.spawn(Particle {
                x,
                y,
                vx: a.cos() * v,
                vy: a.sin() * v * 0.5,
                life,
                gravity: 4.0,
                friction: 1.8,
                color,
                glyph: Glyph::Spark,
                ..Particle::default()
            });
        }
    }

    /// Fontaine : jet vers le haut avec gravité.
    pub fn fountain(&mut self, x: f64, y: f64, n: usize, speed: f64, palette: &[Rgb]) {
        for _ in 0..n {
            let vx = self.rng.range(-0.6, 0.6) * speed;
            let vy = -self.rng.range(0.6, 1.0) * speed * 0.6;
            let color = palette[self
                .rng
                .below(palette.len())
                .min(palette.len().saturating_sub(1))];
            let life = self.rng.range(0.6, 1.2);
            self.spawn(Particle {
                x,
                y,
                vx,
                vy,
                life,
                gravity: 9.0,
                friction: 0.5,
                color,
                glyph: Glyph::Spark,
                ..Particle::default()
            });
        }
    }

    /// Particule de traînée, quasi immobile, qui s'éteint vite.
    pub fn trail(&mut self, x: f64, y: f64, vx: f64, vy: f64, color: Rgb) {
        let jitter = self.rng.range(-0.3, 0.3);
        let life = self.rng.range(0.4, 0.9);
        self.spawn(Particle {
            x,
            y: y + jitter * 0.5,
            vx: vx * 0.1,
            vy: vy * 0.1 + jitter,
            life,
            gravity: 0.5,
            friction: 2.0,
            color,
            glyph: Glyph::Spark,
            ..Particle::default()
        });
    }

    pub fn rng(&mut self) -> &mut FxRng {
        &mut self.rng
    }

    pub fn update(&mut self, dt: f64) {
        if dt.is_nan() || dt <= 0.0 {
            return;
        }
        for p in self.pool.iter_mut().filter(|p| p.alive) {
            p.age += dt;
            if p.age >= p.life {
                p.alive = false;
                continue;
            }
            let keep = (1.0 - p.friction * dt).max(0.0);
            p.vx *= keep;
            p.vy = p.vy * keep + p.gravity * dt;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
        }
    }

    /// Dessine les particules dans le `Buffer`, limitées à `clip`.
    pub fn draw(&self, buf: &mut Buffer, clip: Rect) {
        for p in self.iter() {
            if p.x < clip.x as f64
                || p.y < clip.y as f64
                || p.x >= clip.right() as f64
                || p.y >= clip.bottom() as f64
            {
                continue;
            }
            let (x, y) = (p.x as u16, p.y as u16);
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(p.current_glyph());
                cell.set_fg(p.current_color().color());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_never_grows() {
        let mut ps = ParticleSystem::new(10, 1);
        ps.burst(5.0, 5.0, 50, 10.0, &[Rgb(255, 0, 0)]);
        assert_eq!(ps.capacity(), 10);
        assert_eq!(ps.alive_count(), 10);
    }

    #[test]
    fn particles_age_and_die() {
        let mut ps = ParticleSystem::new(16, 1);
        ps.spawn(Particle {
            life: 1.0,
            vx: 2.0,
            ..Particle::default()
        });
        ps.update(0.5);
        let p = ps.iter().next().copied().unwrap();
        assert!(p.x > 0.0);
        assert_eq!(p.current_glyph(), '·');
        ps.update(0.6);
        assert_eq!(ps.alive_count(), 0);
    }

    #[test]
    fn glyph_and_color_evolve() {
        let mut p = Particle {
            life: 1.0,
            color: Rgb(255, 255, 255),
            ..Particle::default()
        };
        assert_eq!(p.current_glyph(), '✦');
        p.age = 0.3;
        assert_eq!(p.current_glyph(), '+');
        p.age = 0.99;
        assert_eq!(p.current_glyph(), '.');
        assert!(p.current_color().luma() < 0.2);
    }

    #[test]
    fn gravity_pulls_down() {
        let mut ps = ParticleSystem::new(4, 1);
        ps.spawn(Particle {
            life: 5.0,
            gravity: 10.0,
            ..Particle::default()
        });
        for _ in 0..10 {
            ps.update(0.1);
        }
        assert!(ps.iter().next().unwrap().y > 1.0);
    }

    #[test]
    fn reduced_mode_divides_by_four() {
        let mut ps = ParticleSystem::new(400, 1);
        ps.set_reduced(true);
        ps.burst(0.0, 0.0, 40, 5.0, &[Rgb(1, 2, 3)]);
        assert_eq!(ps.alive_count(), 10);
    }
}
