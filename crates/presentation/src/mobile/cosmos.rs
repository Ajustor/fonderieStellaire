//! Écrans mobiles de l'onglet Cosmos (ouverts depuis son menu) et la
//! confirmation « glisser pour confirmer ». Les textes viennent des mêmes
//! fonctions que les fenêtres du terminal ; seule la mise en page change.

use domain::megastructure::{neighbors, GRID_H, GRID_W};
use domain::StructureKind;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use super::screens::{button, place, scrollbar, separator, set_scroll_max, touchify, BUTTON_H};
use super::{Confirm, Confirming, Hit, MobileUi, Page};
use crate::draw;
use crate::fmt;
use crate::fx::Rgb;
use crate::theme;
use crate::visuals::Visuals;
use crate::widgets::cosmos::{self as cosmogony, CosmosItem};
use crate::widgets::starmap::{self, StarMapState};
use crate::widgets::{challenges, daily, event, lab, mega, relics, Ctx};

pub const EVENT: usize = 10;
pub const DAILY: usize = 11;
pub const CHALLENGES: usize = 12;
pub const LAB: usize = 13;
pub const COSMOGONY: usize = 14;
pub const RELICS: usize = 15;
pub const MEGA: usize = 16;
pub const MAP: usize = 17;

/// Largeur du bouton d'action des cartes.
const ACTION_W: u16 = 13;
/// Hauteur de la carte stellaire virtuelle (la vue glisse dessus).
const MAP_VIRTUAL_W: u16 = 96;
const MAP_VIRTUAL_H: u16 = 44;

pub fn page_title(p: Page) -> &'static str {
    match p {
        Page::Event => "Événement cosmique",
        Page::Daily => "Défi du jour",
        Page::Challenges => "Défis",
        Page::Lab => "Laboratoire",
        Page::Cosmogony => "Cosmogonie",
        Page::StarMap => "Carte stellaire",
        Page::Relics => "Reliques",
        Page::Mega => "Mégastructures",
    }
}

pub fn page_screen(p: Page) -> usize {
    match p {
        Page::Event => EVENT,
        Page::Daily => DAILY,
        Page::Challenges => CHALLENGES,
        Page::Lab => LAB,
        Page::Cosmogony => COSMOGONY,
        Page::StarMap => MAP,
        Page::Relics => RELICS,
        Page::Mega => MEGA,
    }
}

/// Retire les touches du clavier d'un texte du terminal (« [Entrée] lancer »
/// devient « lancer ») ; renvoie `None` si la ligne n'était qu'une aide clavier.
pub fn strip_keys(s: &str) -> Option<String> {
    let s = touchify(s);
    let mut out = String::with_capacity(s.len());
    let mut rest = s.as_str();
    let mut removed = false;
    while let Some(start) = rest.find('[') {
        let Some(len) = rest[start..].find(']') else {
            break;
        };
        let inner = &rest[start + 1..start + len];
        if inner.chars().count() <= 8 && !inner.contains(' ') {
            out.push_str(&rest[..start]);
            rest = rest[start + len + 1..].trim_start();
            removed = true;
        } else {
            out.push_str(&rest[..start + len + 1]);
            rest = &rest[start + len + 1..];
        }
    }
    out.push_str(rest);
    let out = touchify(out.trim());
    // Une ligne qui n'était qu'une aide clavier (« [Entrée] envoyer une
    // flotte ») disparaît : le bouton est juste à côté.
    if removed && out.chars().count() < 25 {
        return None;
    }
    Some(out)
}

/// Lignes d'un widget du terminal, nettoyées et renvoyées à la ligne.
fn clean(lines: Vec<(String, Style)>, width: usize) -> Vec<(String, Style)> {
    let mut out = Vec::new();
    for (l, s) in lines {
        if l.trim().is_empty() {
            out.push((String::new(), s));
            continue;
        }
        if let Some(t) = strip_keys(&l) {
            for w in draw::wrap(&t, width) {
                out.push((w, s));
            }
        }
    }
    while out.last().is_some_and(|(l, _)| l.is_empty()) {
        out.pop();
    }
    out
}

fn rgb_lines(lines: Vec<(String, Rgb)>) -> Vec<(String, Style)> {
    lines.into_iter().map(|(l, c)| (l, theme::fg(c))).collect()
}

// ----------------------------------------------------------------------
// Liste de cartes de hauteur variable
// ----------------------------------------------------------------------

struct Action {
    title: String,
    label: String,
    color: Rgb,
    hit: Option<Hit>,
}

enum Row {
    Header(String, Rgb),
    Text(Vec<(String, Style)>),
    Card {
        title: (String, Style),
        status: (String, Rgb),
        lines: Vec<(String, Style)>,
        action: Option<Action>,
    },
    /// Gros bouton pleine largeur.
    Wide(Action),
}

fn row_height(r: &Row) -> u16 {
    match r {
        Row::Header(..) => 2,
        Row::Text(l) => l.len() as u16 + 1,
        Row::Card { lines, .. } => (1 + lines.len() as u16).max(3) + 1,
        Row::Wide(_) => BUTTON_H + 1,
    }
}

