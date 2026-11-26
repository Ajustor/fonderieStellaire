//! Atelier stellaire (touche E) : améliorations permanentes payées en fragments.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Widget};

use super::Ctx;
use crate::draw;
use crate::fmt;
use crate::fx::easing::ease_out_cubic;
use crate::fx::effects;
use crate::layout::centered;
use crate::theme;
use crate::ui::Popup;

pub fn render(buf: &mut Buffer, area: Rect, ctx: &Ctx, popup: &Popup, selected: usize) {
    let s = ease_out_cubic(popup.scale(ctx.now));
    if s <= 0.0 {
        return;
    }
    let snap = ctx.snap;
    effects::fade(buf, area, 0.6 * s);
    let height = (snap.perks.len() as u16 + 9).min(area.height.saturating_sub(2));
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
    if s < 0.95 || inner.height < 6 {
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

    // Liste des améliorations.
    let list_y = inner.y + 2;
    let list_rows = inner.height.saturating_sub(7) as usize;
    let selected = selected.min(snap.perks.len().saturating_sub(1));
    let scroll = selected.saturating_sub(list_rows.saturating_sub(1));
    for (row, (i, p)) in snap
        .perks
        .iter()
        .enumerate()
        .skip(scroll)
        .take(list_rows)
        .enumerate()
    {
        let Some(def) = ctx.catalog.perk(p.id) else {
            continue;
        };
        let y = list_y + row as u16;
        let is_sel = i == selected;
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
        let color = if p.owned {
            theme::GREEN
        } else if p.affordable {
            theme::GOLD
        } else {
            theme::GREY
        };
        x = draw::text(
            buf,
            x,
            y,
            &format!("{} ", def.symbol),
            theme::bold(color),
            right,
        );
        let name_style = if p.owned {
            theme::fg(theme::GREEN)
        } else {
            theme::bold(theme::TEXT)
        };
        draw::text(buf, x, y, def.name, name_style, right);
        let status = if p.owned {
            "● acquise".to_string()
        } else {
            format!("{} ◆", fmt::short(def.cost))
        };
        draw::text_right(buf, right - 1, y, &status, theme::bold(color), inner.x);
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
    if let Some((p, def)) = snap
        .perks
        .get(selected)
        .and_then(|p| ctx.catalog.perk(p.id).map(|d| (p, d)))
    {
        let desc = draw::truncate(def.description, max);
        draw::text(
            buf,
            inner.x + 1,
            sep_y + 1,
            &desc,
            theme::fg(theme::CYAN),
            right,
        );
        let hint = if p.owned {
            "Déjà acquise : active pour toujours, même après une Supernova.".to_string()
        } else if p.affordable {
            "[Entrée] pour l'acheter.".to_string()
        } else {
            let missing = def.cost - snap.fragments_spendable;
            format!(
                "Il manque {} fragment(s) : déclenche une Supernova [P].",
                fmt::short(missing.max(1.0))
            )
        };
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
