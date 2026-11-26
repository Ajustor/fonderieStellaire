//! Cosmogonie (touche G) : le Big Bang, la forge des générateurs primordiaux
//! et la construction du Grand Attracteur, en temps réel.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Widget};

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

/// Symbole des particules primordiales.
pub const PARTICLE: char = '✷';
const BORDER: Rgb = Rgb(255, 210, 110);

/// Ligne sélectionnable de la Cosmogonie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CosmosItem {
    BigBang,
    /// Index dans `snap.forges`.
    Forge(usize),
    /// Étape du Grand Attracteur.
    Stage(usize),
}

pub fn item_count(ctx: &Ctx) -> usize {
    1 + ctx.snap.forges.len() + ctx.catalog.attractor.len()
}

pub fn item_at(ctx: &Ctx, index: usize) -> Option<CosmosItem> {
    let forges = ctx.snap.forges.len();
    match index {
        0 => Some(CosmosItem::BigBang),
        i if i <= forges => Some(CosmosItem::Forge(i - 1)),
        i if i - 1 - forges < ctx.catalog.attractor.len() => {
            Some(CosmosItem::Stage(i - 1 - forges))
        }
        _ => None,
    }
}

enum Row {
    Header(&'static str),
    Item(usize),
}

fn plural(n: u32) -> &'static str {
    if n > 1 {
        "s"
    } else {
        ""
    }
}

/// (symbole, nom, statut, couleur, fait) d'une ligne.
fn describe(ctx: &Ctx, item: CosmosItem) -> (char, String, String, Rgb, bool) {
    let snap = ctx.snap;
    match item {
        CosmosItem::BigBang => {
            let (st, color) = if snap.bigbang_available {
                (
                    format!("+{} {PARTICLE}", snap.particles_available),
                    theme::GOLD,
                )
            } else {
                (
                    format!("{}/{} Big Crunch", snap.crunches, snap.bigbang_needed),
                    theme::GREY,
                )
            };
            (PARTICLE, "Big Bang".into(), st, color, false)
        }
        CosmosItem::Forge(i) => {
            let Some(f) = snap.forges.get(i) else {
                return (' ', String::new(), String::new(), theme::GREY, false);
            };
            let Some(def) = ctx.catalog.generator(f.id) else {
                return (' ', String::new(), String::new(), theme::GREY, false);
            };
            let (st, color) = if f.forged {
                ("● forgé".to_string(), theme::GREEN)
            } else if f.available {
                (format!("{} {PARTICLE}", def.particle_cost), theme::GOLD)
            } else {
                (format!("{} {PARTICLE}", def.particle_cost), theme::GREY)
            };
            (def.symbol, def.name.to_string(), st, color, f.forged)
        }
        CosmosItem::Stage(i) => {
            let Some(stage) = ctx.catalog.attractor_stage(i) else {
                return (' ', String::new(), String::new(), theme::GREY, false);
            };
            let done = i < snap.attractor_stages;
            let building = snap.attractor_build.filter(|b| b.stage == i);
            let (st, color) = if done {
                ("● achevée".to_string(), theme::GREEN)
            } else if let Some(b) = building {
                (
                    format!("construction · {}", fmt::duration(b.remaining)),
                    theme::CYAN,
                )
            } else {
                let color = if i == snap.attractor_stages && snap.attractor_buildable {
                    theme::GOLD
                } else {
                    theme::GREY
                };
                (
                    format!(
                        "{} {PARTICLE} · {}",
                        stage.cost,
                        fmt::duration(stage.build_secs)
                    ),
                    color,
                )
            };
            let symbol = if done { '◆' } else { '◇' };
            (symbol, stage.name.to_string(), st, color, done)
        }
    }
}