fn render_rows(buf: &mut Buffer, area: Rect, m: &mut MobileUi, screen: usize, rows: &[Row]) {
    let total: u16 = rows.iter().map(row_height).sum();
    set_scroll_max(m, screen, total, area.height);
    let scroll = m.scroll[screen];
    let mut vy = 0;
    for row in rows {
        let h = row_height(row);
        let at = vy;
        vy += h;
        let Some(r) = place(area, scroll, at, h) else {
            continue;
        };
        match row {
            Row::Header(t, c) => {
                draw::text(buf, r.x + 1, r.y + 1, t, theme::bold(*c), r.right());
            }
            Row::Text(lines) => {
                for (i, (l, s)) in lines.iter().enumerate() {
                    draw::text(buf, r.x + 1, r.y + i as u16, l, *s, r.right());
                }
            }
            Row::Card {
                title,
                status,
                lines,
                action,
            } => {
                let body_w = if action.is_some() {
                    r.width.saturating_sub(ACTION_W + 1)
                } else {
                    r.width
                };
                let st_x = draw::text_right(
                    buf,
                    r.x + body_w.saturating_sub(1),
                    r.y,
                    &status.0,
                    theme::bold(status.1),
                    r.x + 4,
                );
                draw::text(
                    buf,
                    r.x + 1,
                    r.y,
                    &draw::truncate(&title.0, st_x.saturating_sub(r.x + 2) as usize),
                    title.1,
                    st_x.saturating_sub(1),
                );
                for (i, (l, s)) in lines.iter().enumerate() {
                    draw::text(buf, r.x + 1, r.y + 1 + i as u16, l, *s, r.x + body_w);
                }
                if let Some(a) = action {
                    let b = Rect::new(r.right() - ACTION_W, r.y, ACTION_W, BUTTON_H);
                    button(
                        buf,
                        &mut m.hits,
                        m.press,
                        b,
                        &a.title,
                        &a.label,
                        a.color,
                        a.hit,
                    );
                }
                separator(buf, r.x + 1, r.bottom() - 1, r.width.saturating_sub(2));
            }
            Row::Wide(a) => {
                let b = Rect::new(r.x + 1, r.y, r.width.saturating_sub(2), BUTTON_H);
                button(
                    buf,
                    &mut m.hits,
                    m.press,
                    b,
                    &a.title,
                    &a.label,
                    a.color,
                    a.hit,
                );
            }
        }
    }
    scrollbar(buf, area, scroll, m.scroll_max[screen]);
}

/// Largeur du texte d'une carte avec bouton.
fn body_width(area: Rect) -> usize {
    area.width.saturating_sub(ACTION_W + 3) as usize
}

fn action(title: &str, label: &str, color: Rgb, hit: Option<Hit>) -> Option<Action> {
    Some(Action {
        title: title.to_string(),
        label: label.to_string(),
        color,
        hit,
    })
}

// ----------------------------------------------------------------------
// Pages
// ----------------------------------------------------------------------

pub fn render_page(
    buf: &mut Buffer,
    area: Rect,
    ctx: &Ctx,
    vis: &mut Visuals,
    m: &mut MobileUi,
    page: Page,
    today: u64,
) {
    // Barre de titre : retour au menu Cosmos.
    let bar = Rect::new(area.x, area.y, area.width, 1);
    draw::text(
        buf,
        bar.x + 1,
        bar.y,
        "‹ Cosmos",
        theme::fg(theme::MUTED),
        bar.right(),
    );
    draw::text_right(
        buf,
        bar.right().saturating_sub(2),
        bar.y,
        page_title(page),
        theme::bold(theme::GOLD),
        bar.x + 10,
    );
    m.hits.add(Rect::new(bar.x, bar.y, 12, 1), Hit::ClosePage);
    let area = Rect::new(
        area.x,
        area.y + 2,
        area.width,
        area.height.saturating_sub(2),
    );
    let rows = match page {
        Page::Event => event_rows(ctx, area),
        Page::Daily => daily_rows(ctx, area, today),
        Page::Challenges => challenge_rows(ctx, area),
        Page::Lab => lab_rows(ctx, area),
        Page::Cosmogony => cosmogony_rows(ctx, area),
        Page::StarMap => {
            render_map(buf, area, ctx, vis, m);
            return;
        }
        Page::Relics => {
            render_relics(buf, area, ctx, m);
            return;
        }
        Page::Mega => {
            render_mega(buf, area, ctx, vis, m);
            return;
        }
    };
    render_rows(buf, area, m, page_screen(page), &rows);
}

fn event_rows(ctx: &Ctx, area: Rect) -> Vec<Row> {
    let w = area.width.saturating_sub(3) as usize;
    let Some((index, left)) = ctx.snap.pending_event else {
        return vec![Row::Text(vec![(
            "Aucun événement en attente.".into(),
            theme::fg(theme::MUTED),
        )])];
    };
    let Some(def) = ctx.catalog.events.get(index) else {
        return Vec::new();
    };
    let mut rows = vec![Row::Header(def.title.to_uppercase(), theme::GOLD)];
    rows.push(Row::Text(
        draw::wrap(def.text, w)
            .into_iter()
            .map(|l| (l, theme::fg(theme::TEXT)))
            .collect(),
    ));
    for (i, c) in def.choices.iter().enumerate() {
        let color = if c.risk.is_some() {
            theme::ORANGE
        } else {
            theme::CYAN
        };
        let mut lines: Vec<(String, Style)> = draw::wrap(c.label, w)
            .into_iter()
            .map(|l| (l, theme::bold(theme::TEXT)))
            .collect();
        lines.extend(
            draw::wrap(&format!("→ {}", event::choice_text(c)), w)
                .into_iter()
                .map(|l| (l, theme::fg(color))),
        );
        rows.push(Row::Text(lines));
        rows.push(Row::Wide(Action {
            title: String::new(),
            label: format!("Choisir : {}", c.label),
            color: if i == 0 { theme::GOLD } else { theme::FOCUS },
            hit: Some(Hit::EventChoice(i)),
        }));
    }
    rows.push(Row::Text(vec![(
        format!(
            "Réponds avant {} — sinon l'occasion passe.",
            fmt::duration(left)
        ),
        theme::fg(theme::MUTED),
    )]));
    rows
}

