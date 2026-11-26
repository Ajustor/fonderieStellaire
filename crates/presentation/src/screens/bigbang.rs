//! Séquence plein écran du Big Bang (≈ 8,5 s).
//!
//! 0,0 s tout l'écran est aspiré en spirale vers un point · 1,6 s silence :
//! il ne reste qu'un point qui palpite · 2,4 s éclair blanc, validation ·
//! ondes de choc aux couleurs du spectre, débris lancés dans toutes les
//! directions, une nébuleuse et des étoiles naissent · 3,8 s « BIG BANG » ·
//! 4,8 s particules gagnées · 8,5 s retour au jeu.

use std::f64::consts::TAU;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::bigfont;
use crate::draw;
use crate::fmt;
use crate::fx::color::{gradient, gradient_cyclic};
use crate::fx::easing::{ease_in_cubic, ease_out_cubic};
use crate::fx::effects;
use crate::fx::rng::hash3;
use crate::fx::Rgb;
use crate::theme;
use crate::visuals::StarField;
use crate::widgets::sky::{draw_nebula, draw_stars};

/// Fin de l'aspiration : l'écran de jeu a disparu.
pub const FULL_AT: f64 = 1.6;
/// L'éclair : le Big Bang est validé à cet instant.
pub const COMMIT_AT: f64 = 2.4;
const TEXT_AT: f64 = COMMIT_AT + 1.4;
const COUNT_AT: f64 = COMMIT_AT + 2.4;
pub const DURATION: f64 = 8.5;

/// Du plus chaud au plus froid : la lumière du premier instant qui refroidit.
pub const SPECTRUM: [Rgb; 8] = [
    Rgb(255, 255, 255),
    Rgb(255, 244, 190),
    Rgb(255, 200, 80),
    Rgb(255, 150, 60),
    Rgb(255, 110, 170),
    Rgb(170, 90, 255),
    Rgb(80, 140, 255),
    Rgb(110, 230, 255),
];
const GLITCH: [char; 5] = ['▓', '▒', '░', '█', '✷'];
const DEBRIS: usize = 170;

pub struct BangSequence {
    pub start: f64,
    pub gained: u32,
    pub total: u32,
    pub prod_after: f64,
    pub committed: bool,
}

impl BangSequence {
    pub fn new(now: f64, gained: u32, total: u32, prod_after: f64) -> Self {
        BangSequence {
            start: now,
            gained,
            total,
            prod_after,
            committed: false,
        }
    }

    pub fn t(&self, now: f64) -> f64 {
        now - self.start
    }

    pub fn is_done(&self, now: f64) -> bool {
        self.t(now) >= DURATION
    }

    /// Après l'aspiration, la séquence occupe tout l'écran.
    pub fn is_full(&self, now: f64) -> bool {
        self.t(now) >= FULL_AT
    }

