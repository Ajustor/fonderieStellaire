//! Le ciel vivant : nébuleuse, aurore, étoiles en parallaxe, soleil pulsant,
//! objets en orbite (reflet de la progression), étoiles filantes, comète, « +X ».

use std::f64::consts::{PI, TAU};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};

use super::Ctx;
use crate::draw;
use crate::fx::color::gradient;
use crate::fx::easing::{ease_out_back, ease_out_cubic};
use crate::fx::effects;
use crate::fx::noise::fbm;
use crate::fx::Rgb;
use crate::theme;
use crate::theme::GEN_SLOTS;
use crate::visuals::{StarField, Visuals, FLOATER_LIFE};
use domain::CometKind;

const COMET_TRAIL: [char; 7] = ['═', '═', '═', '─', '─', '·', '·'];
pub const COMET_BANNER: &str = "● UNE COMÈTE PASSE ! [C] pour la capturer";
const COMET_BANNER_SHORT: &str = "● COMÈTE ! [C] capturer";
pub const GOLDEN_BANNER: &str = "● COMÈTE DORÉE ! [C] : production ×77";
const GOLDEN_BANNER_SHORT: &str = "● DORÉE ! [C] vite !";
pub const BLACK_BANNER: &str = "● COMÈTE NOIRE ! [C] ou elle te vole";
const BLACK_BANNER_SHORT: &str = "● NOIRE ! [C] vite !";

/// Couleurs (traîne, tête, bannière) selon la sorte de comète.
fn comet_palette(kind: CometKind) -> ([Rgb; 3], Rgb, Rgb) {
    match kind {
        CometKind::Normal => (theme::COMET_TRAIL, theme::CYAN, theme::PINK),
        CometKind::Golden => (
            [theme::WHITE, theme::GOLD, theme::ORANGE],
            theme::GOLD,
            theme::GOLD,
        ),
        CometKind::Black => (
            [Rgb(200, 120, 255), Rgb(120, 40, 160), Rgb(60, 10, 50)],
            theme::RED,
            theme::RED,
        ),
    }
}

/// Nébuleuse en arrière-plan (value noise qui dérive lentement).
pub fn draw_nebula(buf: &mut Buffer, area: Rect, now: f64, strength: f64) {
    draw_nebula_tinted(buf, area, now, strength, None);
}

/// Nébuleuse teintée aux couleurs d'un univers parallèle.
pub fn draw_nebula_tinted(
    buf: &mut Buffer,
    area: Rect,
    now: f64,
    strength: f64,
    universe_tint: Option<Rgb>,
) {
    if strength <= 0.0 {
        return;
    }
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let fx = (x - area.x) as f64;
            let fy = (y - area.y) as f64;
            let n = fbm(fx * 0.07 + now * 0.035, fy * 0.17 - now * 0.012, 2, 11);
            if n > 0.56 {
                let k = ((n - 0.56) / 0.3).clamp(0.0, 1.0);
                let tint = fbm(fx * 0.03 - now * 0.01, fy * 0.08, 1, 5);
                let mut c =
                    gradient(&theme::NEBULA, tint).lerp(theme::NEBULA[1].glow(0.1), k * 0.5);
                if let Some(t) = universe_tint {
                    c = c.lerp(t.dim(0.55), 0.5);
                }
                let c = theme::BLACK.lerp(c, strength);
                let ch = if n > 0.68 { '▒' } else { '░' };
                draw::put(buf, area, x as i32, y as i32, ch, c);
            }
        }
    }
}