fn daily_rows(ctx: &Ctx, area: Rect, today: u64) -> Vec<Row> {
    let w = area.width.saturating_sub(3) as usize;
    let lines = clean(daily::lines(ctx, 400, today), w);
    let mut rows = vec![Row::Text(lines)];
    let (label, color) = if ctx.snap.daily.is_some() {
        ("Quitter l'univers de poche", theme::GREY)
    } else {
        ("Entrer dans l'univers de poche", theme::GOLD)
    };
    rows.push(Row::Wide(Action {
        title: String::new(),
        label: label.into(),
        color,
        hit: Some(Hit::DailyToggle),
    }));
    rows
}

fn challenge_rows(ctx: &Ctx, area: Rect) -> Vec<Row> {
    let snap = ctx.snap;
    let w = body_width(area);
    let mut rows = vec![Row::Text(
        draw::wrap(
            "Un Défi relance le cycle avec une contrainte. Réussis l'objectif : récompense pour toujours.",
            area.width.saturating_sub(3) as usize,
        )
        .into_iter()
        .map(|l| (l, theme::fg(theme::MUTED)))
        .collect(),
    )];
    for (i, v) in snap.challenges.iter().enumerate() {
        let Some(def) = ctx.catalog.challenge(v.id) else {
            continue;
        };
        let (status, color) = challenges::status(ctx, v, def.goal, def.unlock_supernovas);
        let lines: Vec<(String, Style)> = clean(challenges::detail_lines(ctx, i, false, 400), w)
            .into_iter()
            .take(6)
            .collect();
        let possible = v.active || (v.unlocked && !v.done && snap.active_challenge.is_none());
        let act = if v.done {
            action("", "✓ réussi", theme::GREEN, None)
        } else if v.active {
            action(
                "",
                "Abandonner",
                theme::RED,
                Some(Hit::Ask(Confirm::Challenge(i))),
            )
        } else if possible {
            action(
                "",
                "Lancer",
                theme::PINK,
                Some(Hit::Ask(Confirm::Challenge(i))),
            )
        } else {
            action("", "verrouillé", theme::GREY, None)
        };
        rows.push(Row::Card {
            title: (
                format!("{} {}", def.symbol, def.name),
                if v.unlocked {
                    theme::bold(theme::TEXT)
                } else {
                    theme::fg(theme::GREY)
                },
            ),
            status: (if v.active { status } else { String::new() }, color),
            lines,
            action: act,
        });
    }
    rows
}

fn lab_rows(ctx: &Ctx, area: Rect) -> Vec<Row> {
    let snap = ctx.snap;
    let full = area.width.saturating_sub(3) as usize;
    let w = body_width(area);
    let mut rows = Vec::new();
    rows.push(Row::Text(vec![(
        format!(
            "✶ {} libres sur {} · bonus ×{}",
            snap.dark_matter_spendable,
            snap.crunches,
            fmt::short(snap.crunch_production)
        ),
        theme::bold(theme::CYAN),
    )]));
    // Recherche en cours, épinglée en haut.
    match snap.research {
        Some(r) => {
            let name = ctx.catalog.lab_node(r.id).map(|d| d.name).unwrap_or("?");
            let p = if r.total > 0.0 {
                1.0 - r.remaining / r.total
            } else {
                1.0
            };
            let bar_w = full.saturating_sub(2);
            let filled = (p.clamp(0.0, 1.0) * bar_w as f64).round() as usize;
            rows.push(Row::Text(vec![
                (format!("En cours : {name}"), theme::bold(theme::TEXT)),
                (
                    format!("{}{}", "━".repeat(filled), "─".repeat(bar_w - filled)),
                    theme::fg(Rgb(170, 90, 255).glow(0.3)),
                ),
                (
                    format!(
                        "encore {} · avance même jeu fermé",
                        fmt::duration(r.remaining)
                    ),
                    theme::fg(theme::MUTED),
                ),
            ]));
        }
        None => rows.push(Row::Text(vec![(
            "Aucune recherche en cours : choisis-en une.".into(),
            theme::fg(theme::MUTED),
        )])),
    }
    let order = lab::display_order(ctx);
    for (title, belongs) in lab::SECTIONS {
        let short = title.split(" ·").next().unwrap_or(title);
        rows.push(Row::Header(short.to_string(), theme::GOLD));
        for (pos, &i) in order.iter().enumerate() {
            let Some(v) = snap.lab.get(i) else { continue };
            let Some(def) = ctx.catalog.lab_node(v.id) else {
                continue;
            };
            if !belongs(def.effect) {
                continue;
            }
            let toggleable = def.effect.is_toggleable();
            let (status, color) = lab::status(snap, v, def.cost, def.research_secs, toggleable);
            let lines: Vec<(String, Style)> = clean(lab::detail_lines(ctx, &order, pos, 400), w)
                .into_iter()
                .take(4)
                .collect();
            let researching = snap.research.is_some_and(|r| r.id == v.id);
            let act = if v.done && toggleable {
                if v.paused {
                    action("", "Activer", theme::GREEN, Some(Hit::Lab(pos)))
                } else {
                    action("", "Pause", theme::GREY, Some(Hit::Lab(pos)))
                }
            } else if v.done {
                action("", "✓ acquis", theme::GREEN, None)
            } else if researching {
                action("", "en cours", theme::CYAN, None)
            } else if v.blocked {
                action("", "verrouillé", theme::GREY, None)
            } else if v.available && snap.research.is_none() {
                action(
                    &format!("{} ✶", def.cost),
                    "Rechercher",
                    theme::GOLD,
                    Some(Hit::Lab(pos)),
                )
            } else {
                action(&format!("{} ✶", def.cost), "Rechercher", theme::GREY, None)
            };
            rows.push(Row::Card {
                title: (
                    format!("{} {}", def.symbol, def.name),
                    if v.blocked {
                        theme::fg(theme::GREY)
                    } else {
                        theme::bold(theme::TEXT)
                    },
                ),
                status: (status, color),
                lines,
                action: act,
            });
        }
    }
    rows
}

