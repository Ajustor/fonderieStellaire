//! Reliques (touche H, Acte III) : la collection par ensemble, les
//! emplacements, les bonus d'ensemble ; et la carte qui se retourne quand une
//! relique est rapportée d'expédition.

use std::f64::consts::TAU;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Widget};

use domain::relic::{effect_text, RARITY_NAMES};
use domain::{RelicId, RelicOutcome, RelicSet};

use super::Ctx;
use crate::draw;
use crate::fx::color::gradient_cyclic;
use crate::fx::easing::{ease_in_cubic, ease_out_back, ease_out_cubic};
use crate::fx::effects;
use crate::fx::Rgb;
use crate::layout::centered;
use crate::screens::bigbang::SPECTRUM;
use crate::theme;
use crate::ui::Popup;

/// Couleur de chaque rareté.
pub const RARITY_COLORS: [Rgb; 4] = [
    Rgb(205, 205, 215),
    Rgb(110, 170, 255),
    Rgb(190, 110, 255),
    Rgb(255, 205, 90),
];
/// Durée d'affichage de la carte d'une relique trouvée.
pub const CARD_DURATION: f64 = 3.2;

pub fn set_symbol(set: RelicSet) -> char {
    match set {
        RelicSet::Forge => '▲',
        RelicSet::Voyager => '▸',
        RelicSet::Eclipse => '●',
        RelicSet::Origins => '✶',
    }
}

pub fn rarity_color(r: u8) -> Rgb {
    RARITY_COLORS[(r as usize).min(3)]
}

/// Lignes du détail de la relique sélectionnée.
pub(crate) fn detail_lines(ctx: &Ctx, index: usize, width: usize) -> Vec<(String, Style)> {
    let snap = ctx.snap;
    let mut out = Vec::new();
    let (Some(def), Some(v)) = (ctx.catalog.relics.get(index), snap.relics.get(index)) else {
        return out;
    };
    let mut push = |t: String, s: Style| {
        for l in draw::wrap(&t, width) {
            out.push((l, s));
        }
    };
    let equipped_in_set = snap
        .relic_sets
        .iter()
        .find(|(s, _)| *s == def.set)
        .map(|&(_, n)| n)
        .unwrap_or(0);
    let set_line = format!(
        "Ensemble {} ({}/3 équipées) : {}",
        def.set.name(),
        equipped_in_set.min(3),
        def.set.bonus_text()
    );
    match v.rarity {
        Some(r) => {
            push(
                format!(
                    "{} {} · {}",
                    set_symbol(def.set),
                    def.name,
                    RARITY_NAMES[r as usize]
                ),
                theme::bold(rarity_color(r)),
            );
            push(effect_text(def.effect_at(r)), theme::fg(theme::CYAN));
            push(def.flavor.to_string(), theme::fg(theme::MUTED));
            push(set_line, theme::fg(theme::TEXT));
            let hint = if v.equipped {
                ("[Entrée] ranger".to_string(), theme::MUTED)
            } else if snap.relics_equipped >= snap.relic_slots {
                (
                    "Emplacements pleins : range d'abord une relique.".to_string(),
                    theme::GREY,
                )
            } else {
                ("[Entrée] équiper".to_string(), theme::GOLD)
            };
            push(hint.0, theme::bold(hint.1));
        }
        None => {
            push(
                format!("{} ??? · ensemble {}", set_symbol(def.set), def.set.name()),
                theme::bold(theme::GREY),
            );
            push(
                "Pas encore trouvée : les expéditions réussies en rapportent une fois sur deux, les prospections (vers un système déjà exploré) à chaque fois.".into(),
                theme::fg(theme::MUTED),
            );
            push(set_line, theme::fg(theme::TEXT));
        }
    }
    out
}

