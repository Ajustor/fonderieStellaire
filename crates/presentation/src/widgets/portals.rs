//! Écran des portails : choix de l'univers du prochain cycle (au Big Crunch),
//! ou simple consultation (touche V).
//!
//! Chaque univers est une carte avec un **vortex animé** à ses couleurs. Les
//! cartes arrivent en vol à l'ouverture, glissent avec rebond quand on change
//! de choix, et le portail choisi s'ouvre jusqu'à engloutir l'écran.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::Ctx;
use crate::draw;
use crate::fmt;
use crate::fx::color::gradient;
use crate::fx::easing::{ease_in_cubic, ease_out_back, ease_out_cubic};
use crate::fx::effects;
use crate::fx::Rgb;
use crate::theme;
use crate::visuals::StarField;
use crate::widgets::sky::draw_stars;

/// Durée du glissement entre deux cartes.
pub const SLIDE: f64 = 0.35;
/// Durée de l'arrivée des cartes à l'ouverture.
pub const OPEN: f64 = 0.6;
/// Durée de la plongée dans le portail choisi.
pub const DIVE: f64 = 0.9;

/// Densité du vortex, du vide au plein.
const RAMP: [char; 6] = ['·', ':', '░', '▒', '▓', '█'];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortalState {
    pub opened_at: f64,
    /// Univers sélectionné (index).
    pub sel: usize,
    /// Position de départ du glissement en cours, et son début.
    pub slide_from: f64,
    pub slide_at: f64,
    /// Instant où l'on a plongé dans le portail (mode Big Crunch).
    pub chosen_at: Option<f64>,
    /// Vrai : choix pour un Big Crunch ; faux : simple consultation.
    pub crunch: bool,
}

impl PortalState {
    pub fn new(now: f64, sel: usize, crunch: bool) -> Self {
        PortalState {
            opened_at: now,
            sel,
            slide_from: sel as f64,
            slide_at: f64::NEG_INFINITY,
            chosen_at: None,
            crunch,
        }
    }

    /// Position (fractionnaire) du carrousel, avec rebond.
    pub fn position(&self, now: f64) -> f64 {
        let p = ((now - self.slide_at) / SLIDE).clamp(0.0, 1.0);
        self.slide_from + (self.sel as f64 - self.slide_from) * ease_out_back(p)
    }

    /// Change de carte en partant de la position affichée (pas de saut).
    pub fn select(&mut self, now: f64, sel: usize) {
        if sel == self.sel {
            return;
        }
        self.slide_from = self.position(now);
        self.slide_at = now;
        self.sel = sel;
    }

    pub fn dive_progress(&self, now: f64) -> Option<f64> {
        self.chosen_at.map(|c| ((now - c) / DIVE).clamp(0.0, 1.0))
    }
}

/// Vortex : bras en spirale qui tournent vers le centre, densité et couleur
/// selon la distance. `intensity` ∈ [0, 1] règle sa présence.
#[allow(clippy::too_many_arguments)]
pub fn draw_vortex(
    buf: &mut Buffer,
    clip: Rect,
    cx: f64,
    cy: f64,
    radius: f64,
    now: f64,
    palette: &[Rgb; 3],
    intensity: f64,
) {
    if radius < 0.5 || intensity <= 0.0 {
        return;
    }
    let y0 = (cy - radius / 2.0 - 1.0).floor().max(clip.y as f64) as u16;
    let y1 = (cy + radius / 2.0 + 1.0).ceil().min(clip.bottom() as f64) as u16;
    let x0 = (cx - radius - 1.0).floor().max(clip.x as f64) as u16;
    let x1 = (cx + radius + 1.0).ceil().min(clip.right() as f64) as u16;
    for y in y0..y1 {
        for x in x0..x1 {
            let dx = x as f64 + 0.5 - cx;
            let dy = (y as f64 + 0.5 - cy) * 2.0;
            let d = (dx * dx + dy * dy).sqrt();
            if d > radius {
                continue;
            }
            let q = d / radius;
            let ang = dy.atan2(dx);
            // Trois bras qui s'enroulent vers le centre et tournent.
            let swirl = ang * 3.0 + (1.0 - q) * 7.0 - now * 2.6;
            let arm = 0.5 + 0.5 * swirl.sin();
            let core = (1.0 - q / 0.22).max(0.0);
            let v = ((arm * (1.0 - q.powi(3)) * 0.85 + core) * intensity).clamp(0.0, 1.0);
            if v < 0.14 {
                continue;
            }
            let idx = (((v - 0.14) / 0.86) * RAMP.len() as f64) as usize;
            let ch = RAMP[idx.min(RAMP.len() - 1)];
            let c = gradient(palette, q).glow(core * 0.6 + arm * 0.15 * (1.0 - q));
            draw::put(buf, clip, x as i32, y as i32, ch, c);
        }
    }
}