/// Étoiles scintillantes sur 3 couches qui dérivent à des vitesses différentes.
/// `reveal` ∈ [0, 1] limite la proportion d'étoiles allumées.
pub fn draw_stars(
    buf: &mut Buffer,
    area: Rect,
    field: &StarField,
    now: f64,
    density: f64,
    reveal: f64,
) {
    if area.is_empty() {
        return;
    }
    let w = area.width as f64;
    let h = area.height as f64;
    let wanted = ((w * h / density) as usize).min(field.stars.len());
    let lit = (wanted as f64 * reveal.clamp(0.0, 1.0)) as usize;
    const DRIFT: [f64; 3] = [0.25, 0.6, 1.3];
    for s in field.stars.iter().take(lit) {
        let layer = s.layer.min(2) as usize;
        let x = ((s.x + now * DRIFT[layer] / w.max(1.0)).fract() * w).floor();
        let y = (s.y * h).floor();
        let b = 0.5 + 0.5 * (now * s.speed + s.phase).sin();
        let ch = match layer {
            0 => '.',
            1 => {
                if b > 0.85 {
                    '+'
                } else {
                    '·'
                }
            }
            _ => {
                if b > 0.9 {
                    '✦'
                } else if b > 0.55 {
                    '+'
                } else {
                    '·'
                }
            }
        };
        let base = [0.35, 0.6, 0.95][layer];
        let c = theme::STAR_DIM.lerp(theme::STAR_BRIGHT, base * (0.35 + 0.65 * b));
        draw::put(
            buf,
            area,
            area.x as i32 + x as i32,
            area.y as i32 + y as i32,
            ch,
            c,
        );
    }
}

/// Aurore boréale (tempête solaire) : bandes sinusoïdales vert/cyan/violet.
pub fn draw_aurora(buf: &mut Buffer, area: Rect, now: f64, intensity: f64) {
    if intensity <= 0.0 {
        return;
    }
    let h = area.height as f64;
    for x in area.left()..area.right() {
        let fx = (x - area.x) as f64;
        for (k, color) in theme::AURORA.iter().enumerate() {
            let kf = k as f64;
            let center = h * 0.28
                + (fx * 0.12 + now * 1.1 + kf * 1.9).sin() * h * 0.16
                + (fx * 0.05 - now * 0.6).sin() * h * 0.08
                + kf * 0.9;
            let shimmer = 0.6 + 0.4 * (fx * 0.3 + now * 3.0 + kf).sin();
            for y in area.top()..area.bottom() {
                let d = ((y - area.y) as f64 - center).abs();
                if d < 1.2 {
                    let ch = if d < 0.5 { '▒' } else { '░' };
                    let c = color.dim(1.0 - (intensity * shimmer).clamp(0.0, 1.0) * 0.85);
                    draw::put(buf, area, x as i32, y as i32, ch, c);
                }
            }
        }
    }
}

/// Motif de la constellation (colonne, ligne) dans le coin haut-gauche du ciel.
const CONSTELLATION: [(i32, i32); 14] = [
    (2, 3),
    (5, 2),
    (8, 3),
    (11, 2),
    (14, 1),
    (17, 2),
    (20, 1),
    (23, 2),
    (21, 4),
    (18, 5),
    (15, 4),
    (12, 5),
    (9, 4),
    (6, 5),
];

/// Dessine la constellation : `lit` étoiles allumées (reliées par des points),
/// les autres en pointillé discret. Au-delà du motif, les étoiles brillent plus fort.
pub fn draw_constellation(buf: &mut Buffer, area: Rect, now: f64, lit: u32, pop: f64) {
    if area.width < 26 || area.height < 6 {
        return;
    }
    let lit = lit as usize;
    let laps = lit / CONSTELLATION.len();
    let lit_here = if laps > 0 { CONSTELLATION.len() } else { lit };
    let ox = area.x as i32;
    let oy = area.y as i32;
    // Liens entre étoiles allumées.
    for pair in CONSTELLATION.windows(2).take(lit_here.saturating_sub(1)) {
        let (a, b) = (pair[0], pair[1]);
        let steps = (b.0 - a.0).abs().max((b.1 - a.1).abs()).max(1);
        for k in 1..steps {
            let x = a.0 + (b.0 - a.0) * k / steps;
            let y = a.1 + (b.1 - a.1) * k / steps;
            if draw::is_blank(buf, ox + x, oy + y) {
                draw::put(buf, area, ox + x, oy + y, '·', theme::GOLD.dim(0.6));
            }
        }
    }
    let pop_p = (now - pop) / 0.8;
    for (i, &(x, y)) in CONSTELLATION.iter().enumerate() {
        if i < lit_here {
            let twinkle = 0.5 + 0.5 * (now * 1.7 + i as f64).sin();
            let mut c = theme::GOLD.lerp(theme::WHITE, 0.2 * twinkle + 0.1 * laps.min(5) as f64);
            let newest = i + 1 == lit_here || (laps > 0 && i == (lit % CONSTELLATION.len()));
            if newest && (0.0..1.0).contains(&pop_p) {
                c = theme::WHITE;
            }
            draw::put_styled(buf, area, ox + x, oy + y, '✦', theme::bold(c));
        } else {
            draw::put(buf, area, ox + x, oy + y, '·', theme::BORDER.dim(0.3));
        }
    }
}

