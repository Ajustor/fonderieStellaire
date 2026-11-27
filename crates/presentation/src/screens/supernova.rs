//! Séquences plein écran de la Supernova et du Big Crunch (≈ 5 s), pilotées par
//! une timeline.
//!
//! Supernova : 0,0 s soleil qui grossit + shake · 1,2 s flash blanc · 1,4 s onde
//! de choc · 2,5 s fondu au noir · 3,2 s étoiles + « +N FRAGMENTS » · 5,0 s retour.
//!
//! Big Crunch : le soleil s'effondre, flash violet, l'onde **converge** vers le
//! centre (implosion), puis « BIG CRUNCH » et le nouveau multiplicateur.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::bigfont;
use crate::draw;
use crate::fmt;
use crate::fx::easing::Easing;
use crate::fx::effects;
use crate::fx::rng::hash3;
use crate::fx::Timeline;
use crate::theme;
use crate::visuals::StarField;
use crate::widgets::sky::draw_stars;

pub const COMMIT_AT: f64 = 2.5;
pub const WAVE_AT: f64 = 1.4;
pub const REVEAL_AT: f64 = 3.2;
pub const DURATION: f64 = 5.0;
const GLITCH: [char; 6] = ['▓', '▒', '░', '█', '╳', '◆'];
const CRUNCH_VIOLET: crate::fx::Rgb = crate::fx::Rgb(170, 90, 255);

pub struct NovaSequence {
    pub start: f64,
    timeline: Timeline,
    pub gained: f64,
    pub old_bonus: f64,
    pub new_bonus: f64,
    pub committed: bool,
    pub wave_fired: bool,
    /// Big Crunch (implosion) plutôt que Supernova. Les bonus sont alors des
    /// multiplicateurs de production (×2 → ×3…).
    pub crunch: bool,
    /// Univers de destination du Big Crunch, son nom et ses couleurs.
    pub universe: domain::UniverseId,
    pub universe_name: String,
    pub palette: [crate::fx::Rgb; 3],
    /// Supernova qui ouvre un Défi : c'est le lancement du Défi (qui fait la
    /// Supernova) qui est validé au milieu de l'animation.
    pub challenge: Option<domain::ChallengeId>,
}

impl NovaSequence {
    pub fn new(now: f64, gained: f64, old_bonus: f64, new_bonus: f64) -> Self {
        let timeline = Timeline::new()
            .at("grow", 0.0, 1.2, Easing::InCubic)
            .at("flash", 1.2, 0.7, Easing::Linear)
            .at("wave", WAVE_AT, 1.1, Easing::OutCubic)
            .at("fade", COMMIT_AT, 0.7, Easing::Linear)
            .at("stars", REVEAL_AT, 1.0, Easing::Linear)
            .at("text", REVEAL_AT + 0.1, 0.8, Easing::Linear)
            .at("count", REVEAL_AT + 0.8, 0.9, Easing::OutCubic)
            .at("end", DURATION, 0.0, Easing::Linear);
        NovaSequence {
            start: now,
            timeline,
            gained,
            old_bonus,
            new_bonus,
            committed: false,
            wave_fired: false,
            crunch: false,
            universe: domain::UniverseId::ORIGIN,
            universe_name: String::new(),
            palette: theme::universe_palette(0),
            challenge: None,
        }
    }

    /// Big Crunch vers un univers donné (couleurs et nom de la destination).
    pub fn crunch_into(
        now: f64,
        crunches: u32,
        prod_before: f64,
        prod_after: f64,
        universe: domain::UniverseId,
        universe_name: &str,
    ) -> Self {
        let mut s = Self::crunch(now, crunches, prod_before, prod_after);
        s.universe = universe;
        s.universe_name = universe_name.to_string();
        s.palette = theme::universe_palette(universe.index());
        s
    }

    /// Séquence du Big Crunch : `crunches` après l'effondrement, production
    /// `prod_before` → `prod_after`.
    pub fn crunch(now: f64, crunches: u32, prod_before: f64, prod_after: f64) -> Self {
        let mut s = Self::new(now, crunches as f64, prod_before, prod_after);
        s.crunch = true;
        s
    }

    pub fn t(&self, now: f64) -> f64 {
        now - self.start
    }

    pub fn is_done(&self, now: f64) -> bool {
        self.timeline.is_done(self.t(now))
    }

    pub fn in_reveal(&self, now: f64) -> bool {
        self.t(now) >= REVEAL_AT
    }