fn cosmogony_rows(ctx: &Ctx, area: Rect) -> Vec<Row> {
    let snap = ctx.snap;
    let full = area.width.saturating_sub(3) as usize;
    let w = body_width(area);
    let mut rows = vec![Row::Text(clean(
        rgb_lines(cosmogony::head_lines(ctx, 400)),
        full,
    ))];
    let n = cosmogony::item_count(ctx);
    let mut last_section = "";
    for i in 0..n {
        let Some(item) = cosmogony::item_at(ctx, i) else {
            continue;
        };
        let section = match item {
            CosmosItem::BigBang => "BIG BANG",
            CosmosItem::Forge(_) => "FORGE PRIMORDIALE",
            CosmosItem::Stage(_) => "GRAND ATTRACTEUR",
        };
        if section != last_section {
            rows.push(Row::Header(section.to_string(), theme::GOLD));
            last_section = section;
        }
        let (symbol, name, status, color, done) = cosmogony::describe(ctx, item);
        let lines: Vec<(String, Style)> = clean(rgb_lines(cosmogony::detail(ctx, item, false)), w)
            .into_iter()
            .take(5)
            .collect();
        let act = match item {
            CosmosItem::BigBang if snap.bigbang_available => action(
                &format!("+{} ✷", snap.particles_available),
                "Big Bang",
                theme::GOLD,
                Some(Hit::Ask(Confirm::BigBang)),
            ),
            CosmosItem::BigBang => action("", "pas prêt", theme::GREY, None),
            CosmosItem::Forge(f) => match snap.forges.get(f) {
                Some(v) if v.forged => action("", "✓ forgé", theme::GREEN, None),
                Some(v) if v.available => {
                    action("", "Forger", theme::GOLD, Some(Hit::Cosmogony(i)))
                }
                _ => action("", "Forger", theme::GREY, None),
            },
            CosmosItem::Stage(s) if s < snap.attractor_stages => {
                action("", "✓ achevée", theme::GREEN, None)
            }
            CosmosItem::Stage(s)
                if s == snap.attractor_stages
                    && snap.attractor_build.is_none()
                    && snap.attractor_buildable =>
            {
                action("", "Bâtir", theme::GOLD, Some(Hit::Cosmogony(i)))
            }
            CosmosItem::Stage(s) if snap.attractor_build.is_some_and(|b| b.stage == s) => {
                action("", "en chantier", theme::CYAN, None)
            }
            CosmosItem::Stage(_) => action("", "plus tard", theme::GREY, None),
        };
        rows.push(Row::Card {
            title: (
                format!("{symbol} {name}"),
                if done {
                    theme::fg(theme::GREEN)
                } else {
                    theme::bold(theme::TEXT)
                },
            ),
            status: (status, color),
            lines,
            action: act,
        });
    }
    rows
}

// ----------------------------------------------------------------------
// Carte stellaire : la carte du terminal, rendue en grand et parcourue au doigt
// ----------------------------------------------------------------------

fn map_virtual_area() -> Rect {
    Rect::new(0, 0, MAP_VIRTUAL_W, MAP_VIRTUAL_H)
}

/// Taille de la vue de la carte dans la page.
fn map_view(area: Rect) -> Rect {
    let sheet = 9 + BUTTON_H;
    Rect::new(
        area.x,
        area.y,
        area.width,
        area.height.saturating_sub(sheet),
    )
}

/// Décalage maximal de la vue sur la carte virtuelle.
fn map_pan_max(view: Rect) -> (i32, i32) {
    let map = starmap::map_rect(map_virtual_area());
    (
        (map.width as i32 - view.width as i32).max(0),
        (map.height as i32 - view.height as i32).max(0),
    )
}

pub fn clamp_pan(m: &mut MobileUi, view_w: u16, view_h: u16) {
    let (mx, my) = map_pan_max(Rect::new(0, 0, view_w, view_h));
    m.map_pan.0 = m.map_pan.0.clamp(0, mx);
    m.map_pan.1 = m.map_pan.1.clamp(0, my);
}

/// Système le plus proche de la cellule (`col`, `row`) de la vue.
pub fn system_at(
    ctx_snap: &application::GameSnapshot,
    m: &MobileUi,
    view: Rect,
    col: u16,
    row: u16,
) -> Option<usize> {
    let map = starmap::map_rect(map_virtual_area());
    let vx = map.x as i32 + m.map_pan.0 + (col as i32 - view.x as i32);
    let vy = map.y as i32 + m.map_pan.1 + (row as i32 - view.y as i32);
    let columns = starmap::columns(ctx_snap);
    ctx_snap
        .systems
        .iter()
        .filter(|s| s.revealed)
        .map(|s| {
            let (x, y) = starmap::position(map, s, columns);
            (s.index, (x - vx).abs() + 2 * (y - vy).abs())
        })
        .filter(|&(_, d)| d <= 4)
        .min_by_key(|&(_, d)| d)
        .map(|(i, _)| i)
}