fn orbit_glyph(index: usize, symbol: char) -> char {
    match index {
        0 => '·',
        _ => symbol,
    }
}

pub fn render(buf: &mut Buffer, area: Rect, ctx: &Ctx, vis: &mut Visuals) {
    let title = Line::from(vec![Span::styled(" Ciel ", theme::title_style(false))]);
    let inner = draw::panel(buf, area, title, false);
    if inner.width < 4 || inner.height < 2 {
        return;
    }
    let now = ctx.now;
    let snap = ctx.snap;
    // L'univers en cours colore et rythme le décor.
    let uni = ctx
        .catalog
        .universe(snap.universe)
        .map(|u| u.key)
        .unwrap_or("origin");
    let palette = theme::universe_palette(snap.universe.index());
    // Temps dilaté : le ciel ralentit ; Gravité forte : tout tourne plus vite.
    let tnow = match uni {
        "time" => now * 0.35,
        "gravity" => now * 1.6,
        _ => now,
    };
    let tint = (uni != "origin").then_some(palette[1]);
    let gravity = if uni == "gravity" { 1.25 } else { 1.0 };
    let w = inner.width as f64;
    let h = inner.height as f64;
    let ox = inner.x as f64;
    let oy = inner.y as f64;

    // 1. Nébuleuse.
    if !ctx.reduced {
        draw_nebula_tinted(buf, inner, tnow, 1.0, tint);
    }

    // 2. Aurore pendant la tempête solaire.
    if snap.storm_remaining > 0.0 {
        let intensity = (snap.storm_remaining / 1.5).min(1.0);
        draw_aurora(buf, inner, now, intensity);
    }

    // 3. Spirale de galaxie (Tisseurs de galaxies).
    let weavers = snap.owned.get(8).copied().unwrap_or(0);
    if weavers > 0 {
        let cx = w * 0.3;
        let cy = h * 0.45;
        let max_r = (w * 0.28).min(h * 1.6);
        let mut r = 0.6;
        while r < max_r {
            for arm in 0..2 {
                let a = r * 0.45 + tnow * 0.25 + arm as f64 * PI;
                let px = ox + cx + r * a.cos();
                let py = oy + cy + r * a.sin() * 0.5;
                if draw::is_blank(buf, px as i32, py as i32) {
                    let c = Rgb(120, 90, 170).lerp(Rgb(40, 30, 70), r / max_r);
                    draw::put(buf, inner, px as i32, py as i32, '·', c);
                }
            }
            r += 0.5;
        }
    }

    // 4. Étoiles.
    // Entropie : les étoiles s'éteignent à mesure que le cycle s'étire.
    draw_stars(buf, inner, &vis.stars, tnow, 9.0, snap.entropy);

    // 4 bis. Constellation : chaque niveau acheté allume une étoile.
    if snap.constellation_levels > 0 || snap.fragments > 0.0 {
        draw_constellation(
            buf,
            inner,
            now,
            snap.constellation_levels,
            vis.constellation_pop,
        );
    }

    // 4 ter. Grand Attracteur : une galaxie qui grandit à chaque étape.
    if snap.attractor_stages > 0 || snap.attractor_build.is_some() {
        let building = snap
            .attractor_build
            .map(|b| 1.0 - b.remaining / b.total.max(1.0));
        draw_attractor(
            buf,
            inner,
            tnow,
            now,
            snap.attractor_stages,
            ctx.catalog.attractor.len(),
            building,
            vis.attractor_pop,
        );
    }

    // 4 quater. Les mégastructures, en constellation au-dessus de la fonderie.
    if snap.grid.iter().any(|c| c.is_some()) {
        draw_megastructures(buf, inner, now, snap);
    }

    // 5. Soleil (en bas à droite, partiellement hors cadre).
    let cx = w - 2.0;
    let cy = h - 0.5;
    let base_r = (2.4 + (snap.production + 1.0).log10() * 0.4)
        .min(h * 0.95)
        .min(w * 0.3);
    let r = base_r * vis.sun_scale * gravity * (1.0 + 0.05 * (tnow * 2.2).sin());
    let sun_q = |px: f64, py: f64| -> f64 {
        let dx = px - cx;
        let dy = (py - cy) * 2.0;
        let d = (dx * dx + dy * dy).sqrt();
        let ang = dy.atan2(dx);
        let rr = r * (1.0 + 0.08 * (ang * 6.0 + tnow * 2.0).sin());
        d / rr.max(0.1)
    };
    let x_from = (cx - r * 1.2 - 1.0).max(0.0) as u16;
    let y_from = (cy - r * 0.7 - 1.0).max(0.0) as u16;
    for ly in y_from..inner.height {
        for lx in x_from..inner.width {
            let q = sun_q(lx as f64 + 0.5, ly as f64 + 0.5);
            if q < 1.0 {
                let ch = if q < 0.45 {
                    '█'
                } else if q < 0.7 {
                    '▓'
                } else if q < 0.88 {
                    '▒'
                } else {
                    '░'
                };
                // L'entropie ternit le soleil.
                let c =
                    gradient(&theme::SUN, q).lerp(theme::GREY.dim(0.3), (1.0 - snap.entropy) * 0.7);
                draw::put(
                    buf,
                    inner,
                    (inner.x + lx) as i32,
                    (inner.y + ly) as i32,
                    ch,
                    c,
                );
            }
        }
    }

    // 6. Objets en orbite : nombre = log2(possédés) + 1.
    let first_frame = !vis.orbit_ready;
    let gen_count = ctx.catalog.generators.len().clamp(1, GEN_SLOTS) as f64;
    for (i, def) in ctx.catalog.generators.iter().enumerate().take(GEN_SLOTS) {
        let owned = snap.owned.get(i).copied().unwrap_or(0);
        let count = if owned == 0 {
            0
        } else {
            ((owned as f64).log2().floor() as usize + 1).min(8)
        };
        if count > vis.orbit_count[i] && !first_frame {
            vis.orbit_pop[i] = now;
        }
        vis.orbit_count[i] = count;
        if count == 0 {
            continue;
        }
        let color = theme::generator_color(i);
        let pop = (now - vis.orbit_pop[i]) / 0.6;
        for k in 0..count {
            let newest = k == count - 1 && (0.0..1.0).contains(&pop);
            let (px, py, glyph) = if i == 4 {
                // Sphères de Dyson : arcs autour du soleil.
                let rr = r + 1.4;
                let a = tnow * 0.3 + k as f64 * TAU / count as f64;
                let (c, s) = (a.cos(), a.sin());
                let g = match (c >= 0.0, s >= 0.0) {
                    (true, false) => '◝',
                    (false, false) => '◜',
                    (false, true) => '◟',
                    (true, true) => '◞',
                };
                (cx + rr * c, cy + rr * s * 0.5, g)
            } else {
                let a_axis =
                    r + 2.0 + (i as f64 + 1.0) * ((w * 0.9 - r - 2.0).max(6.0) / gen_count);
                let b_axis = a_axis * 0.42;
                let speed = 1.3 / (1.0 + i as f64 * 0.45);
                let a = -tnow * speed + k as f64 * TAU / count as f64 + i as f64 * 0.9;
                (
                    cx + a_axis * a.cos(),
                    cy + b_axis * a.sin(),
                    orbit_glyph(i, def.symbol),
                )
            };
            if sun_q(px, py) < 1.0 {
                continue;
            }
            let (glyph, c) = if newest {
                let e = ease_out_back(pop);
                let g = if pop < 0.4 { '✦' } else { glyph };
                (g, color.glow((1.0 - pop) * 0.8 * e.min(1.0)))
            } else {
                (glyph, color)
            };
            draw::put(
                buf,
                inner,
                (ox + px).floor() as i32,
                (oy + py).floor() as i32,
                glyph,
                c,
            );
            if newest && pop < 0.5 && !ctx.reduced {
                let ring = 1.0 + ease_out_cubic(pop * 2.0) * 1.5;
                for (dx, dy) in [
                    (ring, 0.0),
                    (-ring, 0.0),
                    (0.0, ring * 0.5),
                    (0.0, -ring * 0.5),
                ] {
                    draw::put(
                        buf,
                        inner,
                        (ox + px + dx).floor() as i32,
                        (oy + py + dy).floor() as i32,
                        '·',
                        color.dim(pop),
                    );
                }
            }
        }
    }
    vis.orbit_ready = true;

    // 6 bis. Clés d'univers : un anneau par Clé autour du soleil, avec une
    // perle brillante qui en fait le tour (sens alternés).
    let mut ring = 0;
    for (ui, view) in snap.universes.iter().enumerate() {
        if !view.key {
            continue;
        }
        let rr = r + 2.6 + ring as f64 * 1.3;
        let color = theme::universe_color(ui);
        let dots = ((rr * 2.4) as usize).max(10);
        for k in (0..dots).step_by(2) {
            let a = k as f64 * TAU / dots as f64 + tnow * 0.05;
            let px = ox + cx + rr * a.cos();
            let py = oy + cy + rr * a.sin() * 0.5;
            if draw::is_blank(buf, px.floor() as i32, py.floor() as i32) {
                draw::put(
                    buf,
                    inner,
                    px.floor() as i32,
                    py.floor() as i32,
                    '·',
                    color.dim(0.45),
                );
            }
        }
        let dir = if ring % 2 == 0 { 1.0 } else { -1.0 };
        let a = dir * now * 0.9 + ui as f64;
        draw::put_styled(
            buf,
            inner,
            (ox + cx + rr * a.cos()).floor() as i32,
            (oy + cy + rr * a.sin() * 0.5).floor() as i32,
            '◆',
            theme::bold(color.glow(0.3)),
        );
        ring += 1;
    }

    // 7. Étoiles filantes.
    for s in &vis.shooting {
        let age = now - s.born;
        let p = age / s.life;
        let hx = s.x + s.vx * age;
        let hy = s.y + s.vy * age;
        let len = (s.vx * s.vx + s.vy * s.vy).sqrt().max(0.1);
        let (ux, uy) = (s.vx / len, s.vy / len);
        for k in 0..5 {
            let kx = hx - ux * k as f64 * 1.4;
            let ky = hy - uy * k as f64 * 0.7;
            let ch = if k == 0 {
                '•'
            } else if k < 3 {
                '─'
            } else {
                '·'
            };
            let c = theme::WHITE.dim((k as f64 * 0.18 + p * 0.6).min(0.95));
            draw::put(buf, inner, (ox + kx) as i32, (oy + ky) as i32, ch, c);
        }
    }

    // 8. Comète.
    match snap.comet_progress {
        Some(p) => {
            let span = w + 12.0;
            let path_y = |pp: f64| vis.comet.y0 + vis.comet.amp * (pp.clamp(0.0, 1.0) * PI).sin();
            let hx = -2.0 + p * span;
            let hy = path_y(p);
            let (trail, head_glow, banner_color) = comet_palette(snap.comet_kind);
            for (k, ch) in COMET_TRAIL.iter().enumerate() {
                let tx = hx - (k as f64 + 1.0);
                let ty = path_y((tx + 2.0) / span);
                let c = gradient(&trail, k as f64 / (COMET_TRAIL.len() - 1) as f64);
                draw::put(
                    buf,
                    inner,
                    (ox + tx).floor() as i32,
                    (oy + ty).floor() as i32,
                    *ch,
                    c,
                );
            }
            let pulse = 0.5 + 0.5 * (now * 10.0).sin();
            draw::put(
                buf,
                inner,
                (ox + hx).floor() as i32,
                (oy + hy).floor() as i32,
                '●',
                theme::WHITE.lerp(head_glow, pulse * 0.6),
            );
            let head = (ox + hx, oy + hy);
            vis.comet.head = Some(head);
            vis.comet.trail_acc += ctx.dt;
            while vis.comet.trail_acc > 0.04 {
                vis.comet.trail_acc -= 0.04;
                let k = vis.rng.below(trail.len());
                vis.particles
                    .trail(head.0 - 3.0, head.1, -10.0, 0.0, trail[k]);
            }
            // Bannière qui clignote doucement en rose.
            let (long, short) = match snap.comet_kind {
                CometKind::Normal => (COMET_BANNER, COMET_BANNER_SHORT),
                CometKind::Golden => (GOLDEN_BANNER, GOLDEN_BANNER_SHORT),
                CometKind::Black => (BLACK_BANNER, BLACK_BANNER_SHORT),
            };
            let text = if (long.chars().count() as u16) + 2 <= inner.width {
                long
            } else {
                short
            };
            let touch_text;
            let text = if vis.touch {
                touch_text = crate::mobile::touch_text(text);
                touch_text.as_str()
            } else {
                text
            };
            let blink = 0.5 + 0.5 * (now * 4.0).sin();
            let c = banner_color.dim(0.45 * (1.0 - blink)).glow(0.25 * blink);
            let tw = text.chars().count() as u16;
            let bx = inner.x + inner.width.saturating_sub(tw) / 2;
            draw::text(buf, bx, inner.y, text, theme::bold(c), inner.right());
        }
        None => {
            vis.comet.head = None;
        }
    }

    // 9. Textes « +X » qui montent et s'estompent.
    for f in &vis.floaters {
        let p = ((now - f.born) / FLOATER_LIFE).clamp(0.0, 1.0);
        let y = f.y - ease_out_cubic(p) * 3.0;
        let c = f.color.lerp(Rgb(20, 20, 30), p * p);
        let len = f.text.chars().count() as f64;
        let x0 = (f.x - len / 2.0).floor() as i32;
        for (i, ch) in f.text.chars().enumerate() {
            draw::put_styled(
                buf,
                inner,
                x0 + i as i32,
                y.floor() as i32,
                ch,
                theme::bold(c),
            );
        }
    }

    // 10. Flash du panneau (capture de comète…).
    let fp = (now - vis.sky_flash) / 0.45;
    if (0.0..1.0).contains(&fp) {
        effects::flash(buf, inner, vis.sky_flash_color, (1.0 - fp) * 0.75);
    }
}

