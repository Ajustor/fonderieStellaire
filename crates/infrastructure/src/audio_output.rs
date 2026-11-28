//! Sortie audio de l'ordinateur (cpal) : le flux appelle une fonction de rendu
//! mono depuis son propre fil, et ses échantillons sont recopiés sur chaque
//! canal de la carte son.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};

/// Rendu mono : remplit le tampon avec les prochains échantillons (−1 à 1).
pub type Render = Box<dyn FnMut(&mut [f32]) + Send + 'static>;

/// Flux audio ouvert : les sons s'arrêtent quand il est détruit.
pub struct AudioOutput {
    stream: cpal::Stream,
    rate: u32,
}

impl AudioOutput {
    /// Ouvre la sortie par défaut. `make` reçoit sa fréquence
    /// d'échantillonnage et renvoie le rendu. Erreur si aucune carte son
    /// (serveur, conteneur) : le jeu reste alors muet.
    pub fn start(make: impl FnOnce(u32) -> Render) -> Result<Self, String> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or("aucune sortie audio")?;
        let supported = device
            .default_output_config()
            .map_err(|e| format!("sortie audio : {e}"))?;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();
        let rate = config.sample_rate.0;
        let render = make(rate);
        let stream = match format {
            SampleFormat::F32 => build::<f32>(&device, &config, render),
            SampleFormat::I16 => build::<i16>(&device, &config, render),
            SampleFormat::U16 => build::<u16>(&device, &config, render),
            other => return Err(format!("format audio non géré : {other}")),
        }?;
        stream.play().map_err(|e| format!("sortie audio : {e}"))?;
        Ok(AudioOutput {
            stream,
            rate,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.rate
    }

    /// Met le flux en marche ou en pause. En pause, la carte son peut se mettre
    /// en veille ; une erreur (sortie disparue) est ignorée.
    pub fn set_playing(&self, playing: bool) {
        if playing {
            let _ = self.stream.play();
        } else {
            let _ = self.stream.pause();
        }
    }
}

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut render: Render,
) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = usize::from(config.channels.max(1));
    let mut mono: Vec<f32> = Vec::new();
    device
        .build_output_stream(
            config,
            move |out: &mut [T], _| {
                let frames = out.len() / channels;
                mono.resize(frames, 0.0);
                render(&mut mono);
                for (frame, &s) in out.chunks_mut(channels).zip(&mono) {
                    let v = T::from_sample(s);
                    frame.fill(v);
                }
            },
            // Une erreur du flux (carte débranchée) coupe juste le son.
            |_| {},
            None,
        )
        .map_err(|e| format!("sortie audio : {e}"))
}