fn render_map(buf: &mut Buffer, area: Rect, ctx: &Ctx, vis: &mut Visuals, m: &mut MobileUi) {
    let snap = ctx.snap;
    let view = map_view(area);
    m.map_sel = m.map_sel.min(snap.systems.len().saturating_sub(1));
    let virt = map_virtual_area();
    let map = starmap::map_rect(virt);
    if !m.map_centered {
        // Première ouverture : vue centrée sur un système où une flotte peut partir.
        if let Some(s) = snap.systems.iter().find(|s| s.available) {
            m.map_sel = s.index;
        }
        if let Some(s) = snap.systems.get(m.map_sel) {
            let (x, y) = starmap::position(map, s, starmap::columns(snap));
            m.map_pan = (
                x - map.x as i32 - view.width as i32 / 2,
                y - map.y as i32 - view.height as i32 / 2,
            );
        }
        m.map_centered = true;
    }
    clamp_pan(m, view.width, view.height);

    let mut off = Buffer::empty(virt);
    let state = StarMapState {
        opened_at: ctx.now - 10.0,
        sel: m.map_sel,
    };
    starmap::render(&mut off, virt, ctx, &state, &vis.stars, &vis.map_pops);
    for y in 0..view.height {
        for x in 0..view.width {
            let sx = map.x as i32 + m.map_pan.0 + x as i32;
            let sy = map.y as i32 + m.map_pan.1 + y as i32;
            if sx < 0 || sy < 0 || sx >= virt.width as i32 || sy >= virt.height as i32 {
                continue;
            }
            let src = off[(sx as u16, sy as u16)].clone();
            buf[(view.x + x, view.y + y)] = src;
        }
    }
    m.hits.add(view, Hit::Map);

    // Fiche du système choisi.
    let sheet = Rect::new(
        area.x,
        view.bottom(),
        area.width,
        area.bottom().saturating_sub(view.bottom()),
    );
    let w = sheet.width.saturating_sub(3) as usize;
    let head = format!(
        "Explorés {} · flottes {}/{}",
        snap.explored,
        snap.fleets.len(),
        snap.fleets_max
    );
    for x in sheet.x..sheet.right() {
        draw::put(buf, sheet, x as i32, sheet.y as i32, '─', theme::BORDER);
    }
    draw::text(
        buf,
        sheet.x + 2,
        sheet.y,
        &format!(" {head} "),
        theme::fg(theme::CYAN),
        sheet.right(),
    );
    let lines = clean(rgb_lines(starmap::detail_lines(ctx, m.map_sel, 400)), w);
    let max_lines = sheet.height.saturating_sub(BUTTON_H + 1) as usize;
    for (i, (l, s)) in lines.iter().take(max_lines).enumerate() {
        draw::text(
            buf,
            sheet.x + 1,
            sheet.y + 1 + i as u16,
            l,
            *s,
            sheet.right(),
        );
    }
    let by = sheet.bottom().saturating_sub(BUTTON_H);
    let mut buttons: Vec<(String, Rgb, Option<Hit>)> = Vec::new();
    let sel = snap.systems.get(m.map_sel);
    if m.map_sel == 0 && snap.convergence_ready {
        buttons.push(("Convergence".into(), theme::GOLD, Some(Hit::Convergence)));
    } else if let Some(s) = sel.filter(|s| s.index > 0) {
        let ok = s.available && snap.fleets.len() < snap.fleets_max;
        buttons.push((
            "Envoyer une flotte".into(),
            if ok { theme::GOLD } else { theme::GREY },
            ok.then_some(Hit::SendFleet),
        ));
    }
    if snap.auto_fleets_unlocked {
        let (l, c) = if snap.auto_fleets {
            ("Auto ●", theme::GREEN)
        } else {
            ("Auto ○", theme::GREY)
        };
        buttons.push((l.into(), c, Some(Hit::ToggleAutoFleets)));
    }
    let n = buttons.len() as u16;
    if n > 0 {
        let auto_w = if snap.auto_fleets_unlocked && n > 1 {
            11
        } else {
            0
        };
        let mut x = sheet.x + 1;
        for (i, (label, color, hit)) in buttons.iter().enumerate() {
            let width = if auto_w > 0 && i as u16 == n - 1 {
                auto_w
            } else {
                sheet.width.saturating_sub(2 + auto_w)
            };
            let r = Rect::new(x, by, width, BUTTON_H);
            button(buf, &mut m.hits, m.press, r, "", label, *color, *hit);
            x += width;
        }
    }
}

// ----------------------------------------------------------------------
// Reliques : 3 emplacements, puis les ensembles en grilles 2 × 2
// ----------------------------------------------------------------------