/// Explication et action de la ligne sélectionnée.
fn detail(ctx: &Ctx, item: CosmosItem, armed: bool) -> Vec<(String, Rgb)> {
    let snap = ctx.snap;
    let mut out = Vec::new();
    match item {
        CosmosItem::BigBang => {
            out.push((
                format!(
                    "Remet à zéro les Big Crunch et tout le niveau Supernova contre des particules \
                     primordiales {PARTICLE} : +100 % de production et +25 % de fragments chacune, \
                     pour toujours. Laboratoire, Clés d'univers, forges et Grand Attracteur sont conservés."
                ),
                theme::CYAN,
            ));
            out.push(if snap.bigbang_available && armed {
                (
                    "Entrée de nouveau pour déclencher le BIG BANG !".to_string(),
                    theme::ORANGE,
                )
            } else if snap.bigbang_available {
                (
                    format!(
                        "[Entrée] déclencher : +{} particule{} (une par tranche de 10 Big Crunch).",
                        snap.particles_available,
                        plural(snap.particles_available)
                    ),
                    theme::GOLD,
                )
            } else {
                (
                    format!(
                        "Il faut {} Big Crunch ({}/{}) : une particule par tranche de 10.",
                        snap.bigbang_needed, snap.crunches, snap.bigbang_needed
                    ),
                    theme::GREY,
                )
            });
        }
        CosmosItem::Forge(i) => {
            let Some((f, def)) = snap
                .forges
                .get(i)
                .and_then(|f| ctx.catalog.generator(f.id).map(|d| (f, d)))
            else {
                return out;
            };
            out.push((def.description.to_string(), theme::CYAN));
            out.push((
                format!(
                    "{}/s par unité · premier exemplaire à {} ✦ · apparaît dans tous les cycles",
                    fmt::short(def.base_prod),
                    fmt::short(def.base_cost)
                ),
                theme::MUTED,
            ));
            let previous_missing = snap.forges[..i].iter().any(|p| !p.forged);
            out.push(if f.forged {
                (
                    "Forgé : il fait partie de ta fonderie pour toujours.".to_string(),
                    theme::GREEN,
                )
            } else if previous_missing {
                (
                    "Forge d'abord le générateur précédent.".to_string(),
                    theme::GREY,
                )
            } else if f.available {
                (
                    format!(
                        "[Entrée] forger : {} particule{} {PARTICLE}",
                        def.particle_cost,
                        plural(def.particle_cost)
                    ),
                    theme::GOLD,
                )
            } else {
                (
                    format!(
                        "Il faut {} particule{} {PARTICLE} libre{} : déclenche un Big Bang.",
                        def.particle_cost,
                        plural(def.particle_cost),
                        plural(def.particle_cost)
                    ),
                    theme::GREY,
                )
            });
        }
        CosmosItem::Stage(i) => {
            let Some(stage) = ctx.catalog.attractor_stage(i) else {
                return out;
            };
            out.push((
                format!("Étape {} : {}", i + 1, stage.description),
                theme::CYAN,
            ));
            let building = snap.attractor_build.filter(|b| b.stage == i);
            out.push(if i < snap.attractor_stages {
                ("Étape achevée : son effet est permanent.".to_string(), theme::GREEN)
            } else if let Some(b) = building {
                (
                    format!(
                        "En construction : encore {} (avance même jeu fermé).",
                        fmt::duration(b.remaining)
                    ),
                    theme::CYAN,
                )
            } else if i > snap.attractor_stages {
                (
                    "Construis d'abord les étapes précédentes.".to_string(),
                    theme::GREY,
                )
            } else if snap.attractor_build.is_some() {
                ("Une étape à la fois.".to_string(), theme::GREY)
            } else if snap.big_bangs == 0 {
                (
                    "Le chantier s'ouvre après ton premier Big Bang.".to_string(),
                    theme::GREY,
                )
            } else if snap.attractor_buildable {
                (
                    format!(
                        "[Entrée] lancer : {} {PARTICLE}, {} de construction (avance même jeu fermé).",
                        stage.cost,
                        fmt::duration(stage.build_secs)
                    ),
                    theme::GOLD,
                )
            } else {
                (
                    format!(
                        "Il faut {} particule{} {PARTICLE} libre{}.",
                        stage.cost,
                        plural(stage.cost),
                        plural(stage.cost)
                    ),
                    theme::GREY,
                )
            });
        }
    }
    out
}

