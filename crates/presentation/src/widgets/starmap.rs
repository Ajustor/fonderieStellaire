//! Carte stellaire (touche X, Acte III) : les systèmes, leurs routes, le
//! brouillard qui recule, les flottes en route et le détail du système visé.

use std::f64::consts::TAU;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use application::{GameSnapshot, SystemView};
use domain::SystemKind;

use super::Ctx;
use crate::draw;
use crate::fmt;
use crate::fx::easing::ease_out_cubic;
use crate::fx::effects;
use crate::fx::noise::fbm;
use crate::fx::Rgb;
use crate::theme;
use crate::visuals::StarField;
use crate::widgets::sky::draw_stars;

/// Ouverture en fondu.
const OPEN: f64 = 0.5;
const FOG: Rgb = Rgb(48, 34, 78);
const ROUTE: Rgb = Rgb(90, 90, 140);
const FLEET: Rgb = Rgb(120, 230, 255);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StarMapState {
    pub opened_at: f64,
    /// Système sélectionné.
    pub sel: usize,
}

impl StarMapState {
    pub fn new(now: f64) -> Self {
        StarMapState {
            opened_at: now,
            sel: 0,
        }
    }
}

/// Couleur propre à chaque type de système.
pub fn kind_color(kind: SystemKind) -> Rgb {
    match kind {
        SystemKind::Nebula => Rgb(255, 120, 200),
        SystemKind::Cluster => Rgb(255, 210, 110),
        SystemKind::BlackHole => Rgb(170, 90, 255),
        SystemKind::Pulsar => Rgb(110, 230, 255),
        SystemKind::Civilization => Rgb(120, 230, 140),
        SystemKind::Anomaly => Rgb(255, 150, 60),
    }
}

/// Danger : du vert (1) au rouge (5).
fn danger_color(d: u8) -> Rgb {
    theme::GREEN.lerp(theme::RED, (d.saturating_sub(1)) as f64 / 4.0)
}

/// Ce que rapporte un système exploré, en clair.
pub fn reward_text(kind: SystemKind, scale: f64) -> String {
    let pct = |per: f64| fmt::percent(per * scale);
    match kind {
        SystemKind::Nebula => format!("+{} de production, pour toujours", pct(0.10)),
        SystemKind::Cluster => format!("+{} de fragments, pour toujours", pct(0.10)),
        SystemKind::BlackHole => format!(
            "+{} h de hors-ligne au plafond, pour toujours",
            fmt::short(2.0 * scale)
        ),
        SystemKind::Pulsar => format!("comètes +{} plus fréquentes, pour toujours", pct(0.10)),
        SystemKind::Civilization => "une flotte de plus, pour toujours".to_string(),
        SystemKind::Anomaly => format!(
            "{} h de production d'un coup, à l'arrivée",
            fmt::short(scale)
        ),
    }
}

/// Zone de la carte (sous le titre, au-dessus du détail).
fn map_rect(area: Rect) -> Rect {
    let detail = 7u16;
    Rect::new(
        area.x + 2,
        area.y + 3,
        area.width.saturating_sub(4),
        area.height.saturating_sub(3 + detail + 1),
    )
}

/// Position écran d'un système.
pub fn position(map: Rect, s: &SystemView, columns: usize) -> (i32, i32) {
    let cols = columns.max(2) as f64 - 1.0;
    let x = map.x as f64 + 2.0 + s.column as f64 / cols * (map.width as f64 - 5.0);
    let y = map.y as f64 + s.y * (map.height.saturating_sub(1)) as f64;
    (x.round() as i32, y.round() as i32)
}

fn columns(snap: &GameSnapshot) -> usize {
    snap.systems.iter().map(|s| s.column).max().unwrap_or(0) + 1
}