fn render_relics(buf: &mut Buffer, area: Rect, ctx: &Ctx, m: &mut MobileUi) {
    let snap = ctx.snap;
    let head = format!(
        "Équipées {}/{} · Éclats {} · production ×{}",
        snap.relics_equipped,
        snap.relic_slots,
        snap.shards,
        fmt::short(snap.relic_production)
    );
    let tile_h = 3u16;
    let set_h = 1 + 2 * tile_h + 1;
    let n_sets = ctx.catalog.relics.len().div_ceil(4) as u16;
    let total = 2 + n_sets * set_h;
    set_scroll_max(m, RELICS, total, area.height);
    let scroll = m.scroll[RELICS];
    if let Some(r) = place(area, scroll, 0, 2) {
        draw::text(
            buf,
            r.x + 1,
            r.y,
            &draw::truncate(&head, r.width as usize - 1),
            theme::bold(theme::CYAN),
            r.right(),
        );
    }
    let half = area.width.saturating_sub(3) / 2;
    for set in 0..n_sets {
        let vy = 2 + set * set_h;
        let first = set as usize * 4;
        let Some(def0) = ctx.catalog.relics.get(first) else {
            continue;
        };
        if let Some(r) = place(area, scroll, vy, 1) {
            let equipped = snap
                .relic_sets
                .iter()
                .find(|(s, _)| *s == def0.set)
                .map(|&(_, n)| n)
                .unwrap_or(0);
            let t = format!(
                "{} {} · {}/3",
                relics::set_symbol(def0.set),
                def0.set.name(),
                equipped.min(3)
            );
            draw::text(buf, r.x + 1, r.y, &t, theme::bold(theme::GOLD), r.right());
        }
        for k in 0..4usize {
            let i = first + k;
            let (Some(def), Some(v)) = (ctx.catalog.relics.get(i), snap.relics.get(i)) else {
                continue;
            };
            let ty = vy + 1 + (k / 2) as u16 * tile_h;
            let Some(row) = place(area, scroll, ty, tile_h) else {
                continue;
            };
            let x = row.x + 1 + (k % 2) as u16 * (half + 1);
            let r = Rect::new(x, row.y, half, tile_h);
            let (label, color) = match v.rarity {
                Some(rar) => (
                    format!("{}{}", if v.equipped { "● " } else { "" }, def.name),
                    relics::rarity_color(rar),
                ),
                None => ("???".to_string(), theme::GREY),
            };
            let border = if v.equipped {
                theme::GOLD
            } else {
                color.lerp(theme::BLACK, 0.3)
            };
            button(
                buf,
                &mut m.hits,
                m.press,
                r,
                "",
                &label,
                border,
                Some(Hit::Details(super::Detail::Relic(i))),
            );
        }
    }
    scrollbar(buf, area, scroll, m.scroll_max[RELICS]);
}

/// Bouton d'une fiche : libellé, couleur, action.
pub type SheetAction = (String, Rgb, Hit);

/// Lignes de la fiche d'une relique, et son bouton.
pub fn relic_sheet(
    ctx: &Ctx,
    i: usize,
    width: usize,
) -> (Vec<(String, Style)>, Option<SheetAction>) {
    let snap = ctx.snap;
    let lines = clean(relics::detail_lines(ctx, i, 400), width);
    let Some(v) = snap.relics.get(i) else {
        return (lines, None);
    };
    let act = if v.rarity.is_none() {
        None
    } else if v.equipped {
        Some(("Ranger".to_string(), theme::GREY, Hit::ToggleRelic(i)))
    } else if snap.relics_equipped < snap.relic_slots {
        Some(("Équiper".to_string(), theme::GOLD, Hit::ToggleRelic(i)))
    } else {
        None
    };
    (lines, act)
}

// ----------------------------------------------------------------------
// Mégastructures : la grille 9 × 5 pivotée en 5 × 9
// ----------------------------------------------------------------------

/// Case d'origine (index dans la grille du domaine) affichée en (colonne, ligne).
pub fn mega_cell(dc: usize, dr: usize) -> usize {
    // Pivot : les colonnes affichées sont les lignes du domaine.
    dc * GRID_W + dr
}