    /// Phase 1 : l'écran de jeu déjà dessiné est aspiré en spirale vers le centre.
    pub fn render_collapse(&self, buf: &mut Buffer, area: Rect, now: f64, reduced: bool) {
        let t = self.t(now);
        let p = ease_in_cubic((t / FULL_AT).clamp(0.0, 1.0));
        let src = buf.clone();
        let cx = area.x as f64 + area.width as f64 / 2.0;
        let cy = area.y as f64 + area.height as f64 / 2.0;
        let max_r = (area.width as f64 / 2.0).hypot(area.height as f64);
        let shrink = (1.0 - 0.985 * p).max(0.015);
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let dx = x as f64 + 0.5 - cx;
                let dy = (y as f64 + 0.5 - cy) * 2.0;
                let r = dx.hypot(dy);
                // Plus on est près du centre, plus ça tourne vite.
                let swirl = if reduced {
                    0.0
                } else {
                    p * 4.0 * (1.0 - (r / max_r).min(1.0))
                };
                let a = dy.atan2(dx) - swirl;
                let sr = r / shrink;
                let sx = cx + sr * a.cos();
                let sy = cy + sr * a.sin() / 2.0;
                let Some(cell) = buf.cell_mut((x, y)) else {
                    continue;
                };
                if sx < area.x as f64
                    || sy < area.y as f64
                    || sx >= area.right() as f64
                    || sy >= area.bottom() as f64
                {
                    cell.reset();
                    continue;
                }
                if let Some(s) = src.cell((sx as u16, sy as u16)) {
                    *cell = s.clone();
                }
            }
        }
        effects::fade(buf, area, 0.15 + 0.6 * p);
        // Le cœur qui avale tout s'illumine.
        let glow = ['·', '•', '●', '✷'][((p * 3.99) as usize).min(3)];
        draw::put_styled(
            buf,
            area,
            cx as i32,
            cy as i32,
            glow,
            theme::bold(theme::WHITE.lerp(theme::GOLD, 1.0 - p)),
        );
    }

    /// Phases 2 à 5 : silence, éclair, expansion, annonce.
    pub fn render_full(&self, buf: &mut Buffer, area: Rect, now: f64, stars: &StarField) {
        let t = self.t(now);
        draw::clear(buf, area);
        let cx = area.x as f64 + area.width as f64 / 2.0;
        let cy = area.y as f64 + area.height as f64 / 2.0;

        if t < COMMIT_AT {
            // Silence : un seul point qui palpite de plus en plus vite.
            let k = (t - FULL_AT) / (COMMIT_AT - FULL_AT);
            let beat = (0.5 + 0.5 * (now * (6.0 + 30.0 * k)).sin()) * (0.4 + 0.6 * k);
            let ch = if k > 0.8 {
                '✷'
            } else if k > 0.4 {
                '●'
            } else {
                '•'
            };
            let c = theme::GOLD.lerp(theme::WHITE, beat);
            draw::put_styled(buf, area, cx as i32, cy as i32, ch, theme::bold(c));
            // Un anneau de poussière qui se resserre sur lui.
            let ring = 6.0 * (1.0 - k) + 1.0;
            for i in 0..24 {
                let a = i as f64 / 24.0 * TAU + now;
                let x = cx + ring * 2.0 * a.cos();
                let y = cy + ring * a.sin();
                draw::put(
                    buf,
                    area,
                    x as i32,
                    y as i32,
                    '·',
                    theme::MUTED.dim(1.0 - k),
                );
            }
            return;
        }

        let e = t - COMMIT_AT;
        let max_r = (area.width as f64 / 2.0).hypot(area.height as f64) + 6.0;

        // L'univers naît : nébuleuse et étoiles.
        draw_nebula(buf, area, now, (e / 2.0).clamp(0.0, 1.0) * 0.85);
        draw_stars(
            buf,
            area,
            stars,
            now,
            12.0,
            ((e - 0.6) / 2.4).clamp(0.0, 1.0),
        );

        // Ondes de choc successives, du blanc au bleu.
        for k in 0..4 {
            let ek = e - k as f64 * 0.22;
            if ek <= 0.0 {
                continue;
            }
            let r = ease_out_cubic((ek / 2.2).min(1.0)) * max_r * 1.1;
            let fade = (1.0 - ek / 2.6).clamp(0.0, 1.0);
            if fade <= 0.0 {
                continue;
            }
            for y in area.top()..area.bottom() {
                for x in area.left()..area.right() {
                    let dx = x as f64 + 0.5 - cx;
                    let dy = (y as f64 + 0.5 - cy) * 2.0;
                    let band = r - dx.hypot(dy);
                    if (0.0..3.0).contains(&band) {
                        let ch = ['█', '▓', '░'][band as usize];
                        let c = gradient(&SPECTRUM, (k as f64 / 4.0 + ek * 0.35).min(1.0))
                            .dim(1.0 - fade);
                        draw::put(buf, area, x as i32, y as i32, ch, c);
                    }
                }
            }
        }

        // Débris : chacun part dans sa direction, avec une traînée.
        if e < 4.5 {
            let life = (1.0 - e / 4.5).clamp(0.0, 1.0);
            for i in 0..DEBRIS {
                let a = hash3(i as i64, 1, 7) * TAU;
                let speed = 8.0 + hash3(i as i64, 2, 7) * 46.0;
                let color = gradient(&SPECTRUM, hash3(i as i64, 3, 7));
                for trail in 0..3 {
                    let te = (e - trail as f64 * 0.06).max(0.0);
                    let r = speed * te * (1.0 - 0.08 * te);
                    let x = cx + r * a.cos();
                    let y = cy + r * a.sin() / 2.0;
                    let ch = match (trail, speed > 40.0) {
                        (0, true) => '✦',
                        (0, false) => '•',
                        (1, _) => '·',
                        _ => '.',
                    };
                    let c = color.dim(1.0 - life + trail as f64 * 0.3);
                    draw::put(buf, area, x as i32, y as i32, ch, c);
                }
            }
        }

        // Le cœur brûlant, qui refroidit.
        let core = gradient(&SPECTRUM, (e / 3.0).min(1.0)).dim((e / 4.0).min(0.8));
        draw::put_styled(buf, area, cx as i32, cy as i32, '✷', theme::bold(core));

        // Éclair blanc.
        if e < 0.5 {
            effects::flash(buf, area, theme::WHITE, 1.0 - e / 0.5);
        }

        // « BIG BANG » qui se décrypte, aux couleurs du spectre.
        let text = "BIG BANG";
        let tp = ((t - TEXT_AT) / 0.9).clamp(0.0, 1.0);
        let cyu = area.y + area.height / 2;
        if t >= TEXT_AT {
            let tw = bigfont::text_width(text);
            let len = text.chars().count();
            if tw + 4 <= area.width {
                let x0 = area.x + (area.width - tw) / 2;
                let y0 = cyu.saturating_sub(3);
                bigfont::layout(text, |dx, dy, ci, ch| {
                    let resolve = ci as f64 / len as f64 * 0.8;
                    let c = gradient_cyclic(&SPECTRUM, dx as f64 / 40.0 - now * 0.35);
                    let (g, c) = if tp >= resolve + 0.15 {
                        (ch, c)
                    } else {
                        let k = hash3(dx as i64, (now * 20.0) as i64, dy as i64);
                        (
                            GLITCH[(k * GLITCH.len() as f64) as usize % GLITCH.len()],
                            theme::WHITE.dim(0.5),
                        )
                    };
                    draw::put_styled(
                        buf,
                        area,
                        (x0 + dx) as i32,
                        (y0 + dy) as i32,
                        g,
                        theme::bold(c),
                    );
                });
                // Reflet qui traverse le titre.
                let sweep = ((t - TEXT_AT - 0.9) / 1.2).clamp(0.0, 1.0);
                if sweep > 0.0 && sweep < 1.0 {
                    let rect = Rect::new(x0, y0, tw, 5);
                    effects::shimmer(buf, rect, sweep * (tw as f64 + 10.0) - 5.0, 4.0, 0.7);
                }
            } else {
                let x = area.x + area.width.saturating_sub(len as u16) / 2;
                draw::text(
                    buf,
                    x,
                    cyu.saturating_sub(2),
                    text,
                    theme::bold(theme::GOLD),
                    area.right(),
                );
            }
        }

        // Particules gagnées, puis la promesse.
        let cp = ((t - COUNT_AT) / 0.9).clamp(0.0, 1.0);
        if t >= COUNT_AT {
            let shown = (self.gained as f64 * ease_out_cubic(cp)).round() as u32;
            let line = format!(
                "+{shown} particule{} primordiale{} ✷  ·  production ×{}",
                if self.gained > 1 { "s" } else { "" },
                if self.gained > 1 { "s" } else { "" },
                fmt::short(self.prod_after)
            );
            centered(buf, area, cyu + 3, &line, theme::bold(theme::CYAN));
            let sub = "Un nouvel univers est né : tout recommence, en plus grand.";
            let c = theme::MUTED.lerp(theme::WHITE, 0.4 * (1.0 - cp));
            centered(buf, area, cyu + 5, sub, theme::bold(c));
        }

        // Retour au jeu : fondu.
        if t > DURATION - 0.6 {
            effects::fade(buf, area, (t - (DURATION - 0.6)) / 0.6);
        }
    }
}