fn wrapped(lines: &[(String, Rgb)], width: usize) -> Vec<(String, Rgb)> {
    lines
        .iter()
        .flat_map(|(t, c)| draw::wrap(t, width).into_iter().map(move |l| (l, *c)))
        .collect()
}

/// En-tête : particules, bonus, avancement du Grand Attracteur.
fn head_lines(ctx: &Ctx, width: usize) -> Vec<(String, Rgb)> {
    let snap = ctx.snap;
    let mut lines = vec![(
        format!(
            "Particules {PARTICLE} libres : {} sur {}   ·   production ×{}   ·   fragments +{}",
            snap.particles_spendable,
            snap.particles,
            fmt::short(snap.particle_production),
            fmt::percent(snap.particle_fragments - 1.0)
        ),
        theme::CYAN,
    )];
    let total = ctx.catalog.attractor.len();
    let progress = match snap.attractor_build {
        Some(b) => {
            let name = ctx
                .catalog
                .attractor_stage(b.stage)
                .map(|s| s.name)
                .unwrap_or("?");
            let p = if b.total > 0.0 {
                1.0 - b.remaining / b.total
            } else {
                1.0
            };
            let filled = (p.clamp(0.0, 1.0) * 16.0).round() as usize;
            format!(
                "Grand Attracteur {}/{total} · en chantier : {name} {}{} encore {}",
                snap.attractor_stages,
                "█".repeat(filled),
                "░".repeat(16 - filled),
                fmt::duration(b.remaining)
            )
        }
        None if snap.attractor_awakened => {
            "Grand Attracteur éveillé : l'Acte II est terminé, ta fonderie continue.".to_string()
        }
        None => format!(
            "Grand Attracteur {}/{total} · production ×{}",
            snap.attractor_stages,
            fmt::short(snap.attractor_production)
        ),
    };
    lines.push((progress, Rgb(190, 110, 255).glow(0.25)));
    wrapped(&lines, width)
}