fn render_mega(buf: &mut Buffer, area: Rect, ctx: &Ctx, vis: &Visuals, m: &mut MobileUi) {
    let snap = ctx.snap;
    let now = ctx.now;
    let built = snap.grid.iter().filter(|c| c.is_some()).count();
    let head = format!(
        "◈ {} · structures {built}/{}",
        fmt::short(snap.exotic),
        GRID_W * GRID_H
    );
    draw::text(
        buf,
        area.x + 1,
        area.y,
        &head,
        theme::bold(theme::CYAN),
        area.right(),
    );
    let cols = GRID_H as u16;
    let cell_w = (area.width.saturating_sub(2) / cols).min(10);
    let cell_h = 3u16;
    let grid_w = cell_w * cols;
    let gx = area.x + (area.width.saturating_sub(grid_w)) / 2;
    let gy = area.y + 1;
    let sel = m.mega_sel.min(snap.grid.len().saturating_sub(1));
    let around: Vec<usize> = neighbors(sel).collect();
    let pop = vis.mega_pop;
    // La palette et la fiche gardent leur place ; la grille prend le reste.
    let bottom_h = BUTTON_H + 3;
    let max_rows = (area.bottom().saturating_sub(gy + bottom_h) / cell_h) as usize;
    let rows = GRID_W.min(max_rows.max(1));
    // Fenêtre verticale de la grille pivotée, qui suit la case choisie.
    let sel_row = sel % GRID_W;
    let first = sel_row.saturating_sub(rows / 2).min(GRID_W - rows);
    for dr in first..first + rows {
        for dc in 0..GRID_H {
            let cell = mega_cell(dc, dr);
            let r = Rect::new(
                gx + dc as u16 * cell_w,
                gy + (dr - first) as u16 * cell_h,
                cell_w,
                cell_h,
            );
            let kind = snap.grid.get(cell).copied().flatten();
            let mut color = match kind {
                Some(k) => mega::kind_color(k),
                None => theme::BAR_EMPTY,
            };
            if around.contains(&cell) {
                color = color.lerp(theme::GOLD, 0.5);
            }
            if cell == sel {
                color = theme::FOCUS;
                draw::fill_bg(buf, r, theme::SELECT_BG);
            }
            if let Some((t, c)) = pop {
                if c == cell && now - t < 0.6 {
                    color = color.glow(0.6 * (1.0 - (now - t) / 0.6));
                }
            }
            draw::card(buf, r, r, color);
            let label = kind.map(|k| k.symbol().to_string()).unwrap_or_default();
            let x = r.x + r.width.saturating_sub(1) / 2;
            draw::text(buf, x, r.y + 1, &label, theme::bold(color), r.right());
            m.hits.add(r, Hit::MegaCell(cell));
        }
    }
    let grid_bottom = gy + rows as u16 * cell_h;

    // Fiche de la case, puis la palette ou les actions.
    let info_w = area.width.saturating_sub(3) as usize;
    let lines = clean(mega::detail_lines(ctx, sel, 400), info_w);
    let info_y = grid_bottom;
    let by = area.bottom().saturating_sub(BUTTON_H);
    for (i, (l, s)) in lines
        .iter()
        .take(by.saturating_sub(info_y) as usize)
        .enumerate()
    {
        draw::text(buf, area.x + 1, info_y + i as u16, l, *s, area.right());
    }
    let kind = snap.grid.get(sel).copied().flatten();
    match kind {
        Some(k) => {
            let mut actions: Vec<(String, Rgb, Hit)> = Vec::new();
            if k == StructureKind::Core && snap.awakening_ready && snap.awakening.is_none() {
                actions.push((
                    "Éveil du Cosmos".into(),
                    theme::GOLD,
                    Hit::Ask(Confirm::Awakening),
                ));
            }
            actions.push((
                "Démolir".into(),
                theme::RED,
                Hit::Ask(Confirm::Demolish(sel)),
            ));
            let w = area.width.saturating_sub(2) / actions.len() as u16;
            for (i, (l, c, h)) in actions.iter().enumerate() {
                let r = Rect::new(area.x + 1 + i as u16 * w, by, w, BUTTON_H);
                button(buf, &mut m.hits, m.press, r, "", l, *c, Some(*h));
            }
        }
        None => {
            let n = StructureKind::ALL.len() as u16;
            let w = area.width.saturating_sub(2) / n;
            for (i, k) in StructureKind::ALL.iter().enumerate() {
                let cost = k.cost(built);
                let ok = snap.exotic >= cost;
                let c = if ok {
                    mega::kind_color(*k)
                } else {
                    theme::GREY
                };
                let r = Rect::new(area.x + 1 + i as u16 * w, by, w, BUTTON_H);
                button(
                    buf,
                    &mut m.hits,
                    m.press,
                    r,
                    &fmt::short(cost),
                    &k.symbol().to_string(),
                    c,
                    Some(Hit::Build(*k)),
                );
            }
        }
    }
}

// ----------------------------------------------------------------------
// Confirmation « glisser pour confirmer »
// ----------------------------------------------------------------------

fn confirm_text(ctx: &Ctx, c: Confirm) -> (String, Vec<(String, Rgb)>, &'static str, Rgb) {
    let snap = ctx.snap;
    match c {
        Confirm::Supernova => {
            let gained = snap.fragments_available;
            let new_bonus = domain::prestige::fragment_multiplier(snap.fragments + gained) - 1.0;
            (
                "✦ SUPERNOVA".into(),
                vec![
                    ("Faire exploser ta fonderie ?".into(), theme::TEXT),
                    (
                        format!("Fragments gagnés : +{}", fmt::short(gained)),
                        theme::PINK,
                    ),
                    (
                        format!(
                            "Bonus : +{} → +{}",
                            fmt::percent(snap.fragment_bonus),
                            fmt::percent(new_bonus)
                        ),
                        theme::CYAN,
                    ),
                    (
                        "Tout repart de zéro, sauf fragments, succès et stats.".into(),
                        theme::MUTED,
                    ),
                ],
                "glisser pour exploser",
                theme::PINK,
            )
        }
        Confirm::Crunch => {
            let next = snap.crunches + 1;
            let prod_after = snap.crunch_production_next;
            (
                "✶ BIG CRUNCH".into(),
                vec![
                    ("Replier tout l'univers sur lui-même ?".into(), theme::TEXT),
                    (
                        format!("Matière noire : {} → {next}", snap.crunches),
                        theme::PINK,
                    ),
                    (
                        format!(
                            "Production ×{} → ×{}",
                            fmt::short(snap.crunch_production),
                            fmt::short(prod_after)
                        ),
                        theme::CYAN,
                    ),
                    (
                        "Perdu : Supernovas, fragments, Atelier, Constellations, Défis.".into(),
                        theme::ORANGE,
                    ),
                    (
                        "Gardé : succès, statistiques, Récolte continue.".into(),
                        theme::GREEN,
                    ),
                ],
                "glisser pour replier",
                theme::PINK,
            )
        }
        Confirm::Challenge(i) => {
            let v = snap.challenges.get(i);
            let name = v
                .and_then(|v| ctx.catalog.challenge(v.id))
                .map(|d| d.name)
                .unwrap_or("?");
            if v.is_some_and(|v| v.active) {
                (
                    format!("◆ {name}"),
                    vec![(
                        "Abandonner ce Défi ? Le cycle continue, sans récompense.".into(),
                        theme::TEXT,
                    )],
                    "glisser pour abandonner",
                    theme::RED,
                )
            } else {
                (
                    format!("◆ {name}"),
                    vec![(
                        "Lancer ce Défi ? Il commence par une Supernova : le cycle repart de zéro avec sa contrainte.".into(),
                        theme::TEXT,
                    )],
                    "glisser pour lancer",
                    theme::PINK,
                )
            }
        }
        Confirm::BigBang => (
            "✷ BIG BANG".into(),
            vec![
                (
                    format!("Particules gagnées : +{}", snap.particles_available),
                    theme::GOLD,
                ),
                (
                    "Tout l'univers renaît : Big Crunch, Labo et univers repartent de zéro.".into(),
                    theme::MUTED,
                ),
            ],
            "glisser pour le Big Bang",
            theme::GOLD,
        ),
        Confirm::NewGame => (
            "Recommencer une partie".into(),
            vec![(
                "Toute la progression sera effacée, aussi sur tes appareils synchronisés (l'ancienne sauvegarde est gardée en .bak)."
                    .into(),
                theme::TEXT,
            )],
            "glisser pour tout effacer",
            theme::RED,
        ),
        Confirm::Demolish(cell) => {
            let name = snap
                .grid
                .get(cell)
                .copied()
                .flatten()
                .map(|k| k.name())
                .unwrap_or("?");
            (
                format!("Démolir : {name}"),
                vec![(
                    "Sans remboursement de la matière exotique.".into(),
                    theme::TEXT,
                )],
                "glisser pour démolir",
                theme::RED,
            )
        }
        Confirm::Awakening => (
            "✶ ÉVEIL DU COSMOS".into(),
            vec![(
                "72 h, même jeu fermé : la fin de l'Acte IV.".into(),
                theme::TEXT,
            )],
            "glisser pour éveiller",
            theme::GOLD,
        ),
    }
}