fn centered(buf: &mut Buffer, area: Rect, y: u16, text: &str, style: ratatui::style::Style) {
    if y >= area.bottom() {
        return;
    }
    let w = text.chars().count() as u16;
    let x = area.x + area.width.saturating_sub(w) / 2;
    draw::text(buf, x, y, text, style, area.right());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(buf: &Buffer) -> String {
        let a = buf.area;
        (0..a.height)
            .map(|y| {
                (0..a.width)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
                    + "\n"
            })
            .collect()
    }

    #[test]
    fn phases() {
        let b = BangSequence::new(10.0, 3, 3, 4.0);
        assert!(!b.is_full(11.0));
        assert!(b.is_full(11.7));
        assert!(!b.is_done(18.0));
        assert!(b.is_done(18.6));
    }

    #[test]
    fn collapse_sucks_the_screen_into_the_center() {
        let area = Rect::new(0, 0, 60, 20);
        let mut buf = Buffer::empty(area);
        for y in 0..20 {
            draw::text(&mut buf, 0, y, &"#".repeat(60), theme::fg(theme::TEXT), 60);
        }
        let b = BangSequence::new(0.0, 3, 3, 4.0);
        b.render_collapse(&mut buf, area, 1.55, false);
        let text = text_of(&buf);
        assert!(
            !text.lines().next().unwrap().contains('#'),
            "les bords sont vides"
        );
        assert!(text.contains('#') || text.contains('✷'));
    }

    #[test]
    fn announces_particles() {
        let area = Rect::new(0, 0, 100, 30);
        let mut buf = Buffer::empty(area);
        let stars = StarField::new(50, 1);
        let b = BangSequence::new(0.0, 3, 3, 4.0);
        b.render_full(&mut buf, area, COUNT_AT + 1.5, &stars);
        let text = text_of(&buf);
        assert!(
            text.contains("+3 particules primordiales ✷  ·  production ×4"),
            "{text}"
        );
        assert!(text.contains('█'), "BIG BANG en grandes lettres");
    }
}