/// Bordure arrondie dessinée à la main, découpée par `clip` (les cartes
/// peuvent dépasser de l'écran pendant le glissement).
fn card_border(buf: &mut Buffer, clip: Rect, x: i32, y: i32, w: i32, h: i32, color: Rgb) {
    for i in 1..w - 1 {
        draw::put(buf, clip, x + i, y, '─', color);
        draw::put(buf, clip, x + i, y + h - 1, '─', color);
    }
    for j in 1..h - 1 {
        draw::put(buf, clip, x, y + j, '│', color);
        draw::put(buf, clip, x + w - 1, y + j, '│', color);
    }
    draw::put(buf, clip, x, y, '╭', color);
    draw::put(buf, clip, x + w - 1, y, '╮', color);
    draw::put(buf, clip, x, y + h - 1, '╰', color);
    draw::put(buf, clip, x + w - 1, y + h - 1, '╯', color);
}

fn text_clipped(buf: &mut Buffer, clip: Rect, x: i32, y: i32, text: &str, color: Rgb, bold: bool) {
    let style = if bold {
        theme::bold(color)
    } else {
        theme::fg(color)
    };
    for (i, ch) in text.chars().enumerate() {
        draw::put_styled(buf, clip, x + i as i32, y, ch, style);
    }
}

fn centered_line(buf: &mut Buffer, area: Rect, y: u16, text: &str, color: Rgb, bold: bool) {
    let text = draw::truncate(text, area.width.saturating_sub(2) as usize);
    let w = text.chars().count() as i32;
    let x = area.x as i32 + (area.width as i32 - w) / 2;
    text_clipped(buf, area, x, y as i32, &text, color, bold);
}