/// Système le plus proche dans une direction (dx, dy), parmi les visibles.
pub fn neighbor(snap: &GameSnapshot, from: usize, dx: i32, dy: i32) -> usize {
    let Some(cur) = snap.systems.get(from) else {
        return from;
    };
    let px = |s: &SystemView| s.column as f64 * 4.0;
    let py = |s: &SystemView| s.y * 8.0;
    snap.systems
        .iter()
        .filter(|s| s.revealed && s.index != from)
        .filter_map(|s| {
            let vx = px(s) - px(cur);
            let vy = py(s) - py(cur);
            let along = vx * dx as f64 + vy * dy as f64;
            if along <= 0.1 {
                return None;
            }
            let across = (vx * dy as f64 - vy * dx as f64).abs();
            Some((s.index, along + across * 2.0))
        })
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(from)
}

/// Trace une route en pointillés entre deux points (Bresenham).
fn route(
    buf: &mut Buffer,
    clip: Rect,
    a: (i32, i32),
    b: (i32, i32),
    mut paint: impl FnMut(usize, usize) -> Option<(char, Rgb)>,
) {
    let (mut x, mut y) = a;
    let dx = (b.0 - a.0).abs();
    let dy = -(b.1 - a.1).abs();
    let sx = if a.0 < b.0 { 1 } else { -1 };
    let sy = if a.1 < b.1 { 1 } else { -1 };
    let mut err = dx + dy;
    let len = dx.max(-dy).max(1) as usize;
    let mut k = 0;
    loop {
        if (x, y) != a && (x, y) != b {
            if let Some((ch, c)) = paint(k, len) {
                draw::put(buf, clip, x, y, ch, c);
            }
        }
        if (x, y) == b {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
        k += 1;
    }
}

/// Lignes du détail du système sélectionné.
fn detail_lines(ctx: &Ctx, sel: usize, width: usize) -> Vec<(String, Rgb)> {
    let snap = ctx.snap;
    let mut out = Vec::new();
    let Some(s) = snap.systems.get(sel) else {
        return out;
    };
    let name = s.name.clone();
    let mut push = |t: String, c: Rgb| {
        for l in draw::wrap(&t, width) {
            out.push((l, c));
        }
    };
    if sel == 0 {
        push(format!("✶ {name} · ta base"), theme::GOLD);
        push(
            format!(
                "Flottes : {} en route sur {} · systèmes explorés : {}/{}",
                snap.fleets.len(),
                snap.fleets_max,
                snap.explored,
                snap.systems.len().saturating_sub(1)
            ),
            theme::CYAN,
        );
        if snap.converged {
            push(
                "La Convergence est accomplie : toutes les routes mènent à ta fonderie.".into(),
                theme::GOLD,
            );
        } else if let Some((left, total)) = snap.convergence {
            let p = (1.0 - left / total.max(1.0)).clamp(0.0, 1.0);
            let filled = (p * 20.0).round() as usize;
            push(
                format!(
                    "Convergence {}{} encore {} (même jeu fermé)",
                    "█".repeat(filled),
                    "░".repeat(20 - filled),
                    fmt::duration(left)
                ),
                theme::GOLD,
            );
        } else if snap.convergence_ready {
            push(
                "[Entrée] lancer la Convergence : relier tous les secteurs à la fonderie (48 h, même jeu fermé). C'est la fin de l'Acte III.".into(),
                theme::GOLD,
            );
        } else {
            push(
                format!(
                    "Convergence : explore toute la carte ({}/{}) et réunis les 16 reliques ({}/16).",
                    snap.explored,
                    snap.systems.len().saturating_sub(1),
                    snap.relics.iter().filter(|r| r.rarity.is_some()).count()
                ),
                theme::MUTED,
            );
        }
        return out;
    }
    let danger: String = (1..=5)
        .map(|i| if i <= s.danger { '◆' } else { '◇' })
        .collect();
    push(
        format!(
            "{} {name} · {} · danger {danger}",
            s.kind.symbol(),
            s.kind.label()
        ),
        kind_color(s.kind),
    );
    let reward = reward_text(s.kind, s.reward_scale);
    if s.explored {
        push(format!("Exploré : {reward}."), theme::GREEN);
        let hint = if let Some(f) = snap.fleets.iter().find(|f| f.target == sel) {
            (
                format!(
                    "Prospection en cours : retour dans {}.",
                    fmt::duration(f.remaining)
                ),
                FLEET,
            )
        } else if s.available {
            (
                format!(
                    "[Entrée] prospecter : une relique à coup sûr si elle réussit ({}, {}).",
                    fmt::duration(s.duration),
                    fmt::percent(s.chance)
                ),
                theme::GOLD,
            )
        } else {
            (
                "Prospection possible dès qu'une flotte est libre.".to_string(),
                theme::GREY,
            )
        };
        push(hint.0, hint.1);
        return out;
    }
    push(format!("Récompense : {reward}."), theme::CYAN);
    push(
        format!(
            "Durée {} (avance même jeu fermé) · chance de succès {}",
            fmt::duration(s.duration),
            fmt::percent(s.chance)
        ),
        theme::TEXT,
    );
    let hint = if let Some(f) = snap.fleets.iter().find(|f| f.target == sel) {
        (
            format!(
                "Flotte en route : arrivée dans {}.",
                fmt::duration(f.remaining)
            ),
            FLEET,
        )
    } else if s.available {
        ("[Entrée] envoyer une flotte".to_string(), theme::GOLD)
    } else if snap.fleets.len() >= snap.fleets_max {
        let next = snap
            .fleets
            .iter()
            .map(|f| f.remaining)
            .fold(f64::INFINITY, f64::min);
        (
            format!(
                "Toutes les flottes sont en route (prochain retour dans {}).",
                fmt::duration(next)
            ),
            theme::GREY,
        )
    } else {
        (
            "Hors de portée : explore d'abord un système voisin.".to_string(),
            theme::GREY,
        )
    };
    push(hint.0, hint.1);
    out
}

/// Nom d'un système (la carte est dans l'état du jeu, pas dans le catalogue).
fn system_name(ctx: &Ctx, index: usize) -> String {
    ctx.snap
        .systems
        .get(index)
        .map(|s| s.name.clone())
        .unwrap_or_default()
}

pub fn render(
    buf: &mut Buffer,
    area: Rect,
    ctx: &Ctx,
    state: &StarMapState,
    stars: &StarField,
    pops: &[(f64, usize)],
) {
    let now = ctx.now;
    let snap = ctx.snap;
    let open = ease_out_cubic(((now - state.opened_at) / OPEN).clamp(0.0, 1.0));
    if open < 0.4 {
        effects::fade(buf, area, open / 0.4);
    } else {
        draw::clear(buf, area);
    }
    draw_stars(buf, area, stars, now * 0.4, 40.0, open);

    // Titre et sous-titre.
    let title = "✶  CARTE STELLAIRE  ✶";
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
    let auto = if snap.auto_fleets {
        " · pilote auto [A]"
    } else if snap.auto_fleets_unlocked {
        " · pilote [A]"
    } else {
        ""
    };
    let sub_long = format!(
        "Explorés : {}/{} · flottes : {}/{} en route · production ×{} · fragments ×{}{auto}",
        snap.explored,
        snap.systems.len().saturating_sub(1),
        snap.fleets.len(),
        snap.fleets_max,
        fmt::short(snap.map_production),
        fmt::short(snap.map_fragments)
    );
    let sub_short = format!(
        "Explorés {}/{} · flottes {}/{}{auto}",
        snap.explored,
        snap.systems.len().saturating_sub(1),
        snap.fleets.len(),
        snap.fleets_max
    );
    let sub = if sub_long.chars().count() + 2 <= area.width as usize {
        sub_long
    } else {
        sub_short
    };
    let sw = sub.chars().count() as u16;
    draw::text(
        buf,
        area.x + area.width.saturating_sub(sw) / 2,
        area.y + 2,
        &sub,
        theme::fg(theme::MUTED.dim(1.0 - open)),
        area.right(),
    );

    let map = map_rect(area);
    if map.height < 4 || map.width < 20 {
        return;
    }
    let cols = columns(snap);
    let pos: Vec<(i32, i32)> = snap
        .systems
        .iter()
        .map(|s| position(map, s, cols))
        .collect();

    // Brouillard : au-delà de la colonne visible la plus lointaine.
    let fog_col = snap
        .systems
        .iter()
        .filter(|s| s.revealed)
        .map(|s| s.column)
        .max()
        .unwrap_or(0);
    let fog_x = if fog_col + 1 >= cols {
        i32::MAX
    } else {
        let a = map.x as f64 + 2.0 + fog_col as f64 / (cols - 1) as f64 * (map.width as f64 - 5.0);
        let b = map.x as f64
            + 2.0
            + (fog_col + 1) as f64 / (cols - 1) as f64 * (map.width as f64 - 5.0);
        ((a + b) / 2.0) as i32
    };
    for y in map.top()..map.bottom() {
        for x in map.left()..map.right() {
            let edge = x as i32 - fog_x;
            if edge < 0 {
                continue;
            }
            // Une brume légère qui s'épaissit au loin et ondule lentement.
            let n = fbm(x as f64 * 0.09 + now * 0.05, y as f64 * 0.18, 3, 77);
            let depth = (edge as f64 / 10.0).clamp(0.0, 1.0);
            let v = n * (0.4 + 0.6 * depth);
            if v > 0.48 {
                let ch = if v > 0.62 { '▒' } else { '░' };
                draw::put(
                    buf,
                    map,
                    x as i32,
                    y as i32,
                    ch,
                    FOG.dim((1.0 - open).max(0.0)),
                );
            }
        }
    }

    // Routes : explorées (avec des éclats qui circulent), visibles, en pointillés.
    for s in snap.systems.iter().filter(|s| s.revealed) {
        for &l in &links_of(ctx, s.index) {
            if l < s.index {
                continue;
            }
            let Some(o) = snap.systems.get(l) else {
                continue;
            };
            if !o.revealed {
                continue;
            }
            let both = s.explored && o.explored;
            let one = s.explored || o.explored;
            route(buf, map, pos[s.index], pos[l], |k, len| {
                if both {
                    let flow = ((now * 6.0) as usize + l) % len.max(4);
                    if k == flow {
                        Some(('•', theme::GOLD))
                    } else {
                        Some(('·', ROUTE.lerp(theme::GOLD, 0.35)))
                    }
                } else if one && k % 2 == 0 {
                    Some(('·', ROUTE))
                } else {
                    None
                }
            });
        }
    }

    // Systèmes.
    for s in &snap.systems {
        if !s.revealed {
            continue;
        }
        let (x, y) = pos[s.index];
        let (ch, c) = if s.index == 0 {
            (
                '✶',
                theme::GOLD.glow(0.25 * (0.5 + 0.5 * (now * 2.0).sin())),
            )
        } else if s.explored {
            let tw = 0.15 * (0.5 + 0.5 * (now * 2.5 + s.index as f64).sin());
            (s.kind.symbol(), kind_color(s.kind).glow(tw))
        } else {
            // Repéré mais inexploré : son type est connu, en sourdine ; le
            // danger colore un halo.
            (s.kind.symbol(), kind_color(s.kind).dim(0.55))
        };
        draw::put_styled(buf, map, x, y, ch, theme::bold(c.dim(1.0 - open)));
        if !s.explored && s.index > 0 {
            draw::put(buf, map, x - 1, y, '·', danger_color(s.danger).dim(0.4));
            draw::put(buf, map, x + 1, y, '·', danger_color(s.danger).dim(0.4));
        }
    }

    // Flottes en route : elles partent du système exploré voisin le plus proche.
    for f in &snap.fleets {
        let Some(t) = snap.systems.get(f.target) else {
            continue;
        };
        let from = links_of(ctx, f.target)
            .into_iter()
            .filter(|&l| snap.systems.get(l).is_some_and(|s| s.explored))
            .min_by_key(|&l| snap.systems[l].column)
            .unwrap_or(0);
        // Un peu au-delà du départ dès le début, pour qu'on la voie partir.
        let p = (1.0 - f.remaining / f.total.max(1.0)).clamp(0.08, 1.0);
        let (ax, ay) = pos[from];
        let (bx, by) = pos[t.index];
        for trail in (0..4).rev() {
            let q = (p - trail as f64 * 0.03).max(0.0);
            let x = ax as f64 + (bx - ax) as f64 * q;
            let y = ay as f64 + (by - ay) as f64 * q;
            let (ch, c) = if trail == 0 {
                ('▸', FLEET.glow(0.3 * (0.5 + 0.5 * (now * 5.0).sin())))
            } else {
                ('·', FLEET.dim(trail as f64 * 0.25))
            };
            draw::put_styled(
                buf,
                map,
                x.round() as i32,
                y.round() as i32,
                ch,
                theme::bold(c),
            );
        }
    }

    // Découvertes récentes : une onde lumineuse.
    for &(at, system) in pops {
        let t = (now - at) / 1.4;
        if !(0.0..1.0).contains(&t) {
            continue;
        }
        let Some(&(x, y)) = pos.get(system) else {
            continue;
        };
        let c = snap
            .systems
            .get(system)
            .map(|s| kind_color(s.kind))
            .unwrap_or(theme::GOLD);
        let r = 1.0 + t * 9.0;
        for i in 0..24 {
            let a = i as f64 / 24.0 * TAU;
            let px = x as f64 + r * 2.0 * a.cos();
            let py = y as f64 + r * a.sin();
            draw::put(
                buf,
                map,
                px.round() as i32,
                py.round() as i32,
                '✦',
                c.dim(t),
            );
        }
    }

    // Réticule de sélection et nom.
    if let Some(&(x, y)) = pos.get(state.sel) {
        let pulse = 0.5 + 0.5 * (now * 4.0).sin();
        let c = theme::WHITE.lerp(theme::CYAN, pulse);
        let d = 2 + (pulse * 1.0).round() as i32;
        for (dx, dy, ch) in [(-d, -1, '╭'), (d, -1, '╮'), (-d, 1, '╰'), (d, 1, '╯')] {
            draw::put_styled(buf, map, x + dx, y + dy, ch, theme::bold(c));
        }
        let name = system_name(ctx, state.sel);
        let w = name.chars().count() as i32;
        let nx = (x - w / 2).clamp(map.x as i32, (map.right() as i32 - w).max(map.x as i32));
        let ny = if y + 2 < map.bottom() as i32 {
            y + 2
        } else {
            y - 2
        };
        for (i, ch) in name.chars().enumerate() {
            draw::put_styled(buf, map, nx + i as i32, ny, ch, theme::bold(theme::TEXT));
        }
    }

    // Détail en bas.
    let detail_top = map.bottom() + 1;
    let width = area.width.saturating_sub(6) as usize;
    let sep: String = "─".repeat(area.width.saturating_sub(4) as usize);
    draw::text(
        buf,
        area.x + 2,
        detail_top,
        &sep,
        theme::fg(theme::BORDER),
        area.right(),
    );
    let lines = detail_lines(ctx, state.sel, width);
    let footer_y = area.bottom().saturating_sub(1);
    for (i, (l, c)) in lines.iter().enumerate() {
        let y = detail_top + 1 + i as u16;
        if y >= footer_y {
            draw::audit::record(l, 0);
            continue;
        }
        draw::text(buf, area.x + 3, y, l, theme::fg(*c), area.right());
    }
    let long = if snap.auto_fleets_unlocked {
        "[←→↑↓] naviguer   [Entrée] envoyer une flotte   [A] pilote auto   [X] / [Échap] fermer"
    } else {
        "[←→↑↓] naviguer   [Entrée] envoyer une flotte   [X] / [Échap] fermer"
    };
    let footer = if long.chars().count() + 2 <= area.width as usize {
        long
    } else {
        "[←→↑↓] naviguer  [Entrée] envoyer  [Échap] fermer"
    };
    let fw = footer.chars().count() as u16;
    draw::text(
        buf,
        area.x + area.width.saturating_sub(fw) / 2,
        footer_y,
        footer,
        theme::fg(theme::GREY),
        area.right(),
    );
}

/// Routes d'un système (lues dans la vue de l'état).
fn links_of(ctx: &Ctx, index: usize) -> Vec<usize> {
    ctx.snap
        .systems
        .get(index)
        .map(|s| s.links.clone())
        .unwrap_or_default()
}
