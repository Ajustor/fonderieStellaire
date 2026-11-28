//! Laboratoire de matière noire (touche L) : recherches en temps réel, une à la
//! fois, qui avancent même jeu fermé.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Widget};

use application::{GameSnapshot, LabView};
use domain::LabEffect;

use super::Ctx;
use crate::draw;
use crate::fmt;
use crate::fx::Rgb;
use crate::theme;
use crate::ui::Popup;

/// Sections du Laboratoire, dans l'ordre d'affichage.
/// Une section : son titre et le test d'appartenance d'une recherche.
pub(crate) type Section = (&'static str, fn(LabEffect) -> bool);

pub(crate) const SECTIONS: [Section; 3] = [
    ("AUTOMATISMES · activables / en pause avec Entrée", |e| {
        e.is_toggleable()
    }),
    ("CONSERVATION · ce qui survit au Big Crunch", |e| {
        matches!(
            e,
            LabEffect::KeepAtelier | LabEffect::KeepChallenges | LabEffect::HeadStart(_)
        )
    }),
    ("PUISSANCE", |e| {
        !e.is_toggleable()
            && !matches!(
                e,
                LabEffect::KeepAtelier | LabEffect::KeepChallenges | LabEffect::HeadStart(_)
            )
    }),
];

/// Ordre d'affichage des recherches (index dans `snap.lab`), section par section.
pub fn display_order(ctx: &Ctx) -> Vec<usize> {
    let mut order = Vec::with_capacity(ctx.snap.lab.len());
    for (_, belongs) in SECTIONS {
        for (i, v) in ctx.snap.lab.iter().enumerate() {
            if ctx
                .catalog
                .lab_node(v.id)
                .is_some_and(|d| belongs(d.effect))
            {
                order.push(i);
            }
        }
    }
    order
}

/// Recherche affichée à la position `selected`.
pub fn item_at(ctx: &Ctx, selected: usize) -> Option<LabView> {
    display_order(ctx)
        .get(selected)
        .and_then(|&i| ctx.snap.lab.get(i).copied())
}

pub(crate) fn status(
    snap: &GameSnapshot,
    v: &LabView,
    cost: u32,
    secs: f64,
    toggleable: bool,
) -> (String, Rgb) {
    if let Some(r) = snap.research.filter(|r| r.id == v.id) {
        (
            format!("recherche · {}", fmt::duration(r.remaining)),
            theme::CYAN,
        )
    } else if v.done && toggleable && v.paused {
        ("○ en pause".into(), theme::GREY)
    } else if v.done && toggleable {
        ("● actif".into(), theme::GREEN)
    } else if v.done {
        ("● acquis".into(), theme::GREEN)
    } else if v.blocked {
        ("après la précédente".into(), theme::GREY)
    } else {
        let color = if v.available {
            theme::GOLD
        } else {
            theme::GREY
        };
        (format!("{cost} ✶ · {}", fmt::duration(secs)), color)
    }
}

enum Row {
    Header(&'static str),
    Item(usize),
}

pub fn render(buf: &mut Buffer, area: Rect, ctx: &Ctx, popup: &Popup, selected: usize) {
    let snap = ctx.snap;

    let order = display_order(ctx);
    let mut rows = Vec::with_capacity(order.len() + SECTIONS.len());
    for (title, belongs) in SECTIONS {
        rows.push(Row::Header(title));
        for (pos, &i) in order.iter().enumerate() {
            let in_section = snap
                .lab
                .get(i)
                .and_then(|v| ctx.catalog.lab_node(v.id))
                .is_some_and(|d| belongs(d.effect));
            if in_section {
                rows.push(Row::Item(pos));
            }
        }
    }

    let width = ctx.frame.width(area, 92);
    // Texte à une largeur de lecture raisonnable, même en onglet pleine largeur.
    let text_w = width.min(92).saturating_sub(4) as usize;
    let head = head_lines(ctx, text_w);
    let detail_h = (0..order.len())
        .map(|i| detail_lines(ctx, &order, i, text_w).len())
        .max()
        .unwrap_or(0) as u16;
    // bordures + en-tête + blanc + liste + séparateur + détail + pied
    let chrome = 2 + head.len() as u16 + 1 + 1 + detail_h + 1;
    let height = ctx.frame.height(area, rows.len() as u16 + chrome);
    let Some((rect, ready)) = ctx.frame.place(buf, area, ctx.now, popup, width, height) else {
        return;
    };
    let violet = Rgb(170, 90, 255);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::bold(violet))
        .title(
            Line::from(Span::styled(
                " ✶ LABORATOIRE DE MATIÈRE NOIRE ✶ ",
                theme::bold(theme::GOLD),
            ))
            .centered(),
        );
    let inner = block.inner(rect);
    block.render(rect, buf);
    if !ready || inner.height < 8 {
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

    let selected = selected.min(order.len().saturating_sub(1));
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
            Row::Item(pos) => {
                let Some(v) = order.get(*pos).and_then(|&i| snap.lab.get(i)) else {
                    continue;
                };
                let Some(def) = ctx.catalog.lab_node(v.id) else {
                    continue;
                };
                let is_sel = *pos == selected;
                if is_sel {
                    draw::fill_bg(buf, Rect::new(inner.x, y, inner.width, 1), theme::SELECT_BG);
                }
                let (st, color) = status(
                    snap,
                    v,
                    def.cost,
                    def.research_secs,
                    def.effect.is_toggleable(),
                );
                let mut x = draw::text(
                    buf,
                    inner.x + 1,
                    y,
                    if is_sel { "▶ " } else { "  " },
                    theme::bold(theme::FOCUS),
                    right,
                );
                x = draw::text(
                    buf,
                    x,
                    y,
                    &format!("{} ", def.symbol),
                    theme::bold(color),
                    right,
                );
                let name_style = if v.done {
                    theme::fg(theme::GREEN)
                } else {
                    theme::bold(theme::TEXT)
                };
                let st_x = right.saturating_sub(1 + st.chars().count() as u16);
                draw::text(buf, x, y, def.name, name_style, st_x.saturating_sub(1));
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
    for (i, (l, style)) in detail_lines(ctx, &order, selected, text_w)
        .iter()
        .enumerate()
    {
        draw::text(buf, inner.x + 1, sep_y + 1 + i as u16, l, *style, right);
    }

    let long = "[↑↓] choisir   [Entrée] lancer / activer   [L] / [Échap] fermer";
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

/// En-tête : matière noire et recherche en cours, renvoyées à la ligne.
pub(crate) fn head_lines(ctx: &Ctx, width: usize) -> Vec<(String, Rgb)> {
    let snap = ctx.snap;
    let violet = Rgb(170, 90, 255);
    let head = format!(
        "Matière noire libre : {} sur {}   ·   bonus ×{} conservé après achat",
        snap.dark_matter_spendable,
        snap.crunches,
        fmt::short(snap.crunch_production)
    );
    let research_line = match snap.research {
        Some(r) => {
            let name = ctx.catalog.lab_node(r.id).map(|d| d.name).unwrap_or("?");
            let p = if r.total > 0.0 {
                1.0 - r.remaining / r.total
            } else {
                1.0
            };
            let bar_w = 20usize;
            let filled = (p.clamp(0.0, 1.0) * bar_w as f64).round() as usize;
            format!(
                "En cours : {name} {}{} encore {} (avance même jeu fermé)",
                "█".repeat(filled),
                "░".repeat(bar_w - filled),
                fmt::duration(r.remaining)
            )
        }
        None => "Aucune recherche en cours : choisis-en une.".to_string(),
    };
    let mut out: Vec<(String, Rgb)> = draw::wrap(&head, width)
        .into_iter()
        .map(|l| (l, theme::CYAN))
        .collect();
    out.extend(
        draw::wrap(&research_line, width)
            .into_iter()
            .map(|l| (l, violet.glow(0.3))),
    );
    out
}

/// Détail de la recherche n° `selected` (description et action), renvoyé à la ligne.
pub(crate) fn detail_lines(
    ctx: &Ctx,
    order: &[usize],
    selected: usize,
    width: usize,
) -> Vec<(String, ratatui::style::Style)> {
    let snap = ctx.snap;
    let mut out = Vec::new();
    let Some(v) = order.get(selected).and_then(|&i| snap.lab.get(i)) else {
        return out;
    };
    let Some(def) = ctx.catalog.lab_node(v.id) else {
        return out;
    };
    for l in draw::wrap(def.description, width) {
        out.push((l, theme::fg(theme::CYAN)));
    }
    let researching = snap.research.is_some_and(|r| r.id == v.id);
    let (hint, color) = if researching {
        ("Recherche en cours…".to_string(), theme::CYAN)
    } else if v.done && def.effect.is_toggleable() {
        let what = if v.paused {
            "relancer"
        } else {
            "mettre en pause"
        };
        (format!("[Entrée] {what}"), theme::MUTED)
    } else if v.done {
        (
            "Recherche terminée : effet permanent.".to_string(),
            theme::GREEN,
        )
    } else if v.blocked {
        let prereq = def
            .requires
            .and_then(|k| ctx.catalog.lab_by_key(k))
            .and_then(|id| ctx.catalog.lab_node(id))
            .map(|d| d.name)
            .unwrap_or("?");
        (format!("Termine d'abord « {prereq} »."), theme::GREY)
    } else if snap.research.is_some() {
        (
            "Une recherche à la fois : attends la fin de la recherche en cours.".to_string(),
            theme::GREY,
        )
    } else if v.available {
        (
            format!(
                "[Entrée] lancer : {} ✶, {} de recherche",
                def.cost,
                fmt::duration(def.research_secs)
            ),
            theme::GOLD,
        )
    } else {
        (
            format!(
                "Il faut {} ✶ de matière noire libre : déclenche des Big Crunch [K].",
                def.cost
            ),
            theme::GREY,
        )
    };
    for l in draw::wrap(&hint, width) {
        out.push((l, theme::bold(color)));
    }
    out
}