/// Le Grand Attracteur dans le ciel : deux bras en spirale autour d'un cœur
/// doré, un peu plus grands et plus brillants à chaque étape achevée. L'étape
/// en chantier se dessine en pointillés qui se remplissent ; une étape achevée
/// envoie une onde lumineuse.
#[allow(clippy::too_many_arguments)]
fn draw_attractor(
    buf: &mut Buffer,
    inner: Rect,
    tnow: f64,
    now: f64,
    stages: usize,
    total: usize,
    building: Option<f64>,
    pop: Option<(f64, usize)>,
) {
    let w = inner.width as f64;
    let h = inner.height as f64;
    let cx = inner.x as f64 + w * 0.14;
    let cy = inner.y as f64 + h * 0.42;
    let total = total.max(1) as f64;
    let k = stages as f64 / total;
    let max_r = (w * 0.16).min(h * 1.1).max(3.0);
    let radius = max_r * (0.35 + 0.65 * k);
    let gold = Rgb(255, 210, 110);
    let violet = Rgb(190, 110, 255);
    let blue = Rgb(90, 150, 255);
    let arm = |r: f64, a0: f64| -> (f64, f64) {
        let a = a0 + r * 0.55 - tnow * 0.4;
        (cx + r * a.cos(), cy + r * a.sin() * 0.5)
    };
    // Bras achevés.
    let mut r = 0.8;
    while r < radius {
        for a0 in [0.0, PI] {
            let (x, y) = arm(r, a0);
            let t = r / radius;
            let c = gold
                .lerp(violet, t)
                .lerp(blue, (t - 0.6).max(0.0))
                .dim(0.55 - 0.45 * k);
            let ch = if t < 0.35 { '•' } else { '·' };
            draw::put(buf, inner, x as i32, y as i32, ch, c);
        }
        r += 0.45;
    }
    // Étape en chantier : le prolongement des bras, en pointillés qui se remplissent.
    if let Some(p) = building {
        let next = max_r * (0.35 + 0.65 * ((stages + 1) as f64 / total));
        let mut r = radius;
        let pulse = 0.5 + 0.5 * (now * 3.0).sin();
        while r < next {
            let done = (r - radius) / (next - radius).max(0.1) <= p;
            for a0 in [0.0, PI] {
                let (x, y) = arm(r, a0);
                let c = if done {
                    violet.dim(0.4)
                } else {
                    Rgb(70, 60, 110).lerp(violet, 0.3 * pulse)
                };
                draw::put(
                    buf,
                    inner,
                    x as i32,
                    y as i32,
                    if done { '·' } else { '.' },
                    c,
                );
            }
            r += 0.9;
        }
    }
    // Cœur qui palpite, de plus en plus éclatant.
    let beat = 0.5 + 0.5 * (now * 2.0).sin();
    let core = gold.glow(0.2 + 0.3 * beat * k);
    let ch = if stages >= total as usize {
        '✶'
    } else {
        '◆'
    };
    draw::put_styled(buf, inner, cx as i32, cy as i32, ch, theme::bold(core));
    // Onde d'une étape achevée.
    if let Some((at, _)) = pop {
        let t = (now - at) / 1.6;
        if (0.0..1.0).contains(&t) {
            let rr = 1.0 + t * max_r * 2.2;
            for i in 0..32 {
                let a = i as f64 / 32.0 * 2.0 * PI;
                let x = cx + rr * a.cos();
                let y = cy + rr * a.sin() * 0.5;
                draw::put(buf, inner, x as i32, y as i32, '✦', gold.dim(t));
            }
        }
    }
}

