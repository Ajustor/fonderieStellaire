//! Atelier stellaire (touche E) : améliorations uniques et Constellations à
//! niveaux, payées en fragments de Supernova.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Widget};

use application::GameSnapshot;
use domain::comet::STORM_MULTIPLIER;
use domain::ConstellationEffect;

use super::Ctx;
use crate::draw;
use crate::fmt;
use crate::fx::easing::ease_out_cubic;
use crate::fx::effects;
use crate::fx::Rgb;
use crate::layout::centered;
use crate::theme;
use crate::ui::Popup;

/// Ce qui est sélectionnable dans l'Atelier, dans l'ordre d'affichage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShopItem {
    Perk(usize),
    Constellation(usize),
}

/// Nombre d'éléments sélectionnables.
pub fn item_count(snap: &GameSnapshot) -> usize {
    snap.perks.len() + snap.constellations.len()
}

/// Élément correspondant à l'index de sélection.
pub fn item_at(snap: &GameSnapshot, selected: usize) -> Option<ShopItem> {
    if selected < snap.perks.len() {
        Some(ShopItem::Perk(selected))
    } else if selected < item_count(snap) {
        Some(ShopItem::Constellation(selected - snap.perks.len()))
    } else {
        None
    }
}

/// Effet cumulé d'une Constellation à un niveau donné, en clair.
pub fn constellation_total(effect: ConstellationEffect, level: u32) -> String {
    let l = level as f64;
    match effect {
        ConstellationEffect::Production(x) => format!("+{} de production", fmt::percent(x * l)),
        ConstellationEffect::Click(x) => format!("+{} de la prod/s par clic", fmt::percent(x * l)),
        ConstellationEffect::Comets(x) => format!(
            "tempête ×{}, pluie d'étoiles +{}",
            fmt::short(STORM_MULTIPLIER + l),
            fmt::percent(x * l)
        ),
        ConstellationEffect::OfflineHours(h) => format!("+{} h de hors-ligne", fmt::short(h * l)),
        ConstellationEffect::Fragments(x) => format!("+{} de fragments", fmt::percent(x * l)),
        ConstellationEffect::Discount(d) => {
            format!("générateurs −{}", fmt::percent(1.0 - d.powi(level as i32)))
        }
    }
}

enum Row {
    Header(&'static str),
    Item(ShopItem),
}

pub fn render(buf: &mut Buffer, area: Rect, ctx: &Ctx, popup: &Popup, selected: usize) {
    let s = ease_out_cubic(popup.scale(ctx.now));
    if s <= 0.0 {
        return;
    }
    let snap = ctx.snap;
    effects::fade(buf, area, 0.6 * s);

    let mut rows: Vec<Row> = Vec::with_capacity(item_count(snap) + 2);
    rows.push(Row::Header("ATELIER · une fois pour toutes"));
    rows.extend((0..snap.perks.len()).map(|i| Row::Item(ShopItem::Perk(i))));
    rows.push(Row::Header(
        "CONSTELLATIONS · sans fin, chaque niveau coûte plus cher",
    ));
    rows.extend((0..snap.constellations.len()).map(|i| Row::Item(ShopItem::Constellation(i))));

    let height = (rows.len() as u16 + 8).min(area.height.saturating_sub(2));
    let width = area.width.saturating_sub(4).min(84);
    let w = ((width as f64 * s).round() as u16).max(2);
    let h = ((height as f64 * s).round() as u16).max(2);
    let rect = centered(area, w, h);
    Clear.render(rect, buf);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::bold(theme::GOLD))
        .title(
            Line::from(Span::styled(
                " ◆ ATELIER STELLAIRE ◆ ",
                theme::bold(theme::GOLD),
            ))
            .centered(),
        );
    let inner = block.inner(rect);
    block.render(rect, buf);
    if s < 0.95 || inner.height < 7 {
        return;
    }
    let right = inner.right();
    let max = inner.width.saturating_sub(2) as usize;

    // En-tête : fragments libres et rappel que le bonus reste acquis.
    let head = format!(
        "Fragments libres : {} sur {}   ·   bonus permanent +{} (conservé après achat)",
        fmt::short(snap.fragments_spendable),
        fmt::short(snap.fragments),
        fmt::percent(snap.fragment_bonus)
    );
    draw::text(
        buf,
        inner.x + 1,
        inner.y,
        &draw::truncate(&head, max),
        theme::bold(theme::CYAN),
        right,
    );

    // Liste : en-têtes de section + éléments ; défilement pour garder la sélection visible.
    let selected = selected.min(item_count(snap).saturating_sub(1));
    let current = item_at(snap, selected);
    let sel_row = rows
        .iter()
        .position(|r| matches!(r, Row::Item(it) if Some(*it) == current))
        .unwrap_or(0);
    let list_y = inner.y + 2;
    let list_rows = inner.height.saturating_sub(7) as usize;
    let scroll = (sel_row + 1).saturating_sub(list_rows);
    for (n, row) in rows.iter().enumerate().skip(scroll).take(list_rows) {
        let y = list_y + (n - scroll) as u16;
        match row {
            Row::Header(title) => {
                draw::text(
                    buf,
                    inner.x + 1,
                    y,
                    &draw::truncate(title, max),
                    theme::bold(theme::MUTED),
                    right,
                );
            }
            Row::Item(item) => render_item(buf, inner, y, ctx, *item, Some(*item) == current),
        }
    }

