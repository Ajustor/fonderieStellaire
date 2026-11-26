//! Fin de l'Acte II : le Grand Attracteur s'éveille.
//!
//! 0 s les étoiles convergent en spirale vers le centre · 3,5 s éclair doré ·
//! 4 s une galaxie immense s'ouvre et tourne · le générique défile puis se
//! pose au centre · [Entrée] pour reprendre (le jeu continue).

use std::f64::consts::TAU;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::draw;
use crate::fx::easing::{ease_in_cubic, ease_out_cubic};
use crate::fx::effects;
use crate::fx::rng::hash3;
use crate::fx::Rgb;
use crate::theme;
use crate::widgets::portals::draw_vortex;

const CONVERGE: f64 = 3.5;
const FLASH: f64 = 0.5;
const CREDITS_AT: f64 = 4.0;
/// Lignes par seconde du défilement.
const SCROLL: f64 = 2.2;
/// On peut quitter le générique après ce délai.
pub const SKIPPABLE_AT: f64 = 6.0;
const ATTRACTOR_PALETTE: [Rgb; 3] = [Rgb(255, 210, 110), Rgb(190, 110, 255), Rgb(90, 150, 255)];

/// Une ligne du générique : texte, couleur, gras.
pub type CreditLine = (String, Rgb, bool);

pub struct EndingSequence {
    pub start: f64,
    lines: Vec<CreditLine>,
}

impl EndingSequence {
    pub fn new(now: f64, lines: Vec<CreditLine>) -> Self {
        EndingSequence { start: now, lines }
    }

    pub fn t(&self, now: f64) -> f64 {
        now - self.start
    }

    pub fn can_close(&self, now: f64) -> bool {
        self.t(now) >= SKIPPABLE_AT
    }

    pub fn render(&self, buf: &mut Buffer, area: Rect, now: f64) {
        let t = self.t(now);
        let cx = area.x as f64 + area.width as f64 / 2.0;
        let cy = area.y as f64 + area.height as f64 / 2.0;
        let max_r = (area.width as f64 / 2.0).hypot(area.height as f64);

        if t < CONVERGE {
            // Le jeu s'assombrit, des milliers de points convergent en spirale.
            effects::fade(buf, area, (t / 1.2).min(0.85));
            let p = ease_in_cubic(t / CONVERGE);
            for i in 0..260 {
                let a0 = hash3(i, 11, 3) * TAU;
                let r0 = (0.25 + 0.75 * hash3(i, 12, 3)) * max_r;
                let r = r0 * (1.0 - p);
                let a = a0 + p * (3.0 + 4.0 * hash3(i, 13, 3));
                let x = cx + r * a.cos();
                let y = cy + r * a.sin() / 2.0;
                let c = ATTRACTOR_PALETTE[i as usize % 3].lerp(theme::WHITE, p);
                let ch = if r < 3.0 { '✦' } else { '·' };
                draw::put(buf, area, x as i32, y as i32, ch, c);
            }
            draw::put_styled(
                buf,
                area,
                cx as i32,
                cy as i32,
                '✶',
                theme::bold(theme::GOLD.lerp(theme::WHITE, p)),
            );
            return;
        }

        // La galaxie du Grand Attracteur s'ouvre.
        draw::clear(buf, area);
        let open = ease_out_cubic(((t - CONVERGE) / 3.0).clamp(0.0, 1.0));
        let radius = 2.0 + open * area.width.min(area.height * 2) as f64 * 0.48;
        draw_vortex(
            buf,
            area,
            cx,
            cy,
            radius,
            now * 0.6,
            &ATTRACTOR_PALETTE,
            if t > CREDITS_AT { 0.45 } else { 1.0 },
        );
        // Éclair doré.
        if t < CONVERGE + FLASH {
            effects::flash(
                buf,
                area,
                theme::GOLD.lerp(theme::WHITE, 0.6),
                1.0 - (t - CONVERGE) / FLASH,
            );
        }
        if t < CREDITS_AT {
            return;
        }

        // Générique : il monte depuis le bas puis se pose au centre.
        let n = self.lines.len() as f64;
        let rest = (cy - n / 2.0).max(area.y as f64 + 1.0);
        let top = (area.bottom() as f64 - (t - CREDITS_AT) * SCROLL).max(rest);
        for (i, (text, color, bold)) in self.lines.iter().enumerate() {
            let y = top + i as f64;
            if y < area.y as f64 || y >= area.bottom() as f64 - 1.0 || text.is_empty() {
                continue;
            }
            let shown = draw::fit(text, area.width.saturating_sub(4) as usize);
            let w = shown.chars().count() as u16;
            let x = area.x + area.width.saturating_sub(w) / 2;
            // Halo : on efface la galaxie derrière le texte.
            draw::clear(buf, Rect::new(x.saturating_sub(1), y as u16, w + 2, 1));
            let style = if *bold {
                theme::bold(*color)
            } else {
                theme::fg(*color)
            };
            draw::text(buf, x, y as u16, &shown, style, area.right());
        }
        if self.can_close(now) {
            let hint = "[Entrée] continuer à jouer";
            let pulse = 0.5 + 0.5 * (now * 2.0).sin();
            let w = hint.chars().count() as u16;
            draw::text(
                buf,
                area.x + area.width.saturating_sub(w) / 2,
                area.bottom().saturating_sub(1),
                hint,
                theme::fg(theme::GREY.lerp(theme::TEXT, pulse)),
                area.right(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credits_scroll_then_rest_in_the_middle() {
        let lines: Vec<CreditLine> = vec![
            ("FIN DE L'ACTE II".into(), theme::GOLD, true),
            (String::new(), theme::TEXT, false),
            ("Merci d'avoir joué !".into(), theme::TEXT, false),
        ];
        let e = EndingSequence::new(0.0, lines);
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        e.render(&mut buf, area, 30.0);
        let text: String = (0..24)
            .map(|y| {
                (0..80)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
                    + "\n"
            })
            .collect();
        assert!(text.contains("FIN DE L'ACTE II"), "{text}");
        assert!(text.contains("Merci d'avoir joué !"));
        assert!(text.contains("[Entrée] continuer à jouer"));
        assert!(!e.can_close(5.0) && e.can_close(6.0));
    }
}
