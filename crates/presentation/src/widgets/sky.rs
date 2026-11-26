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
use crate::visuals::{StarField, Visuals, FLOATER_LIFE};

const COMET_TRAIL: [char; 7] = ['═', '═', '═', '─', '─', '·', '·'];
pub const COMET_BANNER: &str = "● UNE COMÈTE PASSE ! [C] pour la capturer";
const COMET_BANNER_SHORT: &str = "● COMÈTE ! [C] capturer";

/// Nébuleuse en arrière-plan (value noise qui dérive lentement).
pub fn draw_nebula(buf: &mut Buffer, area: Rect, now: f64, strength: f64) {
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
                let c = gradient(&theme::NEBULA, tint).lerp(theme::NEBULA[1].glow(0.1), k * 0.5);
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
    let w = inner.width as f64;
    let h = inner.height as f64;
    let ox = inner.x as f64;
    let oy = inner.y as f64;

    // 1. Nébuleuse.
    if !ctx.reduced {
        draw_nebula(buf, inner, now, 1.0);
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
                let a = r * 0.45 + now * 0.25 + arm as f64 * PI;
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
    draw_stars(buf, inner, &vis.stars, now, 9.0, 1.0);

    // 5. Soleil (en bas à droite, partiellement hors cadre).
    let cx = w - 2.0;
    let cy = h - 0.5;
    let base_r = (2.4 + (snap.production + 1.0).log10() * 0.4)
        .min(h * 0.95)
        .min(w * 0.3);
    let r = base_r * vis.sun_scale * (1.0 + 0.05 * (now * 2.2).sin());
    let sun_q = |px: f64, py: f64| -> f64 {
        let dx = px - cx;
        let dy = (py - cy) * 2.0;
        let d = (dx * dx + dy * dy).sqrt();
        let ang = dy.atan2(dx);
        let rr = r * (1.0 + 0.08 * (ang * 6.0 + now * 2.0).sin());
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
                let c = gradient(&theme::SUN, q);
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
    for (i, def) in ctx.catalog.generators.iter().enumerate().take(9) {
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
                let a = now * 0.3 + k as f64 * TAU / count as f64;
                let (c, s) = (a.cos(), a.sin());
                let g = match (c >= 0.0, s >= 0.0) {
                    (true, false) => '◝',
                    (false, false) => '◜',
                    (false, true) => '◟',
                    (true, true) => '◞',
                };
                (cx + rr * c, cy + rr * s * 0.5, g)
            } else {
                let a_axis = r + 2.0 + (i as f64 + 1.0) * ((w * 0.9 - r - 2.0).max(6.0) / 9.0);
                let b_axis = a_axis * 0.42;
                let speed = 1.3 / (1.0 + i as f64 * 0.45);
                let a = -now * speed + k as f64 * TAU / count as f64 + i as f64 * 0.9;
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
            for (k, ch) in COMET_TRAIL.iter().enumerate() {
                let tx = hx - (k as f64 + 1.0);
                let ty = path_y((tx + 2.0) / span);
                let c = gradient(
                    &theme::COMET_TRAIL,
                    k as f64 / (COMET_TRAIL.len() - 1) as f64,
                );
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
                theme::WHITE.lerp(theme::CYAN, pulse * 0.6),
            );
            let head = (ox + hx, oy + hy);
            vis.comet.head = Some(head);
            vis.comet.trail_acc += ctx.dt;
            while vis.comet.trail_acc > 0.04 {
                vis.comet.trail_acc -= 0.04;
                let k = vis.rng.below(theme::COMET_TRAIL.len());
                vis.particles
                    .trail(head.0 - 3.0, head.1, -10.0, 0.0, theme::COMET_TRAIL[k]);
            }
            // Bannière qui clignote doucement en rose.
            let text = if (COMET_BANNER.chars().count() as u16) + 2 <= inner.width {
                COMET_BANNER
            } else {
                COMET_BANNER_SHORT
            };
            let blink = 0.5 + 0.5 * (now * 4.0).sin();
            let c = theme::PINK.dim(0.45 * (1.0 - blink)).glow(0.25 * blink);
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