pub fn render(buf: &mut Buffer, area: Rect, ctx: &Ctx, popup: &Popup, selected: usize) {
    let s = ease_out_cubic(popup.scale(ctx.now));
    if s <= 0.0 {
        return;
    }
    let snap = ctx.snap;
    effects::fade(buf, area, 0.65 * s);
    let width = area.width.saturating_sub(4).min(100);
    let text_w = width.saturating_sub(4) as usize;
    let count = ctx.catalog.relics.len();
    let selected = selected.min(count.saturating_sub(1));

    // En-tête : emplacements, Éclats, ensembles actifs.
    let mut head: Vec<(String, Style)> = draw::wrap(
        &format!(
            "Emplacements : {}/{} · Éclats : {} (+{}% de production) · production des reliques ×{}",
            snap.relics_equipped,
            snap.relic_slots,
            snap.shards,
            snap.shards,
            crate::fmt::short(snap.relic_production)
        ),
        text_w,
    )
    .into_iter()
    .map(|l| (l, theme::bold(theme::CYAN)))
    .collect();
    let active: Vec<String> = snap
        .relic_sets
        .iter()
        .filter(|(_, n)| *n >= 3)
        .map(|(s, _)| format!("{} ({})", s.name(), s.bonus_text()))
        .collect();
    let sets = if active.is_empty() {
        "Aucun ensemble actif : équipe 3 reliques du même ensemble.".to_string()
    } else {
        format!("Ensembles actifs : {}", active.join(" · "))
    };
    head.extend(
        draw::wrap(&sets, text_w)
            .into_iter()
            .map(|l| (l, theme::fg(theme::GOLD))),
    );

    let detail_h = (0..count)
        .map(|i| detail_lines(ctx, i, text_w).len())
        .max()
        .unwrap_or(0) as u16;
    // bordures + en-tête + blanc + titres + 4 lignes + blanc + séparateur + détail + pied
    let height = (2 + head.len() as u16 + 1 + 1 + 4 + 1 + 1 + detail_h + 1)
        .min(area.height.saturating_sub(2));
    let w = ((width as f64 * s).round() as u16).max(2);
    let h = ((height as f64 * s).round() as u16).max(2);
    let rect = centered(area, w, h);
    Clear.render(rect, buf);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::bold(
            RARITY_COLORS[2].lerp(RARITY_COLORS[3], 0.5 + 0.5 * (ctx.now * 1.5).sin()),
        ))
        .title(Line::from(Span::styled(" ◆ RELIQUES ◆ ", theme::bold(theme::GOLD))).centered());
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
    y += 1;

    // Grille : une colonne par ensemble.
    let col_w = inner.width.saturating_sub(2) / 4;
    for (c, set) in RelicSet::ALL.iter().enumerate() {
        let x = inner.x + 1 + c as u16 * col_w;
        let n = snap
            .relic_sets
            .iter()
            .find(|(s, _)| s == set)
            .map(|&(_, n)| n)
            .unwrap_or(0);
        let title_color = if n >= 3 { theme::GOLD } else { theme::MUTED };
        let title = format!("{} {} {}/3", set_symbol(*set), set.name(), n.min(3));
        draw::text(buf, x, y, &title, theme::bold(title_color), x + col_w);
        for (row, def) in ctx
            .catalog
            .relics
            .iter()
            .filter(|r| r.set == *set)
            .enumerate()
        {
            let ry = y + 1 + row as u16;
            let Some(v) = snap.relics.get(def.id.index()) else {
                continue;
            };
            let is_sel = def.id.index() == selected;
            if is_sel {
                draw::fill_bg(buf, Rect::new(x, ry, col_w, 1), theme::SELECT_BG);
            }
            let (mark, name, color) = match v.rarity {
                Some(r) if v.equipped => ('◆', def.name, rarity_color(r)),
                Some(r) => ('◇', def.name, rarity_color(r)),
                None => ('·', "???", theme::GREY),
            };
            let cursor = if is_sel { "▶" } else { " " };
            let label = format!("{cursor}{mark} {name}");
            // Le nom complet est toujours dans le détail de la sélection.
            let label = draw::fit(&label, col_w.saturating_sub(1) as usize);
            let style = if v.equipped {
                theme::bold(color)
            } else {
                theme::fg(color)
            };
            draw::text(buf, x, ry, &label, style, x + col_w);
        }
    }
    y += 6;

    let sep: String = "─".repeat(inner.width.saturating_sub(2) as usize);
    draw::text(buf, inner.x + 1, y, &sep, theme::fg(theme::BORDER), right);
    y += 1;
    for (l, st) in detail_lines(ctx, selected, text_w) {
        if y >= inner.bottom().saturating_sub(1) {
            draw::audit::record(&l, 0);
            continue;
        }
        draw::text(buf, inner.x + 1, y, &l, st, right);
        y += 1;
    }
    let long = "[←→↑↓] choisir   [Entrée] équiper / ranger   [H] / [Échap] fermer";
    let footer = if long.chars().count() <= inner.width as usize {
        long
    } else {
        "[←→↑↓] [Entrée] [Échap]"
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

/// Sélection voisine dans la grille (colonnes = ensembles, 4 lignes).
pub fn navigate(selected: usize, dx: i32, dy: i32) -> usize {
    let col = (selected / 4) as i32;
    let row = (selected % 4) as i32;
    let col = (col + dx).clamp(0, 3);
    let row = (row + dy).clamp(0, 3);
    (col * 4 + row) as usize
}

/// Une relique trouvée : une carte qui se retourne, brille et repart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RelicCard {
    pub start: f64,
    pub relic: RelicId,
    pub rarity: u8,
    pub outcome: RelicOutcome,
}