    /// Facteur de taille du soleil du ciel.
    pub fn sun_scale(&self, now: f64) -> f64 {
        let t = self.t(now);
        if t >= REVEAL_AT {
            1.0
        } else if self.crunch {
            // Le soleil s'effondre sur lui-même.
            (1.0 - 0.95 * self.timeline.progress("grow", t)).max(0.05)
        } else {
            1.0 + 4.0 * self.timeline.progress("grow", t)
        }
    }

    /// Post-effets appliqués sur l'écran de jeu déjà dessiné (phases 1 à 4).
    pub fn render_over_game(&self, buf: &mut Buffer, area: Rect, now: f64, reduced: bool) {
        let t = self.t(now);
        let tl = &self.timeline;

        // 1. Shake qui s'intensifie.
        if !reduced && t < WAVE_AT + 0.3 {
            let intensity = tl.progress("grow", t) * 1.6;
            let (dx, dy) = effects::shake_offset(now, intensity);
            effects::offset(buf, area, dx, dy);
            // Glitch : la réalité se déchire juste avant l'explosion.
            if t > 0.5 {
                effects::glitch(
                    buf,
                    area,
                    0.03 * tl.progress("grow", t),
                    (now * 15.0) as i64,
                );
            }
        }

        // 3. Onde de choc depuis le centre.
        if t >= WAVE_AT {
            let p = tl.progress("wave", t);
            effects::fade(buf, area, 0.35 + 0.4 * p);
            let cx = area.x as f64 + area.width as f64 / 2.0;
            let cy = area.y as f64 + area.height as f64 / 2.0;
            let max_r =
                ((area.width as f64 / 2.0).powi(2) + (area.height as f64).powi(2)).sqrt() + 4.0;
            // Supernova : l'onde s'étend ; Big Crunch : elle converge vers le centre.
            let r = if self.crunch {
                (1.0 - p) * max_r
            } else {
                p * max_r
            };
            for y in area.top()..area.bottom() {
                for x in area.left()..area.right() {
                    let dx = x as f64 + 0.5 - cx;
                    let dy = (y as f64 + 0.5 - cy) * 2.0;
                    let band = r - (dx * dx + dy * dy).sqrt();
                    if (0.0..4.0).contains(&band) {
                        let ch = ['█', '▓', '▒', '░'][band as usize];
                        let c = if self.crunch {
                            self.palette[0]
                                .lerp(self.palette[1], band / 4.0)
                                .dim(p * 0.4)
                        } else {
                            theme::WHITE
                                .lerp(theme::ORANGE, band / 4.0 + p * 0.3)
                                .dim(p * 0.5)
                        };
                        draw::put(buf, area, x as i32, y as i32, ch, c);
                    }
                }
            }
        }

        // 2. Flash blanc → orange.
        if (1.2..1.9).contains(&t) {
            let amount = if t < 1.4 { 1.0 } else { 1.0 - (t - 1.4) / 0.5 };
            let color = if self.crunch {
                CRUNCH_VIOLET
                    .lerp(self.palette[0], 0.5)
                    .lerp(theme::BLACK, (t - 1.2) / 0.7)
            } else {
                theme::WHITE.lerp(theme::ORANGE, (t - 1.2) / 0.7)
            };
            effects::flash(buf, area, color, amount.clamp(0.0, 1.0));
        }

        // 4. Fondu au noir, les panneaux se dissolvent.
        if t >= COMMIT_AT {
            let p = tl.progress("fade", t);
            effects::dissolve(buf, area, p, 77);
            effects::fade(buf, area, p);
        }
    }

