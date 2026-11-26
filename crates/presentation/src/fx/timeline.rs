//! Timelines : enchaîner (`then`), paralléliser (`with`) et espacer (`delay`)
//! des étapes nommées, puis interroger leur avancement à un instant donné.

use super::easing::Easing;

#[derive(Clone, Debug)]
struct Step {
    id: &'static str,
    start: f64,
    duration: f64,
    easing: Easing,
}

#[derive(Clone, Debug, Default)]
pub struct Timeline {
    steps: Vec<Step>,
    cursor: f64,
    last_start: f64,
}

impl Timeline {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ajoute une étape qui commence après la précédente.
    pub fn then(mut self, id: &'static str, duration: f64, easing: Easing) -> Self {
        self.last_start = self.cursor;
        self.steps.push(Step {
            id,
            start: self.cursor,
            duration,
            easing,
        });
        self.cursor += duration;
        self
    }

    /// Ajoute une étape qui commence en même temps que la précédente.
    pub fn with(mut self, id: &'static str, duration: f64, easing: Easing) -> Self {
        let start = self.last_start;
        self.steps.push(Step {
            id,
            start,
            duration,
            easing,
        });
        self.cursor = self.cursor.max(start + duration);
        self
    }

    /// Décale la prochaine étape.
    pub fn delay(mut self, d: f64) -> Self {
        self.cursor += d;
        self
    }

    /// Place une étape à un instant absolu (relatif au début de la timeline).
    pub fn at(mut self, id: &'static str, start: f64, duration: f64, easing: Easing) -> Self {
        self.last_start = start;
        self.steps.push(Step {
            id,
            start,
            duration,
            easing,
        });
        self.cursor = self.cursor.max(start + duration);
        self
    }

    pub fn total(&self) -> f64 {
        self.steps
            .iter()
            .map(|s| s.start + s.duration)
            .fold(0.0, f64::max)
    }

    pub fn start_of(&self, id: &str) -> Option<f64> {
        self.steps.iter().find(|s| s.id == id).map(|s| s.start)
    }

    /// Avancement « easé » de l'étape `id` au temps `t` : 0 avant, 1 après.
    pub fn progress(&self, id: &str, t: f64) -> f64 {
        match self.steps.iter().find(|s| s.id == id) {
            Some(s) if s.duration <= 0.0 => {
                if t >= s.start {
                    1.0
                } else {
                    0.0
                }
            }
            Some(s) => s.easing.apply(((t - s.start) / s.duration).clamp(0.0, 1.0)),
            None => 0.0,
        }
    }

    /// Vrai si l'étape est en cours au temps `t`.
    pub fn is_active(&self, id: &str, t: f64) -> bool {
        self.steps
            .iter()
            .any(|s| s.id == id && t >= s.start && t < s.start + s.duration)
    }

    pub fn is_done(&self, t: f64) -> bool {
        t >= self.total()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn then_with_delay() {
        let tl = Timeline::new()
            .then("a", 1.0, Easing::Linear)
            .with("b", 2.0, Easing::Linear)
            .delay(0.5)
            .then("c", 1.0, Easing::Linear);
        assert_eq!(tl.start_of("a"), Some(0.0));
        assert_eq!(tl.start_of("b"), Some(0.0));
        assert_eq!(tl.start_of("c"), Some(2.5));
        assert_eq!(tl.total(), 3.5);
        assert_eq!(tl.progress("a", 0.5), 0.5);
        assert_eq!(tl.progress("b", 0.5), 0.25);
        assert_eq!(tl.progress("c", 0.0), 0.0);
        assert_eq!(tl.progress("c", 10.0), 1.0);
        assert!(tl.is_active("b", 1.5));
        assert!(!tl.is_active("a", 1.5));
        assert!(tl.is_done(3.5));
    }

    #[test]
    fn absolute_steps() {
        let tl = Timeline::new().at("flash", 1.2, 0.2, Easing::Linear).at(
            "wave",
            1.4,
            1.1,
            Easing::OutCubic,
        );
        assert!((tl.progress("flash", 1.3) - 0.5).abs() < 1e-9);
        assert!((tl.total() - 2.5).abs() < 1e-12);
        assert_eq!(tl.progress("inconnu", 5.0), 0.0);
    }
}
