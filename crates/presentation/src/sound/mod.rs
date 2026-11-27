//! Sons du jeu, générés par le code (aucun fichier audio) : chaque son est une
//! recette de quelques notes, synthétisée au lancement en plusieurs variantes
//! légèrement différentes, puis mélangée par le [`Mixer`].
//!
//! La présentation choisit le son ; la sortie (cpal sur ordinateur, AudioTrack
//! sur Android) tire les échantillons du mixeur depuis son propre fil.

pub mod synth;

use std::sync::{Arc, Mutex};

use synth::{Note, Rng, Wave};

use crate::screens::supernova::{COMMIT_AT, WAVE_AT};

/// Les sons du jeu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sfx {
    /// Récolte : « tic » cristallin.
    Harvest,
    /// Récolte continue : le même, plus aigu.
    HarvestHeld,
    /// Achat à l'unité : « pop ».
    Buy,
    /// Achat par lot : mini-arpège.
    BuyBatch,
    /// Palier de générateur : accord majeur égrené.
    Milestone,
    /// Succès : accord majeur égrené, plus haut et plus long.
    Achievement,
    /// Comète attrapée : glissando montant.
    Comet,
    /// Supernova : basse qui gonfle, souffle, puis accord lumineux, calés sur
    /// l'animation.
    Supernova,
    /// Défi réussi : petite fanfare.
    Challenge,
}

impl Sfx {
    pub const ALL: [Sfx; 9] = [
        Sfx::Harvest,
        Sfx::HarvestHeld,
        Sfx::Buy,
        Sfx::BuyBatch,
        Sfx::Milestone,
        Sfx::Achievement,
        Sfx::Comet,
        Sfx::Supernova,
        Sfx::Challenge,
    ];

    fn index(self) -> usize {
        self as usize
    }

    /// Écart minimal entre deux déclenchements (s) : la récolte continue ou
    /// une rafale d'achats ne saturent pas.
    pub fn min_interval(self) -> f64 {
        match self {
            Sfx::Harvest | Sfx::HarvestHeld => 0.07,
            Sfx::Buy | Sfx::BuyBatch => 0.06,
            Sfx::Milestone => 0.15,
            Sfx::Achievement => 0.3,
            Sfx::Comet => 0.3,
            Sfx::Supernova | Sfx::Challenge => 1.0,
        }
    }

    /// Variantes précalculées (une seule pour les longs sons).
    fn variants(self) -> usize {
        match self {
            Sfx::Supernova | Sfx::Challenge => 1,
            _ => 4,
        }
    }