pub fn render(buf: &mut Buffer, area: Rect, ctx: &Ctx, state: &PortalState, stars: &StarField) {
    let now = ctx.now;
    let snap = ctx.snap;
    let open = ease_out_cubic(((now - state.opened_at) / OPEN).clamp(0.0, 1.0));

    // Fond : le jeu s'efface, un ciel étoilé prend sa place.
    if open < 0.45 {
        effects::fade(buf, area, open / 0.45);
    } else {
        draw::clear(buf, area);
    }
    draw_stars(buf, area, stars, now * 0.6, 10.0, open);

    // Titre en dégradé, sous-titre.
    let title = if state.crunch {
        "✶  CHOISIS TON UNIVERS  ✶"
    } else {
        "✶  UNIVERS PARALLÈLES  ✶"
    };
    let tw = title.chars().count() as i32;
    let tx = area.x as i32 + (area.width as i32 - tw) / 2;
    for (i, ch) in title.chars().enumerate() {
        let c = theme::logo_color(i as f64 * 3.0, now).dim(1.0 - open);
        draw::put_styled(
            buf,
            area,
            tx + i as i32,
            area.y as i32 + 1,
            ch,
            theme::bold(c),
        );
    }
    let keys = snap.universes.iter().filter(|u| u.key).count();
    let keys_total = ctx
        .catalog
        .universes
        .iter()
        .filter(|u| u.key_reward.is_some())
        .count();
    let subtitle = if state.crunch {
        let n = snap.crunches + 1;
        let prod = fmt::short(1.0 + domain::catalog::balance::CRUNCH_PRODUCTION * n as f64);
        let long = format!(
            "Big Crunch n°{n} · production ×{prod} · ses règles s'appliquent jusqu'au prochain Big Crunch"
        );
        if long.chars().count() + 2 <= area.width as usize {
            long
        } else {
            format!("Big Crunch n°{n} · production ×{prod} · règles valables tout le cycle")
        }
    } else {
        format!("Clés obtenues : {keys}/{keys_total} · le choix se fait au Big Crunch [U]")
    };
    centered_line(
        buf,
        area,
        area.y + 2,
        &subtitle,
        theme::MUTED.dim(1.0 - open),
        false,
    );

    // Carrousel de cartes.
    let n = ctx.catalog.universes.len();
    let card_w = (area.width as i32 / 3 - 2).clamp(22, 34);
    let card_h = (area.height as i32 - 7).clamp(12, 24);
    let gap = 3;
    let pos = state.position(now);
    let area_cx = area.x as f64 + area.width as f64 / 2.0;
    // Arrivée en vol depuis le bas.
    let fly = ((1.0 - ease_out_back(open)) * area.height as f64).round() as i32;
    let card_y = area.y as i32 + 4 + fly;
    let dive = state.dive_progress(now);

    for i in 0..n {
        let Some(def) = ctx.catalog.universes.get(i) else {
            continue;
        };
        let view = snap.universes.get(i);
        let offset = i as f64 - pos;
        let center_x = area_cx + offset * (card_w + gap) as f64;
        let x = (center_x - card_w as f64 / 2.0).round() as i32;
        if x + card_w < area.x as i32 || x > area.right() as i32 {
            continue;
        }
        let selected = i == state.sel;
        let dist = offset.abs();
        // Les cartes éloignées s'estompent ; pendant la plongée, tout sauf l'élue disparaît.
        let mut dim = (dist * 0.32).min(0.75);
        if let Some(p) = dive {
            if !selected {
                dim = dim.max(p);
            }
        }
        let palette = theme::universe_palette(i);
        let pulse = 0.5 + 0.5 * (now * 3.0).sin();
        let border = if selected {
            palette[0].glow(0.25 * pulse)
        } else {
            palette[1].dim(0.3 + dim * 0.6)
        };
        // Intérieur de la carte : on efface ce qu'il y avait derrière.
        let visible = area.intersection(Rect::new(
            x.max(0) as u16,
            card_y.max(0) as u16,
            (card_w + x.min(0)).max(0) as u16,
            card_h.max(0) as u16,
        ));
        draw::clear(buf, visible);
        card_border(buf, area, x, card_y, card_w, card_h, border);
        if view.is_some_and(|v| v.current) {
            text_clipped(
                buf,
                area,
                x + 2,
                card_y,
                " ◆ ICI ",
                palette[0].dim(dim),
                true,
            );
        }

        // Textes de la carte, renvoyés à la ligne : nom et règles en haut,
        // Clé et bonus en bas ; le vortex prend la place qui reste.
        let text_w = (card_w - 4).max(1) as usize;
        let has_key = view.is_some_and(|v| v.key);
        let (key_line, reward_line, key_color) = if def.key_reward.is_none() {
            (
                "Aucune Clé ici".to_string(),
                "Les règles de toujours".to_string(),
                theme::GREY,
            )
        } else if has_key {
            (
                format!("★ {} obtenue", def.key_name),
                def.reward_text.to_string(),
                theme::GOLD,
            )
        } else {
            (
                format!("Clé : {} Supernovas", def.key_goal),
                def.reward_text.to_string(),
                theme::MUTED,
            )
        };
        let name_lines = draw::wrap(def.name, text_w);
        let rule_lines = draw::wrap(def.rules_text, text_w);
        let key_lines = draw::wrap(&key_line, text_w);
        let reward_lines = draw::wrap(&reward_line, text_w);
        let top_n = (name_lines.len() + rule_lines.len()) as i32;
        let bottom_n = (key_lines.len() + reward_lines.len()) as i32;
        let vortex_h = (card_h - 2 - top_n - bottom_n - 1).max(3);
        let vcx = x as f64 + card_w as f64 / 2.0;
        let vcy = card_y as f64 + 1.0 + vortex_h as f64 / 2.0;
        let radius = ((card_w - 4) as f64 / 2.0).min(vortex_h as f64 - 0.5);
        let spin = if selected { now } else { now * 0.35 + i as f64 };
        let inner_clip = area.intersection(Rect::new(
            (x + 1).max(0) as u16,
            (card_y + 1).max(0) as u16,
            (card_w - 2).max(0) as u16,
            vortex_h.max(0) as u16,
        ));
        draw_vortex(
            buf,
            inner_clip,
            vcx,
            vcy,
            radius,
            spin,
            &palette,
            if selected { 1.0 } else { 0.75 } * (1.0 - dim),
        );
        // Symbole de l'univers au cœur, qui pulse.
        let core = palette[0].glow(0.5 + 0.5 * pulse).dim(dim);
        draw::put_styled(
            buf,
            area,
            vcx.floor() as i32,
            vcy.floor() as i32,
            def.symbol,
            theme::bold(core),
        );

        // Textes : nom, règles, Clé.
        let text_x = x + 2;
        let mut ty = card_y + 1 + vortex_h;
        for line in &name_lines {
            text_clipped(buf, area, text_x, ty, line, palette[0].dim(dim), true);
            ty += 1;
        }
        for line in &rule_lines {
            text_clipped(buf, area, text_x, ty, line, theme::TEXT.dim(dim), false);
            ty += 1;
        }
        let mut ty = card_y + card_h - 1 - bottom_n;
        for line in &key_lines {
            text_clipped(buf, area, text_x, ty, line, key_color.dim(dim), true);
            ty += 1;
        }
        for line in &reward_lines {
            text_clipped(
                buf,
                area,
                text_x,
                ty,
                line,
                theme::CYAN.dim(dim.max(0.2)),
                false,
            );
            ty += 1;
        }
    }

    // Aide.
    let footer = if state.crunch {
        "[←→] choisir   [Entrée] plonger dans ce portail   [Échap] annuler"
    } else {
        "[←→] parcourir   [V] / [Échap] fermer"
    };
    centered_line(
        buf,
        area,
        area.bottom().saturating_sub(1),
        footer,
        theme::GREY.dim(1.0 - open),
        false,
    );

    // Plongée : le portail choisi s'ouvre jusqu'à engloutir l'écran, puis flash.
    if let Some(p) = dive {
        let palette = theme::universe_palette(state.sel);
        let vcx = area_cx;
        let vcy = card_y as f64 + 1.0 + ((card_h - 8).max(4)) as f64 / 2.0;
        let max_r = (area.width as f64).hypot(area.height as f64 * 2.0);
        let radius = 4.0 + ease_in_cubic(p) * max_r;
        draw_vortex(
            buf,
            area,
            vcx,
            vcy,
            radius,
            now * (1.0 + 3.0 * p),
            &palette,
            1.0,
        );
        if p > 0.75 {
            effects::flash(buf, area, palette[0].glow(0.6), (p - 0.75) / 0.25);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carousel_slides_without_jumping() {
        let mut s = PortalState::new(0.0, 2, true);
        assert_eq!(s.position(0.0), 2.0);
        s.select(1.0, 3);
        assert_eq!(s.position(1.0), 2.0, "part de la position affichée");
        assert!((s.position(1.0 + SLIDE) - 3.0).abs() < 1e-9);
        // Changement en plein glissement : repart de la position courante.
        s.select(1.0 + SLIDE / 2.0, 4);
        let mid = s.position(1.0 + SLIDE / 2.0);
        assert!(mid > 2.0 && mid < 3.6, "{mid}");
    }

    #[test]
    fn dive_progress() {
        let mut s = PortalState::new(0.0, 0, true);
        assert_eq!(s.dive_progress(5.0), None);
        s.chosen_at = Some(1.0);
        assert!((s.dive_progress(1.0 + DIVE / 2.0).unwrap() - 0.5).abs() < 1e-9);
        assert_eq!(s.dive_progress(10.0), Some(1.0));
    }

    #[test]
    fn vortex_draws_inside_its_radius_only() {
        let area = Rect::new(0, 0, 40, 20);
        let mut buf = Buffer::empty(area);
        draw_vortex(
            &mut buf,
            area,
            20.0,
            10.0,
            8.0,
            1.0,
            &theme::universe_palette(1),
            1.0,
        );
        let drawn: Vec<(u16, u16)> = (0..20)
            .flat_map(|y| (0..40).map(move |x| (x, y)))
            .filter(|&(x, y)| buf[(x, y)].symbol() != " ")
            .collect();
        assert!(!drawn.is_empty());
        for (x, y) in drawn {
            let dx = x as f64 + 0.5 - 20.0;
            let dy = (y as f64 + 0.5 - 10.0) * 2.0;
            assert!(dx.hypot(dy) <= 8.0 + 1e-9);
        }
    }

    #[test]
    fn wrapping() {
        assert_eq!(draw::wrap("aa bb cc", 5), ["aa bb", "cc"]);
    }
}