pub fn render_card(buf: &mut Buffer, area: Rect, ctx: &Ctx, card: &RelicCard) {
    let t = ctx.now - card.start;
    if !(0.0..CARD_DURATION).contains(&t) {
        return;
    }
    let Some(def) = ctx.catalog.relic(card.relic) else {
        return;
    };
    let color = rarity_color(card.rarity);
    let legendary = card.rarity >= 3;
    // Entrée : la carte se retourne (sa largeur s'ouvre) ; sortie : elle s'efface.
    let open = ease_out_back((t / 0.45).clamp(0.0, 1.0));
    let fade = if t > CARD_DURATION - 0.5 {
        ease_in_cubic((t - (CARD_DURATION - 0.5)) / 0.5)
    } else {
        0.0
    };
    effects::fade(buf, area, 0.45 * (1.0 - fade));
    let full_w = 38u16.min(area.width.saturating_sub(4));
    let w = ((full_w as f64 * open).round() as u16).clamp(2, full_w);
    let h = 9u16;
    let rect = centered(area, w, h);
    draw::clear(buf, rect);
    // Bordure : couleur de la rareté, arc-en-ciel pour une légendaire.
    for i in 0..(2 * (w + h)) as usize {
        let (x, y) = perimeter(rect, i);
        let c = if legendary {
            gradient_cyclic(&SPECTRUM, i as f64 / 24.0 - ctx.now * 0.8)
        } else {
            color.glow(0.25 * (0.5 + 0.5 * (ctx.now * 4.0 + i as f64 * 0.3).sin()))
        };
        let ch =
            if (x == rect.x || x + 1 == rect.right()) && (y == rect.y || y + 1 == rect.bottom()) {
                '◆'
            } else if y == rect.y || y + 1 == rect.bottom() {
                '─'
            } else {
                '│'
            };
        draw::put_styled(buf, area, x as i32, y as i32, ch, theme::bold(c.dim(fade)));
    }
    if open < 0.9 {
        return;
    }
    let outcome = match card.outcome {
        RelicOutcome::New => "Nouvelle relique !".to_string(),
        RelicOutcome::Upgraded => "Améliorée !".to_string(),
        RelicOutcome::Shard => "Doublon : +1 Éclat (+1 % de production)".to_string(),
    };
    let lines: [(String, Rgb); 5] = [
        ("RELIQUE TROUVÉE".to_string(), theme::MUTED),
        (format!("{} {}", set_symbol(def.set), def.name), color),
        (RARITY_NAMES[card.rarity as usize].to_uppercase(), color),
        (effect_text(def.effect_at(card.rarity)), theme::CYAN),
        (outcome, theme::GOLD),
    ];
    let inner_w = rect.width.saturating_sub(4) as usize;
    for (i, (text, c)) in lines.iter().enumerate() {
        let shown = draw::fit(text, inner_w);
        let tw = shown.chars().count() as u16;
        let x = rect.x + rect.width.saturating_sub(tw) / 2;
        let y = rect.y + 2 + i as u16 + u16::from(i > 2);
        if y + 1 >= rect.bottom() {
            continue;
        }
        draw::text(
            buf,
            x,
            y,
            &shown,
            theme::bold(c.dim(fade)),
            rect.right() - 1,
        );
    }
    // Reflet qui traverse la carte.
    let sweep = ((t - 0.5) / 0.9).clamp(0.0, 1.0);
    if sweep > 0.0 && sweep < 1.0 {
        effects::shimmer(
            buf,
            rect,
            sweep * (rect.width as f64 + 12.0) - 6.0,
            3.0,
            0.6,
        );
    }
    // Étincelles autour d'une relique épique ou légendaire.
    if card.rarity >= 2 {
        let n = if legendary { 18 } else { 8 };
        for i in 0..n {
            let a = i as f64 / n as f64 * TAU + ctx.now * 1.5;
            let r = 1.0 + (t * 2.0).min(1.0);
            let x = rect.x as f64
                + rect.width as f64 / 2.0
                + a.cos() * (rect.width as f64 / 2.0 + 2.0 * r);
            let y =
                rect.y as f64 + rect.height as f64 / 2.0 + a.sin() * (rect.height as f64 / 2.0 + r);
            let c = if legendary {
                gradient_cyclic(&SPECTRUM, i as f64 / n as f64 + ctx.now)
            } else {
                color
            };
            draw::put(
                buf,
                area,
                x.round() as i32,
                y.round() as i32,
                '✦',
                c.dim(fade),
            );
        }
    }
}

/// i-ème case du pourtour d'un rectangle (sens horaire depuis le coin haut gauche).
fn perimeter(r: Rect, i: usize) -> (u16, u16) {
    let w = r.width.max(1) as usize;
    let h = r.height.max(1) as usize;
    let i = i % (2 * (w + h) - 4).max(1);
    if i < w {
        (r.x + i as u16, r.y)
    } else if i < w + h - 1 {
        (r.right() - 1, r.y + (i - w + 1) as u16)
    } else if i < 2 * w + h - 2 {
        (r.right() - 1 - (i - (w + h - 1) + 1) as u16, r.bottom() - 1)
    } else {
        (r.x, r.bottom() - 1 - (i - (2 * w + h - 2) + 1) as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_navigation_stays_inside() {
        assert_eq!(navigate(0, -1, 0), 0);
        assert_eq!(navigate(0, 1, 0), 4);
        assert_eq!(navigate(3, 0, 1), 3);
        assert_eq!(navigate(5, 0, -1), 4);
        assert_eq!(navigate(15, 1, 1), 15);
    }

    #[test]
    fn perimeter_walks_the_border() {
        let r = Rect::new(2, 3, 5, 4);
        assert_eq!(perimeter(r, 0), (2, 3));
        assert_eq!(perimeter(r, 4), (6, 3));
        assert_eq!(perimeter(r, 5), (6, 4));
    }
}