pub fn render(
    buf: &mut Buffer,
    area: Rect,
    ctx: &Ctx,
    popup: &Popup,
    selected: usize,
    armed: bool,
) {
    let s = ease_out_cubic(popup.scale(ctx.now));
    if s <= 0.0 {
        return;
    }
    let snap = ctx.snap;
    effects::fade(buf, area, 0.65 * s);

    let width = area.width.saturating_sub(4).min(92);
    let text_w = width.saturating_sub(4) as usize;
    let count = item_count(ctx);
    let selected = selected.min(count.saturating_sub(1));
    let head = head_lines(ctx, text_w);
    // Détail de hauteur fixe (le plus long), pour une fenêtre qui ne saute pas.
    let detail_h = (0..count)
        .filter_map(|i| item_at(ctx, i))
        .map(|it| wrapped(&detail(ctx, it, armed), text_w).len())
        .max()
        .unwrap_or(0) as u16;

    let mut rows = vec![Row::Header("BIG BANG"), Row::Item(0)];
    rows.push(Row::Header(
        "FORGE PRIMORDIALE · des générateurs pour toujours",
    ));
    rows.extend((0..snap.forges.len()).map(|i| Row::Item(1 + i)));
    rows.push(Row::Header(
        "GRAND ATTRACTEUR · une étape à la fois, en temps réel",
    ));
    rows.extend((0..ctx.catalog.attractor.len()).map(|i| Row::Item(1 + snap.forges.len() + i)));

    // bordures + en-tête + blanc + liste + séparateur + détail + pied
    let chrome = 2 + head.len() as u16 + 1 + 1 + detail_h + 1;
    let height = (rows.len() as u16 + chrome).min(area.height.saturating_sub(2));
    let w = ((width as f64 * s).round() as u16).max(2);
    let h = ((height as f64 * s).round() as u16).max(2);
    let rect = centered(area, w, h);
    Clear.render(rect, buf);
    let pulse = 0.5 + 0.5 * (ctx.now * 1.8).sin();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::bold(BORDER.lerp(Rgb(190, 110, 255), pulse)))
        .title(Line::from(Span::styled(" ✷ COSMOGONIE ✷ ", theme::bold(theme::GOLD))).centered());
    let inner = block.inner(rect);
    block.render(rect, buf);
    if s < 0.95 || inner.height < 8 {
        return;
    }
    let right = inner.right();
    for (i, (l, c)) in head.iter().enumerate() {
        draw::text(
            buf,
            inner.x + 1,
            inner.y + i as u16,
            l,
            theme::bold(*c),
            right,
        );
    }

    let sel_row = rows
        .iter()
        .position(|r| matches!(r, Row::Item(p) if *p == selected))
        .unwrap_or(0);
    let list_y = inner.y + head.len() as u16 + 1;
    let list_rows = inner.height.saturating_sub(chrome - 2) as usize;
    let scroll = (sel_row + 1).saturating_sub(list_rows);
    for (n, row) in rows.iter().enumerate().skip(scroll).take(list_rows) {
        let y = list_y + (n - scroll) as u16;
        match row {
            Row::Header(title) => {
                draw::text(buf, inner.x + 1, y, title, theme::bold(theme::MUTED), right);
            }
            Row::Item(index) => {
                let Some(item) = item_at(ctx, *index) else {
                    continue;
                };
                let is_sel = *index == selected;
                if is_sel {
                    draw::fill_bg(buf, Rect::new(inner.x, y, inner.width, 1), theme::SELECT_BG);
                }
                let (symbol, name, st, color, done) = describe(ctx, item);
                let mut x = draw::text(
                    buf,
                    inner.x + 1,
                    y,
                    if is_sel { "▶ " } else { "  " },
                    theme::bold(theme::FOCUS),
                    right,
                );
                // Le Big Bang disponible brille de toutes les couleurs.
                let sym_color = if item == CosmosItem::BigBang && snap.bigbang_available {
                    gradient_cyclic(&SPECTRUM, ctx.now * 0.5)
                } else {
                    color
                };
                x = draw::text(
                    buf,
                    x,
                    y,
                    &format!("{symbol} "),
                    theme::bold(sym_color),
                    right,
                );
                let name_style = if done {
                    theme::fg(theme::GREEN)
                } else {
                    theme::bold(theme::TEXT)
                };
                let st_x = right.saturating_sub(1 + st.chars().count() as u16);
                draw::text(buf, x, y, &name, name_style, st_x.saturating_sub(1));
                draw::text_right(buf, right - 1, y, &st, theme::bold(color), inner.x);
            }
        }
    }

    // Détail de la sélection.
    let sep_y = inner.bottom() - 2 - detail_h;
    let sep: String = "─".repeat(inner.width.saturating_sub(2) as usize);
    draw::text(
        buf,
        inner.x + 1,
        sep_y,
        &sep,
        theme::fg(theme::BORDER),
        right,
    );
    if let Some(item) = item_at(ctx, selected) {
        for (i, (l, c)) in wrapped(&detail(ctx, item, armed), text_w)
            .iter()
            .enumerate()
        {
            let style: Style = theme::fg(*c);
            draw::text(buf, inner.x + 1, sep_y + 1 + i as u16, l, style, right);
        }
    }

    let long = "[↑↓] choisir   [Entrée] déclencher / forger / construire   [G] / [Échap] fermer";
    let footer = if long.chars().count() <= inner.width as usize {
        long
    } else {
        "[↑↓] choisir  [Entrée] valider  [Échap] fermer"
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
