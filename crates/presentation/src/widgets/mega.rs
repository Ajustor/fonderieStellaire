//! Mégastructures (touche B, Acte IV) : la grille, ses liaisons qui brillent,
//! la palette des structures et le détail de la case visée.

use std::f64::consts::TAU;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Widget};

use domain::megastructure::{cell_strength, neighbors, GRID_H, GRID_W};
use domain::StructureKind;

use super::Ctx;
use crate::draw;
use crate::fmt;
use crate::fx::color::gradient_cyclic;
use crate::fx::easing::ease_out_cubic;
use crate::fx::effects;
use crate::fx::Rgb;
use crate::layout::centered;
use crate::screens::bigbang::SPECTRUM;
use crate::theme;
use crate::ui::Popup;

/// Symbole de la matière exotique.
pub const EXOTIC: char = '◈';
const LINK: Rgb = Rgb(255, 205, 110);

pub fn kind_color(k: StructureKind) -> Rgb {
    match k {
        StructureKind::Collector => Rgb(110, 230, 255),
        StructureKind::Relay => Rgb(190, 110, 255),
        StructureKind::WorldForge => Rgb(255, 150, 60),
        StructureKind::Archive => Rgb(255, 210, 110),
        StructureKind::Beacon => Rgb(120, 230, 140),
        StructureKind::Core => Rgb(255, 120, 200),
    }
}

/// Déplacement du curseur dans la grille.
pub fn navigate(sel: usize, dx: i32, dy: i32) -> usize {
    let x = (sel % GRID_W) as i32 + dx;
    let y = (sel / GRID_W) as i32 + dy;
    (y.clamp(0, GRID_H as i32 - 1) as usize) * GRID_W + x.clamp(0, GRID_W as i32 - 1) as usize
}

fn pct(x: f64) -> String {
    format!("{}%", (x * 100.0).round() as i64)
}

/// Ce que vaut la structure d'une case, en clair.
fn strength_text(k: StructureKind, s: f64) -> String {
    match k {
        StructureKind::Collector => format!("+{} {EXOTIC}/h", fmt::short(s)),
        StructureKind::Relay => "renforce ses voisines de 25 %".to_string(),
        StructureKind::WorldForge => format!("+{} de production", pct(s)),
        StructureKind::Archive => format!("+{} de fragments", pct(s)),
        StructureKind::Beacon => format!(
            "expéditions −{} de durée, +{} de succès",
            pct(0.04 * s),
            pct(0.02 * s)
        ),
        StructureKind::Core => format!("production ×{}", fmt::short(1.0 + s)),
    }
}

pub(crate) fn detail_lines(ctx: &Ctx, sel: usize, width: usize) -> Vec<(String, Style)> {
    let snap = ctx.snap;
    let mut out = Vec::new();
    let mut push = |t: String, s: Style| {
        for l in draw::wrap(&t, width) {
            out.push((l, s));
        }
    };
    // L'Éveil du Cosmos, depuis le Cœur.
    if snap.grid.get(sel).copied().flatten() == Some(StructureKind::Core) {
        if snap.awakened {
            push(
                "Le Cosmos est éveillé : production ×100, pour toujours.".into(),
                theme::bold(theme::GOLD),
            );
        } else if let Some((left, total)) = snap.awakening {
            let p = (1.0 - left / total.max(1.0)).clamp(0.0, 1.0);
            let filled = (p * 20.0).round() as usize;
            push(
                format!(
                    "Éveil du Cosmos {}{} encore {} (même jeu fermé)",
                    "█".repeat(filled),
                    "░".repeat(20 - filled),
                    fmt::duration(left)
                ),
                theme::bold(theme::GOLD),
            );
        } else if snap.awakening_ready {
            push(
                "[Entrée] lancer l'Éveil du Cosmos (72 h, même jeu fermé) : la fin de l'Acte IV."
                    .into(),
                theme::bold(theme::GOLD),
            );
        } else {
            let built = snap.grid.iter().filter(|c| c.is_some()).count();
            push(
                format!(
                    "Éveil du Cosmos : les six types bâtis et au moins {} structures ({built}/{}).",
                    domain::megastructure::AWAKENING_STRUCTURES,
                    domain::megastructure::AWAKENING_STRUCTURES
                ),
                theme::fg(theme::MUTED),
            );
        }
    }
    match snap.grid.get(sel).copied().flatten() {
        Some(k) => {
            push(
                format!(
                    "{} {} : {}",
                    k.symbol(),
                    k.name(),
                    strength_text(k, cell_strength(&snap.grid, sel))
                ),
                theme::bold(kind_color(k)),
            );
            push(k.description().to_string(), theme::fg(theme::MUTED));
            push(
                "[D] démolir (sans remboursement) pour réorganiser la grille.".into(),
                theme::fg(theme::GREY),
            );
        }
        None => {
            push(
                "Case libre : choisis une structure avec [1] à [6].".into(),
                theme::bold(theme::TEXT),
            );
            push(
                "Les voisines comptent : un Relais renforce tout ce qui le touche, une Forge-monde aime les Collecteurs, une Archive aime les Forges-mondes, un Phare préfère les bords, le Cœur veut de la variété autour de lui.".into(),
                theme::fg(theme::MUTED),
            );
        }
    }
    out
}

