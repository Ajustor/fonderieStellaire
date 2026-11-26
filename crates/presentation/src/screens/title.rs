//! Écran titre : saut en hyperespace, logo qui se décrypte, ciel animé,
//! gains hors-ligne, invite qui pulse.

use std::f64::consts::PI;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, BorderType, Borders, Widget};

use application::GameSnapshot;

use crate::draw;
use crate::fmt;
use crate::fx::effects;
use crate::fx::rng::hash3;
use crate::fx::{FxRng, Rgb};
use crate::theme;
use crate::visuals::StarField;
use crate::widgets::sky::{draw_nebula, draw_stars};

pub const LOGO: [&str; 13] = [
    "███████╗ ██████╗ ███╗   ██╗██████╗ ███████╗██████╗ ██╗███████╗",
    "██╔════╝██╔═══██╗████╗  ██║██╔══██╗██╔════╝██╔══██╗██║██╔════╝",
    "█████╗  ██║   ██║██╔██╗ ██║██║  ██║█████╗  ██████╔╝██║█████╗  ",
    "██╔══╝  ██║   ██║██║╚██╗██║██║  ██║██╔══╝  ██╔══██╗██║██╔══╝  ",
    "██║     ╚██████╔╝██║ ╚████║██████╔╝███████╗██║  ██║██║███████╗",
    "╚═╝      ╚═════╝ ╚═╝  ╚═══╝╚═════╝ ╚══════╝╚═╝  ╚═╝╚═╝╚══════╝",
    "",
    "███████╗████████╗███████╗██╗     ██╗      █████╗ ██╗██████╗ ███████╗",
    "██╔════╝╚══██╔══╝██╔════╝██║     ██║     ██╔══██╗██║██╔══██╗██╔════╝",
    "███████╗   ██║   █████╗  ██║     ██║     ███████║██║██████╔╝█████╗  ",
    "╚════██║   ██║   ██╔══╝  ██║     ██║     ██╔══██║██║██╔══██╗██╔══╝  ",
    "███████║   ██║   ███████╗███████╗███████╗██║  ██║██║██║  ██║███████╗",
    "╚══════╝   ╚═╝   ╚══════╝╚══════╝╚══════╝╚═╝  ╚═╝╚═╝╚═╝  ╚═╝╚══════╝",
];
pub const COMPACT_TITLE: &str = "✦ FONDERIE STELLAIRE ✦";
pub const SUBTITLE: &str = "un idle game cosmique";
pub const PROMPT: &str = "▸ Appuie sur une touche pour commencer ◂";

/// Fin de l'intro (au-delà, une touche lance la partie).
pub const INTRO_END: f64 = 3.0;
const HYPER_END: f64 = 2.3;
const LOGO_START: f64 = 1.2;
const LOGO_SPAN: f64 = 1.3;
const DECRYPT_GLYPHS: [char; 6] = ['▓', '▒', '░', '█', '╳', '◆'];
const HYPER_STARS: usize = 220;

fn logo_width() -> u16 {
    LOGO.iter().map(|l| l.chars().count()).max().unwrap_or(0) as u16
}

#[derive(Clone, Copy, Debug)]
struct Star3 {
    x: f64,
    y: f64,
    z: f64,
}

/// Message affiché quand l'ancienne sauvegarde a été effacée.
const WIPE_NOTICE: &str = "L'équilibrage du jeu a été entièrement revu : ton ancienne partie a été effacée et tout le monde repart de zéro. Bonne nouvelle forge !";

pub struct TitleScreen {
    pub start: f64,
    skipped: bool,
    hyper: Vec<Star3>,
    rng: FxRng,
    /// (secondes d'absence, poussière gagnée)
    pub offline: Option<(f64, f64)>,
    /// L'ancienne sauvegarde a été effacée (nouvel équilibrage) : on prévient.
    pub wiped: bool,
}

impl TitleScreen {
    pub fn new(now: f64, seed: u64) -> Self {
        let mut rng = FxRng::new(seed);
        let hyper = (0..HYPER_STARS)
            .map(|_| Star3 {
                x: rng.range(-1.0, 1.0),
                y: rng.range(-1.0, 1.0),
                z: rng.range(0.05, 1.0),
            })
            .collect();
        TitleScreen {
            start: now,
            skipped: false,
            hyper,
            rng,
            offline: None,
            wiped: false,
        }
    }