    /// La recette, avec ses petites variations (hauteur, rythme, intensité).
    fn recipe(self, rng: &mut Rng) -> Vec<Note> {
        let p = rng.around(0.03);
        match self {
            Sfx::Harvest | Sfx::HarvestHeld => {
                let f = if self == Sfx::HarvestHeld {
                    3100.0
                } else {
                    2300.0
                } * p;
                vec![
                    Note::new(Wave::Sine, 0.0, f, 0.05, 0.32).glide(f * 0.94),
                    Note::new(Wave::Sine, 0.0, f * 2.01, 0.025, 0.1),
                ]
            }
            Sfx::Buy => vec![
                Note::new(Wave::Sine, 0.0, 620.0 * p, 0.07, 0.5).glide(300.0 * p),
                Note::new(Wave::Triangle, 0.0, 1240.0 * p, 0.035, 0.12).glide(600.0 * p),
            ],
            Sfx::BuyBatch => [523.25, 659.25, 783.99]
                .iter()
                .enumerate()
                .flat_map(|(i, &f)| {
                    let at = i as f32 * 0.045 * rng.around(0.1);
                    [
                        Note::new(Wave::Sine, at, f * p, 0.09, 0.32).glide(f * p * 0.9),
                        Note::new(Wave::Triangle, at, f * 2.0 * p, 0.05, 0.08),
                    ]
                })
                .collect(),
            Sfx::Milestone => arpeggio(&[523.25, 659.25, 783.99, 1046.5], 0.07, 0.6, 0.22, p, rng),
            Sfx::Achievement => {
                arpeggio(&[783.99, 987.77, 1174.66, 1567.98], 0.08, 0.9, 0.2, p, rng)
            }
            Sfx::Challenge => {
                let mut notes = arpeggio(
                    &[523.25, 659.25, 783.99, 1046.5, 1318.5],
                    0.09,
                    0.5,
                    0.2,
                    p,
                    rng,
                );
                // Accord final tenu.
                for f in [523.25, 659.25, 783.99, 1046.5] {
                    notes.push(Note::new(Wave::Triangle, 0.5, f * p, 1.4, 0.1).attack(0.02));
                }
                notes
            }
            Sfx::Comet => vec![
                Note::new(Wave::Triangle, 0.0, 500.0 * p, 0.45, 0.3)
                    .glide(2000.0 * p)
                    .attack(0.03),
                Note::new(Wave::Sine, 0.0, 1000.0 * p, 0.45, 0.12)
                    .glide(4000.0 * p)
                    .attack(0.03),
                Note::new(Wave::Noise, 0.25, 9000.0, 0.25, 0.05),
            ],
            Sfx::Supernova => {
                let wave = WAVE_AT as f32;
                let chord = COMMIT_AT as f32;
                let mut notes = vec![
                    // La basse gonfle jusqu'à l'onde de choc (ses harmoniques
                    // la rendent audible sur un haut-parleur de téléphone).
                    Note::new(Wave::Sine, 0.0, 55.0, 1.6, 0.6)
                        .glide(70.0)
                        .attack(wave),
                    Note::new(Wave::Triangle, 0.0, 110.0, 1.4, 0.22)
                        .glide(140.0)
                        .attack(wave),
                    // Onde de choc : coup sourd et souffle qui s'assombrit.
                    Note::new(Wave::Sine, wave, 140.0, 0.7, 0.6).glide(40.0),
                    Note::new(Wave::Noise, wave - 0.05, 7000.0, 1.4, 0.5)
                        .glide(250.0)
                        .attack(0.06),
                ];
                // Accord lumineux quand la Supernova se produit.
                for (i, f) in [523.25, 659.25, 783.99, 1046.5, 1318.5]
                    .into_iter()
                    .enumerate()
                {
                    let at = chord + i as f32 * 0.07;
                    notes.push(Note::new(Wave::Triangle, at, f, 2.2, 0.13).attack(0.01));
                    notes.push(Note::new(Wave::Sine, at, f * 2.0, 1.2, 0.05).attack(0.01));
                }
                notes
            }
        }
    }
}

/// Accord égrené : chaque note un peu après la précédente.
fn arpeggio(freqs: &[f32], step: f32, decay: f32, gain: f32, p: f32, rng: &mut Rng) -> Vec<Note> {
    freqs
        .iter()
        .enumerate()
        .flat_map(|(i, &f)| {
            let at = i as f32 * step * rng.around(0.08);
            let g = gain * rng.around(0.1);
            [
                Note::new(Wave::Sine, at, f * p, decay, g),
                Note::new(Wave::Triangle, at, f * 2.0 * p, decay * 0.4, g * 0.25),
            ]
        })
        .collect()
}

/// Voix en cours de lecture.
struct Voice {
    clip: Arc<[f32]>,
    pos: usize,
}

/// Nombre maximal de sons simultanés (le plus ancien laisse sa place).
const MAX_VOICES: usize = 12;

/// Mélange les sons en cours. Partagé entre le jeu (qui déclenche) et le fil
/// audio (qui tire les échantillons).
pub struct Mixer {
    rate: u32,
    bank: Vec<Vec<Arc<[f32]>>>,
    voices: Vec<Voice>,
    gain: f32,
    muted: bool,
    rng: Rng,
}

pub type SharedMixer = Arc<Mutex<Mixer>>;

impl Mixer {
    /// Synthétise tous les sons à `rate` Hz.
    pub fn new(rate: u32) -> Self {
        let rate = rate.clamp(8000, 192_000);
        let mut rng = Rng::new(0x5EED);
        let bank = Sfx::ALL
            .iter()
            .map(|&sfx| {
                (0..sfx.variants())
                    .map(|_| {
                        let notes = sfx.recipe(&mut rng);
                        Arc::from(synth::render(&notes, rate, &mut rng))
                    })
                    .collect()
            })
            .collect();
        Mixer {
            rate,
            bank,
            voices: Vec::with_capacity(MAX_VOICES),
            gain: volume_gain(60),
            muted: false,
            rng,
        }
    }

    pub fn shared(rate: u32) -> SharedMixer {
        Arc::new(Mutex::new(Mixer::new(rate)))
    }

    pub fn rate(&self) -> u32 {
        self.rate
    }

