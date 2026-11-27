//! Synthèse : chaque son est une liste de notes (oscillateur, enveloppe,
//! glissement de hauteur), rendue une fois en échantillons mono.

/// Forme d'onde d'une note.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Wave {
    Sine,
    Triangle,
    /// Bruit blanc passé dans un filtre passe-bas (souffle).
    Noise,
}

/// Une note : quelques nombres suffisent à la décrire.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Note {
    pub wave: Wave,
    /// Début, en secondes depuis le début du son.
    pub start: f32,
    /// Hauteur au début et à la fin de la note (Hz) : glissement exponentiel.
    /// Pour le bruit : fréquence de coupure du filtre.
    pub freq: (f32, f32),
    /// Montée (s) : linéaire si courte, en courbe si longue (son qui gonfle).
    pub attack: f32,
    /// Extinction (s) après la montée : décroissance exponentielle.
    pub decay: f32,
    pub gain: f32,
}

impl Note {
    pub const fn new(wave: Wave, start: f32, freq: f32, decay: f32, gain: f32) -> Self {
        Note {
            wave,
            start,
            freq: (freq, freq),
            attack: 0.003,
            decay,
            gain,
        }
    }

    pub const fn glide(mut self, to: f32) -> Self {
        self.freq.1 = to;
        self
    }

    pub const fn attack(mut self, attack: f32) -> Self {
        self.attack = attack;
        self
    }

    fn end(&self) -> f32 {
        self.start + self.attack + self.decay
    }

    /// Enveloppe à `t` secondes du début de la note.
    fn envelope(&self, t: f32) -> f32 {
        if t < self.attack {
            let x = t / self.attack;
            // Une longue montée gonfle doucement puis s'emballe.
            if self.attack > 0.2 {
                x * x
            } else {
                x
            }
        } else {
            // −40 dB à la fin de l'extinction.
            (-4.6 * (t - self.attack) / self.decay).exp()
        }
    }
}

/// Petit générateur pseudo-aléatoire (variations et bruit), sans dépendance.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    /// Nombre dans [0, 1).
    pub fn unit(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Nombre dans [1 − amount, 1 + amount].
    pub fn around(&mut self, amount: f32) -> f32 {
        1.0 + (self.unit() * 2.0 - 1.0) * amount
    }
}

/// Rend les notes en échantillons mono à `rate` Hz.
pub fn render(notes: &[Note], rate: u32, rng: &mut Rng) -> Vec<f32> {
    let sr = rate as f32;
    let len = notes.iter().map(Note::end).fold(0.0, f32::max);
    let mut out = vec![0.0f32; (len * sr).ceil() as usize + 1];
    for n in notes {
        let first = (n.start * sr) as usize;
        let count = ((n.attack + n.decay) * sr) as usize;
        let ratio = n.freq.1 / n.freq.0;
        let mut phase = 0.0f32;
        let mut low = 0.0f32;
        for i in 0..count {
            let Some(slot) = out.get_mut(first + i) else {
                break;
            };
            let t = i as f32 / sr;
            let progress = i as f32 / count as f32;
            let f = n.freq.0 * ratio.powf(progress);
            let v = match n.wave {
                Wave::Sine => (phase * std::f32::consts::TAU).sin(),
                Wave::Triangle => 4.0 * (phase - (phase + 0.5).floor()).abs() - 1.0,
                Wave::Noise => {
                    let a = 1.0 - (-std::f32::consts::TAU * f / sr).exp();
                    low += a * (rng.unit() * 2.0 - 1.0 - low);
                    low * 2.0
                }
            };
            phase = (phase + f / sr).fract();
            *slot += v * n.envelope(t) * n.gain;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_lasts_attack_plus_decay_and_fades_out() {
        let n = Note::new(Wave::Sine, 0.1, 440.0, 0.2, 0.5);
        let pcm = render(&[n], 8000, &mut Rng::new(1));
        assert_eq!(pcm.len(), (0.303f32 * 8000.0).ceil() as usize + 1);
        assert!(
            pcm[..790].iter().all(|&s| s == 0.0),
            "silence avant le début"
        );
        let peak = pcm.iter().fold(0.0f32, |a, &s| a.max(s.abs()));
        assert!(peak > 0.4 && peak <= 0.5, "{peak}");
        let tail = pcm[pcm.len() - 20..]
            .iter()
            .fold(0.0f32, |a, &s| a.max(s.abs()));
        assert!(tail < 0.01, "s'éteint : {tail}");
    }

    #[test]
    fn noise_is_filtered_and_varies_with_the_seed() {
        let n = Note::new(Wave::Noise, 0.0, 800.0, 0.1, 0.5);
        let a = render(&[n], 8000, &mut Rng::new(1));
        let b = render(&[n], 8000, &mut Rng::new(2));
        assert_ne!(a, b);
        assert!(a.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
    }
}