pub fn render(
    buf: &mut Buffer,
    area: Rect,
    ctx: &Ctx,
    popup: &Popup,
    sel: usize,
    pop: Option<(f64, usize)>,
) {
    let s = ease_out_cubic(popup.scale(ctx.now));
    if s <= 0.0 {
        return;
    }
    let snap = ctx.snap;
    let now = ctx.now;
    effects::fade(buf, area, 0.7 * s);
    let width = area.width.saturating_sub(4).min(100);
    let text_w = width.saturating_sub(4) as usize;
    let e = snap.grid_effects;
    let mut head: Vec<(String, Style)> = draw::wrap(
        &format!(
            "Matière exotique : {} {EXOTIC} (+{}/h, même jeu fermé) · production ×{} · fragments ×{} · expéditions −{}",
            fmt::short(snap.exotic),
            fmt::short(e.exotic_per_hour),
            fmt::short(e.production),
            fmt::short(e.fragments),
            pct(e.expedition_speed)
        ),
        text_w,
    )
    .into_iter()
    .map(|l| (l, theme::bold(theme::CYAN)))
    .collect();
    // Palette des structures, avec leur prix.
    let palette: Vec<String> = StructureKind::ALL
        .iter()
        .enumerate()
        .map(|(i, k)| {
            let built_core = k.unique() && snap.grid.contains(&Some(*k));
            let price = if built_core {
                "bâti".to_string()
            } else {
                format!("{} {EXOTIC}", fmt::short(snap.structure_costs[i]))
            };
            format!("[{}] {} {} {price}", i + 1, k.symbol(), k.name())
        })
        .collect();
    let palette_lines = draw::wrap(&palette.join("   "), text_w);
    head.push((String::new(), Style::default()));

    // Taille des cases selon la place : 6×3 avec liaisons, sinon 4×1.
    let big = area.width >= GRID_W as u16 * 7 + 10 && area.height >= GRID_H as u16 * 3 + 20;
    let (cw, ch) = if big { (7u16, 3u16) } else { (5u16, 1u16) };
    let grid_h = GRID_H as u16 * ch;
    let detail_h = (0..snap.grid.len().max(1))
        .map(|i| detail_lines(ctx, i, text_w).len())
        .max()
        .unwrap_or(3) as u16;
    // bordures + en-tête + grille + blanc + palette + séparateur + détail + pied
    let height =
        (2 + head.len() as u16 + grid_h + 1 + palette_lines.len() as u16 + 1 + detail_h + 1)
            .min(area.height.saturating_sub(2));
    let w = ((width as f64 * s).round() as u16).max(2);
    let h = ((height as f64 * s).round() as u16).max(2);
    let rect = centered(area, w, h);
    Clear.render(rect, buf);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::bold(
            LINK.lerp(Rgb(190, 110, 255), 0.5 + 0.5 * (now * 1.3).sin()),
        ))
        .title(
            Line::from(Span::styled(
                " ◈ MÉGASTRUCTURES ◈ ",
                theme::bold(theme::GOLD),
            ))
            .centered(),
        );
    let inner = block.inner(rect);
    block.render(rect, buf);
    if s < 0.95 || inner.height < 10 {
        return;
    }
    let right = inner.right();
    let mut y = inner.y;
    for (l, st) in &head {
        draw::text(buf, inner.x + 1, y, l, *st, right);
        y += 1;
    }

    // Grille.
    let gx = inner.x + inner.width.saturating_sub(GRID_W as u16 * cw) / 2;
    let gy = y;
    let cell_rect = |i: usize| {
        Rect::new(
            gx + (i % GRID_W) as u16 * cw,
            gy + (i / GRID_W) as u16 * ch,
            cw - 1,
            ch,
        )
    };
    for (i, &kind) in snap.grid.iter().enumerate() {
        let r = cell_rect(i);
        let is_sel = i == sel;
        let pulse = 0.5 + 0.5 * (now * 4.0).sin();
        let border = if is_sel {
            theme::WHITE.lerp(theme::CYAN, pulse)
        } else if let Some(k) = kind {
            kind_color(k).dim(0.45)
        } else {
            theme::BORDER.dim(0.4)
        };
        if ch >= 3 {
            draw::card(buf, inner, r, border);
        } else {
            let (l, rr) = if is_sel { ('▶', '◀') } else { ('[', ']') };
            draw::put(buf, inner, r.x as i32, r.y as i32, l, border);
            draw::put(buf, inner, r.right() as i32 - 1, r.y as i32, rr, border);
        }
        let cx = r.x + r.width / 2;
        let cy = r.y + r.height / 2;
        match kind {
            Some(k) => {
                let glow = 0.15 * (0.5 + 0.5 * (now * 2.0 + i as f64).sin());
                // Éveil : une vague de lumière balaie la grille ; éveillé, elle
                // scintille aux couleurs du spectre.
                let col = (i % GRID_W) as f64 + (i / GRID_W) as f64;
                let color = if snap.awakened {
                    gradient_cyclic(&SPECTRUM, col / 12.0 - now * 0.3)
                } else if snap.awakening.is_some() {
                    let wave = ((now * 2.0 - col * 0.5).sin() * 0.5 + 0.5).powi(4);
                    kind_color(k).lerp(theme::WHITE, wave)
                } else {
                    kind_color(k).glow(glow)
                };
                draw::put_styled(
                    buf,
                    inner,
                    cx as i32,
                    cy as i32,
                    k.symbol(),
                    theme::bold(color),
                );
            }
            None => draw::put(buf, inner, cx as i32, cy as i32, '·', theme::GREY.dim(0.3)),
        }
    }
    // Liaisons entre structures voisines : un éclat circule dessus.
    for i in 0..snap.grid.len() {
        let Some(a) = snap.grid[i] else { continue };
        for n in neighbors(i).filter(|&n| n > i) {
            let Some(b) = snap.grid.get(n).copied().flatten() else {
                continue;
            };
            let flow = 0.5 + 0.5 * ((now * 3.0) - (i + n) as f64 * 0.7).sin();
            let relay = a == StructureKind::Relay || b == StructureKind::Relay;
            let c = if relay {
                kind_color(StructureKind::Relay).lerp(theme::WHITE, 0.4 * flow)
            } else {
                LINK.dim(0.5 - 0.3 * flow)
            };
            let (ra, rb) = (cell_rect(i), cell_rect(n));
            if n == i + 1 {
                // Voisine de droite : liaison dans l'espace entre les cases.
                let lx = ra.right();
                let ly = ra.y + ra.height / 2;
                draw::put(buf, inner, lx as i32, ly as i32, '─', c);
            } else if ch > 1 {
                // Voisine du dessous : on raccorde les deux bordures.
                let lx = ra.x + ra.width / 2;
                draw::put(buf, inner, lx as i32, ra.bottom() as i32 - 1, '┬', c);
                draw::put(buf, inner, lx as i32, rb.y as i32, '┴', c);
            }
        }
    }
    // Construction toute fraîche : balayage lumineux puis onde.
    if let Some((at, cell)) = pop {
        let t = now - at;
        if (0.0..1.2).contains(&t) {
            let r = cell_rect(cell);
            if t < 0.4 {
                let row = r.y + ((t / 0.4) * r.height as f64) as u16;
                for x in r.x..r.right() {
                    draw::put(
                        buf,
                        inner,
                        x as i32,
                        row as i32,
                        '▀',
                        theme::WHITE.dim(t / 0.4),
                    );
                }
            }
            let rr = 1.0 + t * 8.0;
            let (cx, cy) = (
                r.x as f64 + r.width as f64 / 2.0,
                r.y as f64 + r.height as f64 / 2.0,
            );
            for k in 0..20 {
                let a = k as f64 / 20.0 * TAU;
                draw::put(
                    buf,
                    inner,
                    (cx + rr * 2.0 * a.cos()).round() as i32,
                    (cy + rr * a.sin()).round() as i32,
                    '✦',
                    LINK.dim(t / 1.2),
                );
            }
        }
    }
    y = gy + grid_h + 1;
    for l in &palette_lines {
        draw::text(buf, inner.x + 1, y, l, theme::fg(theme::GOLD), right);
        y += 1;
    }
    let sep: String = "─".repeat(inner.width.saturating_sub(2) as usize);
    draw::text(buf, inner.x + 1, y, &sep, theme::fg(theme::BORDER), right);
    y += 1;
    for (l, st) in detail_lines(ctx, sel, text_w) {
        if y >= inner.bottom().saturating_sub(1) {
            draw::audit::record(&l, 0);
            continue;
        }
        draw::text(buf, inner.x + 1, y, &l, st, right);
        y += 1;
    }
    let long = if snap.awakening_ready {
        "[←→↑↓] case   [1-6] bâtir   [D] démolir   [Entrée sur le Cœur] Éveil   [Échap] fermer"
    } else {
        "[←→↑↓] case   [1-6] bâtir   [D] démolir   [B] / [Échap] fermer"
    };
    let footer = if long.chars().count() <= inner.width as usize {
        long
    } else {
        "[1-6] bâtir  [D] démolir  [Échap]"
    };
    let fw = footer.chars().count() as u16;
    draw::text(
        buf,
        inner.x + inner.width.saturating_sub(fw) / 2,
        inner.bottom() - 1,
        footer,
        theme::fg(theme::GREY),
        right,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_stays_on_the_grid() {
        assert_eq!(navigate(0, -1, 0), 0);
        assert_eq!(navigate(0, 0, -1), 0);
        assert_eq!(navigate(0, 1, 1), GRID_W + 1);
        let last = GRID_W * GRID_H - 1;
        assert_eq!(navigate(last, 1, 1), last);
    }
}
