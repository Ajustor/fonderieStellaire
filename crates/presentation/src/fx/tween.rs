//! Tweens basés sur le temps absolu (secondes depuis le lancement).

use ratatui::style::Color;

use super::color::Rgb;
use super::easing::Easing;

pub trait Lerp: Copy {
    fn lerp(a: Self, b: Self, t: f64) -> Self;
}

impl Lerp for f64 {
    fn lerp(a: f64, b: f64, t: f64) -> f64 {
        a + (b - a) * t
    }
}

impl Lerp for (f64, f64) {
    fn lerp(a: Self, b: Self, t: f64) -> Self {
        (f64::lerp(a.0, b.0, t), f64::lerp(a.1, b.1, t))
    }
}

impl Lerp for Rgb {
    fn lerp(a: Rgb, b: Rgb, t: f64) -> Rgb {
        // Les easings « back » dépassent 1 : on borne pour les couleurs.
        a.lerp(b, t)
    }
}

impl Lerp for Color {
    fn lerp(a: Color, b: Color, t: f64) -> Color {
        match (Rgb::from_color(a), Rgb::from_color(b)) {
            (Some(x), Some(y)) => x.lerp(y, t).color(),
            _ => {
                if t < 0.5 {
                    a
                } else {
                    b
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Tween<T: Lerp> {
    pub from: T,
    pub to: T,
    pub start: f64,
    pub duration: f64,
    pub easing: Easing,
}

impl<T: Lerp> Tween<T> {
    pub fn new(from: T, to: T, start: f64, duration: f64, easing: Easing) -> Self {
        Tween {
            from,
            to,
            start,
            duration,
            easing,
        }
    }

    /// Tween déjà terminé, figé sur `value`.
    pub fn idle(value: T) -> Self {
        Tween::new(value, value, f64::NEG_INFINITY, 0.0, Easing::Linear)
    }

    /// Avancement brut (non « easé ») dans [0, 1].
    pub fn progress(&self, now: f64) -> f64 {
        if self.duration <= 0.0 {
            return 1.0;
        }
        ((now - self.start) / self.duration).clamp(0.0, 1.0)
    }

    pub fn value(&self, now: f64) -> T {
        T::lerp(self.from, self.to, self.easing.apply(self.progress(now)))
    }

    pub fn is_done(&self, now: f64) -> bool {
        self.progress(now) >= 1.0
    }

    /// Change la cible sans redémarrer l'animation.
    pub fn retarget(&mut self, to: T) {
        self.to = to;
    }

    /// Redémarre depuis la valeur courante vers une nouvelle cible.
    pub fn restart(&mut self, now: f64, to: T) {
        self.from = self.value(now);
        self.to = to;
        self.start = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tween_f64() {
        let t = Tween::new(0.0, 10.0, 1.0, 2.0, Easing::Linear);
        assert_eq!(t.value(0.0), 0.0);
        assert_eq!(t.value(2.0), 5.0);
        assert_eq!(t.value(3.0), 10.0);
        assert!(t.is_done(3.5));
        assert!(!t.is_done(2.9));
    }

    #[test]
    fn tween_pair_and_color() {
        let t = Tween::new((0.0, 0.0), (4.0, 8.0), 0.0, 1.0, Easing::Linear);
        assert_eq!(t.value(0.5), (2.0, 4.0));
        let c = Tween::new(
            Color::Rgb(0, 0, 0),
            Color::Rgb(200, 100, 50),
            0.0,
            1.0,
            Easing::Linear,
        );
        assert_eq!(c.value(0.5), Color::Rgb(100, 50, 25));
    }

    #[test]
    fn restart_continues_from_current_value() {
        let mut t = Tween::new(0.0, 10.0, 0.0, 1.0, Easing::Linear);
        t.restart(0.5, 0.0);
        assert_eq!(t.from, 5.0);
        assert_eq!(t.value(0.5), 5.0);
        assert_eq!(t.value(1.5), 0.0);
    }

    #[test]
    fn idle_is_constant() {
        let t = Tween::idle(3.0);
        assert_eq!(t.value(-100.0), 3.0);
        assert_eq!(t.value(100.0), 3.0);
    }
}