    /// Temps écoulé dans l'intro (sauté → au moins `INTRO_END`).
    pub fn t(&self, now: f64) -> f64 {
        let t = now - self.start;
        if self.skipped {
            t.max(INTRO_END)
        } else {
            t
        }
    }

    pub fn intro_done(&self, now: f64) -> bool {
        self.t(now) >= INTRO_END
    }

    pub fn skip(&mut self) {
        self.skipped = true;
    }

    /// Vitesse d'avance des étoiles : accélère puis freine.
    fn hyper_speed(t: f64) -> f64 {
        if t >= HYPER_END {
            return 0.06;
        }
        let p = (t / HYPER_END).clamp(0.0, 1.0);
        0.12 + 2.4 * (PI * p).sin().powi(2)
    }

    pub fn update(&mut self, dt: f64, now: f64) {
        let t = self.t(now);
        if t > HYPER_END + 1.0 {
            return;
        }
        let v = Self::hyper_speed(t);
        for s in &mut self.hyper {
            s.z -= v * dt;
            if s.z <= 0.02 {
                s.x = self.rng.range(-1.0, 1.0);
                s.y = self.rng.range(-1.0, 1.0);
                s.z = self.rng.range(0.8, 1.0);
            }
        }
    }

    fn draw_hyperspace(&self, buf: &mut Buffer, area: Rect, t: f64) {
        let alpha = if t < HYPER_END {
            (t / 0.3).min(1.0)
        } else {
            (1.0 - (t - HYPER_END) / 0.7).max(0.0)
        };
        if alpha <= 0.0 {
            return;
        }
        let v = Self::hyper_speed(t);
        let cx = area.x as f64 + area.width as f64 / 2.0;
        let cy = area.y as f64 + area.height as f64 / 2.0;
        let f = area.width as f64 * 0.3;
        let project = |s: &Star3, z: f64| (cx + s.x / z * f, cy + s.y / z * f * 0.5);
        for s in &self.hyper {
            let (hx, hy) = project(s, s.z);
            let (tx, ty) = project(s, (s.z + v * 0.09).min(1.5));
            let dx = hx - tx;
            let dy = hy - ty;
            let steps = dx.abs().max(dy.abs()).ceil().min(40.0) as i32;
            let angle = (dy * 2.0).atan2(dx).to_degrees().rem_euclid(180.0);
            let line = if !(22.5..157.5).contains(&angle) {
                '─'
            } else if (67.5..112.5).contains(&angle) {
                '│'
            } else if dx * dy > 0.0 {
                '╲'
            } else {
                '╱'
            };
            let near = (1.0 - s.z).clamp(0.0, 1.0);
            let trail_color = Rgb(70, 90, 190)
                .lerp(theme::WHITE, near * 0.7)
                .dim(1.0 - alpha);
            for k in 1..=steps {
                let p = k as f64 / steps.max(1) as f64;
                let x = tx + dx * (p - 1.0 / steps.max(1) as f64);
                let y = ty + dy * (p - 1.0 / steps.max(1) as f64);
                draw::put(
                    buf,
                    area,
                    x as i32,
                    y as i32,
                    line,
                    trail_color.dim(1.0 - p * 0.6),
                );
            }
            let head = if s.z < 0.25 { '✦' } else { '·' };
            let c = theme::STAR_DIM.lerp(theme::WHITE, near).dim(1.0 - alpha);
            draw::put(buf, area, hx as i32, hy as i32, head, c);
        }
    }