    /// Phase 5 : écran noir, étoiles qui se rallument, annonce des fragments.
    pub fn render_reveal(&self, buf: &mut Buffer, area: Rect, now: f64, stars: &StarField) {
        let t = self.t(now);
        let tl = &self.timeline;
        draw::clear(buf, area);
        draw_stars(buf, area, stars, now, 12.0, tl.progress("stars", t));
        if self.crunch {
            let open = tl.progress("stars", t);
            let cx = area.x as f64 + area.width as f64 / 2.0;
            let cy = area.y as f64 + area.height as f64 / 2.0;
            let radius = open * area.width.min(area.height * 2) as f64 * 0.45;
            crate::widgets::portals::draw_vortex(
                buf,
                area,
                cx,
                cy,
                radius,
                now,
                &self.palette,
                0.55,
            );
        }

        let text = if self.crunch {
            "BIG CRUNCH".to_string()
        } else {
            format!("+{} FRAGMENTS", fmt::short(self.gained))
        };
        let text_p = tl.progress("text", t);
        let tw = bigfont::text_width(&text);
        let cy = area.y + area.height / 2;
        let len = text.chars().count().max(1);
        if text_p > 0.0 {
            if tw + 4 <= area.width {
                let x0 = area.x + (area.width - tw) / 2;
                let y0 = cy.saturating_sub(3);
                bigfont::layout(&text, |dx, dy, ci, ch| {
                    let resolve = ci as f64 / len as f64 * 0.8;
                    let color = theme::logo_color(dx as f64, now);
                    let (g, c) = if text_p >= resolve + 0.15 {
                        (ch, color)
                    } else {
                        let k = hash3(dx as i64, (now * 20.0) as i64, dy as i64);
                        (
                            GLITCH[(k * GLITCH.len() as f64) as usize % GLITCH.len()],
                            theme::PINK.dim(0.4),
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
            } else {
                let w = len as u16;
                let x = area.x + area.width.saturating_sub(w) / 2;
                draw::text(
                    buf,
                    x,
                    cy.saturating_sub(2),
                    &text,
                    theme::bold(theme::GOLD),
                    area.right(),
                );
            }
        }

        let count_p = tl.progress("count", t);
        if count_p > 0.0 {
            let bonus = self.old_bonus + (self.new_bonus - self.old_bonus) * count_p;
            let line = if self.crunch {
                format!(
                    "Matière noire : {} · production ×{}",
                    self.gained,
                    fmt::short(bonus)
                )
            } else {
                format!("Bonus de production : +{}", fmt::percent(bonus))
            };
            let w = line.chars().count() as u16;
            let x = area.x + area.width.saturating_sub(w) / 2;
            draw::text(
                buf,
                x,
                cy + 2,
                &line,
                theme::bold(theme::CYAN),
                area.right(),
            );
            let sub = if self.crunch && !self.universe_name.is_empty() {
                format!("Bienvenue dans l'univers : {}", self.universe_name)
            } else if self.crunch {
                "L'univers se replie sur lui-même… puis tout recommence.".to_string()
            } else {
                "La fonderie renaît de la poussière…".to_string()
            };
            let w = sub.chars().count() as u16;
            let x = area.x + area.width.saturating_sub(w) / 2;
            let sub_color = if self.crunch {
                self.palette[0].lerp(theme::WHITE, 0.3 * (1.0 - count_p))
            } else {
                theme::MUTED.dim(1.0 - count_p)
            };
            draw::text(buf, x, cy + 4, &sub, theme::bold(sub_color), area.right());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phases() {
        let n = NovaSequence::new(10.0, 3.0, 0.1, 0.25);
        assert_eq!(n.sun_scale(10.0), 1.0);
        assert!(n.sun_scale(11.1) > 3.0);
        assert!(!n.in_reveal(12.0));
        assert!(n.in_reveal(13.3));
        assert!(!n.is_done(14.9));
        assert!(n.is_done(15.0));
    }

    #[test]
    fn crunch_implodes_and_announces() {
        let n = NovaSequence::crunch(0.0, 3, 3.0, 4.0);
        assert!(n.sun_scale(1.1) < 0.3, "le soleil s'effondre");
        let area = Rect::new(0, 0, 100, 30);
        let mut buf = Buffer::empty(area);
        let stars = StarField::new(50, 1);
        n.render_reveal(&mut buf, area, 4.95, &stars);
        let text: String = (0..30)
            .map(|y| {
                (0..100)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
                    + "\n"
            })
            .collect();
        assert!(text.contains("Matière noire : 3 · production ×4"), "{text}");
    }

    #[test]
    fn reveal_renders_fragments() {
        let n = NovaSequence::new(0.0, 3.0, 0.1, 0.25);
        let area = Rect::new(0, 0, 100, 30);
        let mut buf = Buffer::empty(area);
        let stars = StarField::new(50, 1);
        n.render_reveal(&mut buf, area, 4.95, &stars);
        let text: String = (0..30)
            .map(|y| {
                (0..100)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
                    + "\n"
            })
            .collect();
        assert!(text.contains("Bonus de production : +25%"), "{text}");
        assert!(text.contains('█'));
    }
}