    /// Volume en % (courbe perceptive).
    pub fn set_volume(&mut self, percent: u8) {
        self.gain = volume_gain(percent);
    }

    /// Coupe la sortie (mode silencieux, arrière-plan) sans oublier les sons.
    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        if muted {
            self.voices.clear();
        }
    }

    pub fn play(&mut self, sfx: Sfx) {
        if self.muted {
            return;
        }
        let variants = &self.bank[sfx.index()];
        let pick = (self.rng.unit() * variants.len() as f32) as usize;
        let clip = variants[pick.min(variants.len() - 1)].clone();
        if self.voices.len() >= MAX_VOICES {
            self.voices.remove(0);
        }
        self.voices.push(Voice { clip, pos: 0 });
    }

    /// Sons en cours.
    pub fn active(&self) -> usize {
        self.voices.len()
    }

    /// Remplit `out` (mono) avec le mélange des sons en cours.
    pub fn render(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        if self.muted {
            return;
        }
        for v in &mut self.voices {
            let rest = &v.clip[v.pos..];
            let n = rest.len().min(out.len());
            for (o, s) in out.iter_mut().zip(&rest[..n]) {
                *o += s;
            }
            v.pos += n;
        }
        self.voices.retain(|v| v.pos < v.clip.len());
        for o in out.iter_mut() {
            *o = soft_clip(*o * self.gain);
        }
    }
}

fn volume_gain(percent: u8) -> f32 {
    let v = f32::from(percent.min(100)) / 100.0;
    v * v
}

/// Écrêtage doux : linéaire jusqu'à 0,8, puis arrondi vers 1.
fn soft_clip(x: f32) -> f32 {
    let a = x.abs();
    if a <= 0.8 {
        x
    } else {
        x.signum() * (0.8 + 0.2 * ((a - 0.8) / 0.2).tanh())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drain(m: &mut Mixer) -> Vec<f32> {
        let mut all = Vec::new();
        let mut buf = [0.0f32; 256];
        while m.active() > 0 {
            m.render(&mut buf);
            all.extend_from_slice(&buf);
        }
        all
    }

    #[test]
    fn every_sound_is_short_audible_and_bounded() {
        let mut m = Mixer::new(22_050);
        m.set_volume(100);
        for sfx in Sfx::ALL {
            m.play(sfx);
            let pcm = drain(&mut m);
            let secs = pcm.len() as f32 / 22_050.0;
            let peak = pcm.iter().fold(0.0f32, |a, &s| a.max(s.abs()));
            assert!(peak > 0.1 && peak <= 1.0, "{sfx:?} : crête {peak}");
            let longest = if sfx == Sfx::Supernova { 5.5 } else { 2.0 };
            assert!(secs <= longest, "{sfx:?} : {secs} s");
        }
    }

    #[test]
    fn supernova_swells_then_bursts_at_the_shock_wave() {
        let mut m = Mixer::new(22_050);
        m.set_volume(100);
        m.play(Sfx::Supernova);
        let pcm = drain(&mut m);
        let rms = |from: f64, to: f64| {
            let s = &pcm[(from * 22_050.0) as usize..(to * 22_050.0) as usize];
            (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt()
        };
        assert!(rms(0.0, 0.3) < rms(1.0, WAVE_AT), "la basse gonfle");
        assert!(rms(WAVE_AT, WAVE_AT + 0.3) > rms(0.0, 0.3), "onde de choc");
        assert!(rms(COMMIT_AT, COMMIT_AT + 0.5) > 0.01, "accord");
    }

    #[test]
    fn volume_mute_and_voice_limit() {
        let mut m = Mixer::new(8000);
        m.set_volume(0);
        m.play(Sfx::Buy);
        assert!(drain(&mut m).iter().all(|&s| s == 0.0), "volume nul");
        m.set_volume(60);
        m.set_muted(true);
        m.play(Sfx::Buy);
        assert_eq!(m.active(), 0, "coupé : rien ne joue");
        m.set_muted(false);
        for _ in 0..30 {
            m.play(Sfx::Harvest);
        }
        assert_eq!(m.active(), MAX_VOICES);
    }

    #[test]
    fn variants_differ_slightly() {
        let m = Mixer::new(8000);
        let v = &m.bank[Sfx::Harvest.index()];
        assert_eq!(v.len(), 4);
        assert_ne!(v[0], v[1]);
    }
}