    // Détail de la sélection.
    let sep_y = inner.bottom() - 4;
    let sep: String = "─".repeat(inner.width.saturating_sub(2) as usize);
    draw::text(
        buf,
        inner.x + 1,
        sep_y,
        &sep,
        theme::fg(theme::BORDER),
        right,
    );
    if let Some(item) = current {
        let (desc, hint) = detail(ctx, item);
        draw::text(
            buf,
            inner.x + 1,
            sep_y + 1,
            &draw::truncate(&desc, max),
            theme::fg(theme::CYAN),
            right,
        );
        draw::text(
            buf,
            inner.x + 1,
            sep_y + 2,
            &draw::truncate(&hint, max),
            theme::fg(theme::MUTED),
            right,
        );
    }

    let mut footer = String::from("[↑↓] choisir   [Entrée] acheter   [E] / [Échap] fermer");
    if snap.autobuy_unlocked {
        footer.push_str(if snap.autobuy {
            "   [T] automate : actif"
        } else {
            "   [T] automate : arrêté"
        });
    }
    let footer = draw::truncate(&footer, max);
    let fw = footer.chars().count() as u16;
    draw::text(
        buf,
        inner.x + inner.width.saturating_sub(fw) / 2,
        inner.bottom() - 1,
        &footer,
        theme::fg(theme::GREY),
        right,
    );
}

fn render_item(buf: &mut Buffer, inner: Rect, y: u16, ctx: &Ctx, item: ShopItem, is_sel: bool) {
    let snap = ctx.snap;
    let right = inner.right();
    // (symbole, nom, statut, couleur, déjà « fait »)
    let (symbol, name, status, color, done): (char, &str, String, Rgb, bool) = match item {
        ShopItem::Perk(i) => {
            let Some(p) = snap.perks.get(i) else { return };
            let Some(def) = ctx.catalog.perk(p.id) else {
                return;
            };
            let color = if p.owned {
                theme::GREEN
            } else if p.affordable {
                theme::GOLD
            } else {
                theme::GREY
            };
            let status = if p.owned {
                "● acquise".to_string()
            } else {
                format!("{} ◆", fmt::short(def.cost))
            };
            (def.symbol, def.name, status, color, p.owned)
        }
        ShopItem::Constellation(i) => {
            let Some(c) = snap.constellations.get(i) else {
                return;
            };
            let Some(def) = ctx.catalog.constellation(c.id) else {
                return;
            };
            let color = if c.maxed {
                theme::GREEN
            } else if c.affordable {
                theme::GOLD
            } else {
                theme::GREY
            };
            let status = if c.maxed {
                format!("niv. {} · MAX", c.level)
            } else {
                format!("niv. {} · {} ◆", c.level, fmt::short(c.cost))
            };
            (def.symbol, def.name, status, color, c.maxed)
        }
    };
    if is_sel {
        draw::fill_bg(buf, Rect::new(inner.x, y, inner.width, 1), theme::SELECT_BG);
    }
    let mut x = draw::text(
        buf,
        inner.x + 1,
        y,
        if is_sel { "▶ " } else { "  " },
        theme::bold(theme::FOCUS),
        right,
    );
    x = draw::text(buf, x, y, &format!("{symbol} "), theme::bold(color), right);
    let name_style = if done {
        theme::fg(theme::GREEN)
    } else {
        theme::bold(theme::TEXT)
    };
    draw::text(buf, x, y, name, name_style, right);
    draw::text_right(buf, right - 1, y, &status, theme::bold(color), inner.x);
}

/// (ce que ça fait, indication d'achat) pour l'élément sélectionné.
fn detail(ctx: &Ctx, item: ShopItem) -> (String, String) {
    let snap = ctx.snap;
    let missing = |cost: f64| {
        format!(
            "Il manque {} fragment(s) : déclenche une Supernova [P].",
            fmt::short((cost - snap.fragments_spendable).max(1.0))
        )
    };
    match item {
        ShopItem::Perk(i) => {
            let Some(p) = snap.perks.get(i) else {
                return (String::new(), String::new());
            };
            let Some(def) = ctx.catalog.perk(p.id) else {
                return (String::new(), String::new());
            };
            let hint = if p.owned {
                "Déjà acquise : active pour toujours, même après une Supernova.".to_string()
            } else if p.affordable {
                "[Entrée] pour l'acheter.".to_string()
            } else {
                missing(def.cost)
            };
            (def.description.to_string(), hint)
        }
        ShopItem::Constellation(i) => {
            let Some(c) = snap.constellations.get(i) else {
                return (String::new(), String::new());
            };
            let Some(def) = ctx.catalog.constellation(c.id) else {
                return (String::new(), String::new());
            };
            let now = if c.level > 0 {
                format!(
                    " · actuellement : {}",
                    constellation_total(def.effect, c.level)
                )
            } else {
                String::new()
            };
            let hint = if c.maxed {
                "Niveau maximal atteint.".to_string()
            } else if c.affordable {
                format!(
                    "[Entrée] → niveau {} : {}",
                    c.level + 1,
                    constellation_total(def.effect, c.level + 1)
                )
            } else {
                missing(c.cost)
            };
            (format!("{}{now}", def.description), hint)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totals_are_readable() {
        assert_eq!(
            constellation_total(ConstellationEffect::Production(0.2), 3),
            "+60% de production"
        );
        assert_eq!(
            constellation_total(ConstellationEffect::Comets(0.25), 2),
            "tempête ×9, pluie d'étoiles +50%"
        );
        assert_eq!(
            constellation_total(ConstellationEffect::OfflineHours(2.0), 3),
            "+6 h de hors-ligne"
        );
        assert_eq!(
            constellation_total(ConstellationEffect::Discount(0.97), 1),
            "générateurs −3%"
        );
    }
}
