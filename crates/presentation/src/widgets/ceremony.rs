//! Cérémonie de la Clé d'univers : deux anneaux qui tournent en sens inverse
//! autour d'une clé dessinée en blocs, un reflet qui la traverse, le nom écrit
//! en machine à écrire, puis tout s'efface.

use std::f64::consts::TAU;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use domain::UniverseDef;

use crate::draw;
use crate::fx::color::gradient;
use crate::fx::easing::{ease_in_cubic, ease_out_back, ease_out_cubic};
use crate::fx::effects::{self, typewriter};
use crate::fx::Rgb;
use crate::theme;

/// Durée totale de la cérémonie.
pub const DURATION: f64 = 4.6;

/// La clé, en blocs (11 colonnes × 3 lignes).
const KEY_ART: [&str; 3] = [" ▄▀▀▄      ", "█    █▀▀█▀█", " ▀▄▄▀      "];

/// Opacité globale : apparition, maintien, disparition.
fn presence(t: f64) -> f64 {
    if t < 0.35 {
        ease_out_cubic(t / 0.35)
    } else if t > DURATION - 0.6 {
        1.0 - ease_in_cubic((t - (DURATION - 0.6)) / 0.6)
    } else {
        1.0
    }
}

pub fn render(buf: &mut Buffer, area: Rect, now: f64, start: f64, def: &UniverseDef) {
    let t = now - start;
    if !(0.0..DURATION).contains(&t) {
        return;
    }
    let a = presence(t);
    let palette = theme::universe_palette(def.id.index());
    effects::fade(buf, area, 0.7 * a);

    let cx = area.x as f64 + area.width as f64 / 2.0;
    let cy = area.y as f64 + area.height as f64 / 2.0 - 2.0;
    let grow = ease_out_back((t / 0.8).clamp(0.0, 1.0));
    // À la fin, les anneaux s'écartent en s'effaçant.
    let spread = 1.0 + 0.6 * ease_in_cubic(((t - (DURATION - 0.6)) / 0.6).clamp(0.0, 1.0));

    // Anneau extérieur : symboles de l'univers.
    let outer_r = 13.0 * grow * spread;
    let count = 16;
    for k in 0..count {
        let ang = now * 0.9 + k as f64 * TAU / count as f64;
        let x = cx + outer_r * ang.cos();
        let y = cy + outer_r * ang.sin() * 0.5;
        let c = gradient(&palette, k as f64 / count as f64)
            .glow(0.2)
            .dim(1.0 - a);
        let ch = if k % 2 == 0 { def.symbol } else { '·' };
        draw::put_styled(
            buf,
            area,
            x.floor() as i32,
            y.floor() as i32,
            ch,
            theme::bold(c),
        );
    }
    // Anneau intérieur, en sens inverse.
    let inner_r = 8.0 * grow * spread;
    for k in 0..24 {
        let ang = -now * 1.4 + k as f64 * TAU / 24.0;
        let x = cx + inner_r * ang.cos();
        let y = cy + inner_r * ang.sin() * 0.5;
        let c = palette[1].glow(0.3).dim(1.0 - a * 0.9);
        draw::put(buf, area, x.floor() as i32, y.floor() as i32, '◆', c);
    }

    // La clé, avec un reflet qui la traverse en boucle.
    let kw = KEY_ART[0].chars().count() as f64;
    let kx = (cx - kw / 2.0).floor() as i32;
    let ky = (cy - 1.0).floor() as i32;
    let sweep = ((t * 0.9).fract() * (kw + 8.0)) - 4.0;
    for (r, line) in KEY_ART.iter().enumerate() {
        for (i, ch) in line.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let d = (i as f64 + r as f64 - sweep).abs();
            let shine = if d < 2.5 { 0.7 * (1.0 - d / 2.5) } else { 0.0 };
            let c = theme::GOLD.lerp(palette[0], 0.4).glow(shine).dim(1.0 - a);
            draw::put_styled(buf, area, kx + i as i32, ky + r as i32, ch, theme::bold(c));
        }
    }

    // Textes en machine à écrire.
    let lines: [(String, Rgb, f64); 3] = [
        (
            format!("✶  {}  ✶", def.key_name.to_uppercase()),
            palette[0],
            0.5,
        ),
        (def.reward_text.to_string(), theme::CYAN, 1.3),
        (format!("Univers : {}", def.name), theme::MUTED, 2.0),
    ];
    let base_y = (cy + outer_r * 0.5 / spread + 2.0).round() as u16;
    for (k, (text, color, delay)) in lines.iter().enumerate() {
        let shown = typewriter(text, t - delay, 40.0);
        if shown.is_empty() {
            continue;
        }
        let w = text.chars().count() as u16;
        let x = area.x + area.width.saturating_sub(w) / 2;
        let y = base_y + k as u16 * 2;
        if y >= area.bottom() {
            continue;
        }
        draw::text(
            buf,
            x,
            y,
            shown,
            theme::bold(color.dim(1.0 - a)),
            area.right(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_fades_in_and_out() {
        assert_eq!(presence(0.0), 0.0);
        assert_eq!(presence(2.0), 1.0);
        assert!(presence(DURATION - 0.01) < 0.05);
    }

    #[test]
    fn ceremony_renders_key_and_name() {
        let catalog = domain::Catalog::standard();
        let def = &catalog.universes[1];
        let area = Rect::new(0, 0, 100, 30);
        let mut buf = Buffer::empty(area);
        render(&mut buf, area, 3.0, 0.0, def);
        let text: String = (0..30)
            .map(|y| {
                (0..100)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
                    + "\n"
            })
            .collect();
        assert!(text.contains("CLÉ DE GRAVITÉ"), "{text}");
        assert!(text.contains("█    █▀▀█▀█"));
    }
}
