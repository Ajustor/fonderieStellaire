//! Moteur d'effets maison : easing, tweens, timelines, particules, couleurs,
//! bruit et post-effets sur le `Buffer`. Aucune dépendance au jeu.

pub mod color;
pub mod easing;
pub mod effects;
pub mod noise;
pub mod palette;
pub mod particles;
pub mod rng;
pub mod timeline;
pub mod tween;

pub use color::Rgb;
pub use easing::Easing;
pub use particles::ParticleSystem;
pub use rng::FxRng;
pub use timeline::Timeline;
pub use tween::{Lerp, Tween};