    pub fn render(
        &self,
        buf: &mut Buffer,
        area: Rect,
        now: f64,
        stars: &StarField,
        snap: &GameSnapshot,
        reduced: bool,
    ) {
        let t = self.t(now);
        // Fond : nébuleuse + étoiles qui apparaissent à la fin du saut.
        let bg = ((t - 1.6) / 1.0).clamp(0.0, 1.0);
        if !reduced {
            draw_nebula(buf, area, now, bg);
        }
        draw_stars(buf, area, stars, now, 14.0, bg);
        self.draw_hyperspace(buf, area, t);

        let big_logo = area.width >= 72 && area.height >= LOGO.len() as u16 + 10;
        let logo_h = if big_logo { LOGO.len() as u16 } else { 1 };
        let offline_h = if self.offline.is_some() { 4 } else { 0 };
        let notice_w = area.width.saturating_sub(6).min(72);
        let notice = if self.wiped {
            draw::wrap(WIPE_NOTICE, notice_w.saturating_sub(4) as usize)
        } else {
            Vec::new()
        };
        let notice_h = if notice.is_empty() {
            0
        } else {
            notice.len() as u16 + 3
        };
        let total = logo_h + 2 + 2 + offline_h + notice_h + 1 + 3;
        let mut y = area.y + area.height.saturating_sub(total) / 2;

        // Logo qui se décrypte de gauche à droite, puis reflet toutes les ~4 s.
        let (logo_rect, rows): (Rect, &[&str]) = if big_logo {
            let w = logo_width();
            (
                Rect::new(area.x + area.width.saturating_sub(w) / 2, y, w, logo_h),
                &LOGO,
            )
        } else {
            let w = COMPACT_TITLE.chars().count() as u16;
            (
                Rect::new(area.x + area.width.saturating_sub(w) / 2, y, w, 1),
                std::slice::from_ref(&COMPACT_TITLE),
            )
        };
        let lw = logo_rect.width.max(1) as f64;
        if t >= LOGO_START {
            // Le logo se détache du ciel : on dégage sa zone.
            let margin = Rect::new(
                logo_rect.x.saturating_sub(1),
                logo_rect.y,
                logo_rect.width + 2,
                logo_rect.height,
            );
            draw::clear(buf, area.intersection(margin));
        }
        for (r, line) in rows.iter().enumerate() {
            for (c, ch) in line.chars().enumerate() {
                if ch == ' ' {
                    continue;
                }
                let resolve = LOGO_START
                    + (c as f64 / lw) * LOGO_SPAN * 0.75
                    + hash3(c as i64, r as i64, 3) * 0.3;
                let x = logo_rect.x as i32 + c as i32;
                let yy = logo_rect.y as i32 + r as i32;
                let color = theme::logo_color(c as f64, now);
                if t < resolve - 0.45 {
                    continue;
                } else if t < resolve {
                    let k = hash3(c as i64, (now * 20.0) as i64, r as i64);
                    let g = DECRYPT_GLYPHS
                        [(k * DECRYPT_GLYPHS.len() as f64) as usize % DECRYPT_GLYPHS.len()];
                    draw::put(buf, area, x, yy, g, color.dim(0.35));
                } else {
                    draw::put_styled(buf, area, x, yy, ch, theme::bold(color));
                }
            }
        }
        if t >= INTRO_END {
            let phase = (now - self.start - INTRO_END).rem_euclid(4.0);
            if phase < 1.0 {
                let pos = phase * (lw + 2.0 * logo_rect.height as f64 + 12.0) - 6.0;
                effects::shimmer(buf, logo_rect, pos, 4.0, 0.75);
            }
        }
        y += logo_h + 1;

        // Sous-titre.
        let sub_alpha = ((t - 2.2) / 0.6).clamp(0.0, 1.0);
        centered_text(
            buf,
            area,
            y,
            SUBTITLE,
            theme::MUTED.dim(1.0 - sub_alpha),
            false,
        );
        y += 2;

        // Gains hors-ligne.
        if let Some((secs, amount)) = self.offline {
            let msg = format!(
                "Pendant ton absence ({}), ta fonderie a produit {} ✦",
                fmt::duration(secs),
                fmt::short(amount)
            );
            let msg = draw::truncate(&msg, area.width.saturating_sub(6) as usize);
            let w = msg.chars().count() as u16 + 4;
            let rect = Rect::new(area.x + area.width.saturating_sub(w) / 2, y, w, 3);
            let rect = area.intersection(rect);
            if sub_alpha > 0.0 {
                draw::clear(buf, rect);
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(theme::fg(theme::PINK.dim(1.0 - sub_alpha)))
                    .render(rect, buf);
                draw::text(
                    buf,
                    rect.x + 2,
                    rect.y + 1,
                    &msg,
                    theme::bold(theme::GOLD.dim(1.0 - sub_alpha)),
                    rect.right().saturating_sub(1),
                );
            }
            y += offline_h;
        }

        // Remise à zéro : cadre qui pulse, texte écrit à la machine.
        if !notice.is_empty() && sub_alpha > 0.0 {
            let w = notice_w;
            let h = notice.len() as u16 + 2;
            let rect = area.intersection(Rect::new(
                area.x + area.width.saturating_sub(w) / 2,
                y,
                w,
                h,
            ));
            draw::clear(buf, rect);
            let pulse = 0.5 + 0.5 * (now * 2.5).sin();
            let border = theme::ORANGE.lerp(theme::PINK, pulse).dim(1.0 - sub_alpha);
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(theme::bold(border))
                .title(
                    ratatui::text::Line::from(ratatui::text::Span::styled(
                        " ✶ NOUVEL ÉQUILIBRAGE ✶ ",
                        theme::bold(theme::GOLD.dim(1.0 - sub_alpha)),
                    ))
                    .centered(),
                )
                .render(rect, buf);
            // Le texte s'écrit à la suite, ligne après ligne.
            let mut typed = (t - INTRO_END + 0.3) * 60.0;
            for (i, line) in notice.iter().enumerate() {
                let n = line.chars().count();
                let shown: String = line.chars().take(typed.max(0.0) as usize).collect();
                typed -= n as f64;
                draw::text(
                    buf,
                    rect.x + 2,
                    rect.y + 1 + i as u16,
                    &shown,
                    theme::bold(theme::TEXT.dim(1.0 - sub_alpha)),
                    rect.right().saturating_sub(1),
                );
            }
            y += notice_h;
        }

        // Statistiques.
        let stats = if snap.play_time <= 0.0 && snap.supernovas == 0 {
            "Nouvelle partie · la poussière d'étoile t'attend".to_string()
        } else {
            format!(
                "Big Crunch : {}  ·  Supernovas : {}  ·  Fragments : {}  ·  Temps de jeu : {}",
                snap.crunches,
                snap.supernovas,
                fmt::short(snap.fragments),
                fmt::duration(snap.play_time)
            )
        };
        centered_text(
            buf,
            area,
            y,
            &stats,
            theme::GREY.dim(1.0 - sub_alpha),
            false,
        );
        y += 2;

        // Invite qui pulse.
        if t >= INTRO_END - 0.2 {
            let b = 0.55 + 0.45 * (now * 3.0).sin();
            let c = theme::GOLD.dim(0.65 * (1.0 - b)).glow(0.2 * b);
            centered_text(buf, area, y, PROMPT, c, true);
            centered_text(buf, area, y + 1, "[Q] Quitter", theme::GREY, false);
        }
    }
}

fn centered_text(buf: &mut Buffer, area: Rect, y: u16, text: &str, color: Rgb, bold: bool) {
    if y >= area.bottom() {
        return;
    }
    let text = draw::truncate(text, area.width as usize);
    let w = text.chars().count() as u16;
    let x = area.x + area.width.saturating_sub(w) / 2;
    let style = if bold {
        theme::bold(color)
    } else {
        theme::fg(color)
    };
    draw::text(buf, x, y, &text, style, area.right());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logo_lines_fit_72_columns() {
        assert!(logo_width() <= 72);
    }

    #[test]
    fn skip_jumps_to_end_of_intro() {
        let mut t = TitleScreen::new(0.0, 1);
        assert!(!t.intro_done(1.0));
        t.skip();
        assert!(t.intro_done(1.0));
    }

    #[test]
    fn hyperspace_speeds_up_then_slows_down() {
        let a = TitleScreen::hyper_speed(0.1);
        let b = TitleScreen::hyper_speed(HYPER_END / 2.0);
        let c = TitleScreen::hyper_speed(HYPER_END * 0.95);
        assert!(b > a && b > c);
    }
}