/// La grille des Mégastructures, en petit, dans le ciel : chaque structure est
/// une étoile de sa couleur, reliée à ses voisines. Pendant l'Éveil, une vague
/// de lumière la parcourt ; éveillée, elle scintille aux couleurs du spectre.
fn draw_megastructures(buf: &mut Buffer, inner: Rect, now: f64, snap: &application::GameSnapshot) {
    use domain::megastructure::{neighbors, GRID_H, GRID_W};
    let w = inner.width as f64;
    let h = inner.height as f64;
    let sx = (w * 0.30 / GRID_W as f64).clamp(1.0, 3.0);
    let sy = (h * 0.45 / GRID_H as f64).clamp(0.5, 1.5);
    let ox = inner.x as f64 + w * 0.46;
    let oy = inner.y as f64 + h * 0.08;
    let pos = |i: usize| {
        (
            (ox + (i % GRID_W) as f64 * sx).round() as i32,
            (oy + (i / GRID_W) as f64 * sy).round() as i32,
        )
    };
    for (i, c) in snap.grid.iter().enumerate() {
        if c.is_none() {
            continue;
        }
        for n in neighbors(i).filter(|&n| n > i) {
            if snap.grid.get(n).copied().flatten().is_none() {
                continue;
            }
            let (a, b) = (pos(i), pos(n));
            let (mx, my) = ((a.0 + b.0) / 2, (a.1 + b.1) / 2);
            if (mx, my) != a && (mx, my) != b {
                draw::put(buf, inner, mx, my, '·', Rgb(120, 110, 170));
            }
        }
    }
    for (i, c) in snap.grid.iter().enumerate() {
        let Some(k) = c else { continue };
        let (x, y) = pos(i);
        let col = (i % GRID_W) as f64 + (i / GRID_W) as f64;
        let base = crate::widgets::mega::kind_color(*k);
        let color = if snap.awakened {
            crate::fx::color::gradient_cyclic(
                &crate::screens::bigbang::SPECTRUM,
                col / 12.0 - now * 0.3,
            )
        } else if snap.awakening.is_some() {
            base.lerp(
                Rgb(255, 255, 255),
                ((now * 2.0 - col * 0.5).sin() * 0.5 + 0.5).powi(4),
            )
        } else {
            base.dim(0.25 - 0.2 * (now * 1.5 + i as f64).sin())
        };
        let ch = if *k == domain::StructureKind::Core {
            '✶'
        } else {
            '✦'
        };
        draw::put(buf, inner, x, y, ch, color);
    }
}