/// Rectangle du curseur de la confirmation (pour le suivi du doigt).
pub fn slider_rect(area: Rect) -> Rect {
    Rect::new(
        area.x + 1,
        area.bottom().saturating_sub(BUTTON_H + 1),
        area.width.saturating_sub(2),
        BUTTON_H,
    )
}

pub fn render_confirm(buf: &mut Buffer, area: Rect, ctx: &Ctx, m: &mut MobileUi, c: Confirming) {
    let (title, lines, hint, color) = confirm_text(ctx, c.what);
    let w = area.width.saturating_sub(4) as usize;
    let mut wrapped: Vec<(String, Rgb)> = Vec::new();
    for (l, col) in &lines {
        for x in draw::wrap(l, w) {
            wrapped.push((x, *col));
        }
    }
    let h = (wrapped.len() as u16 + 2 * BUTTON_H + 5).min(area.height);
    let r = Rect::new(area.x, area.bottom() - h, area.width, h);
    // Tout ce qui est dessous est inactif.
    m.hits.add(area, Hit::Block);
    // Apparition : la fiche monte du bas.
    let t = ((ctx.now - c.opened_at) / 0.2).clamp(0.0, 1.0);
    let rise = ((1.0 - crate::fx::easing::ease_out_cubic(t)) * h as f64) as u16;
    let r = Rect::new(r.x, r.y + rise, r.width, r.height.saturating_sub(rise));
    if r.height < 4 {
        return;
    }
    crate::fx::effects::fade(
        buf,
        Rect::new(area.x, area.y, area.width, r.y - area.y),
        0.5,
    );
    draw::clear(buf, r);
    draw::fill_bg(buf, r, theme::SELECT_BG);
    draw::card(buf, r, r, color);
    draw::text(
        buf,
        r.x + 2,
        r.y,
        &format!(" {title} "),
        theme::bold(theme::GOLD),
        r.right() - 1,
    );
    // Pendant la montée, la fiche est tronquée : rien sous son bord.
    for (i, (l, col)) in wrapped.iter().enumerate() {
        let y = r.y + 2 + i as u16;
        if y + 1 >= r.bottom() {
            break;
        }
        draw::text(buf, r.x + 2, y, l, theme::fg(*col), r.right() - 1);
    }
    if rise > 0 {
        return;
    }
    // Annuler, puis le curseur à faire glisser.
    let cancel = Rect::new(
        r.x + 1,
        r.bottom().saturating_sub(2 * BUTTON_H + 1),
        r.width.saturating_sub(2),
        BUTTON_H,
    );
    button(
        buf,
        &mut m.hits,
        m.press,
        cancel,
        "",
        "Annuler",
        theme::GREY,
        Some(Hit::CancelConfirm),
    );
    let track = slider_rect(r);
    draw::card(buf, track, track, color.lerp(theme::BLACK, 0.3));
    let inner_w = track.width.saturating_sub(2);
    let knob_w = 5u16.min(inner_w);
    let travel = inner_w.saturating_sub(knob_w);
    let kx = track.x + 1 + (c.slide.clamp(0.0, 1.0) * travel as f64).round() as u16;
    // Traînée derrière le bouton, puis le libellé.
    draw::fill_bg(
        buf,
        Rect::new(track.x + 1, track.y + 1, kx - track.x - 1 + knob_w, 1),
        color.lerp(theme::BLACK, 0.7),
    );
    let pulse = 0.5 + 0.5 * (ctx.now * 3.0).sin();
    let label = format!("{hint} ▸▸");
    let lx = track.x + 1 + inner_w.saturating_sub(label.chars().count() as u16) / 2;
    draw::text(
        buf,
        lx.max(kx + knob_w + 1),
        track.y + 1,
        &label,
        theme::fg(theme::MUTED.lerp(theme::TEXT, pulse * 0.5)),
        track.right() - 1,
    );
    draw::fill_bg(buf, Rect::new(kx, track.y + 1, knob_w, 1), color);
    draw::text(
        buf,
        kx + knob_w / 2,
        track.y + 1,
        "▶",
        theme::bold(theme::WHITE),
        track.right(),
    );
    m.hits.add(track, Hit::Slider);
}
