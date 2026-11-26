//! Panneau des améliorations : liste triée par coût + description de la sélection.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};

use super::Ctx;
use crate::draw;
use crate::fmt;
use crate::fx::easing::ease_out_cubic;
use crate::fx::effects;
use crate::theme;
use crate::ui::{keep_visible, Panel, Ui};
use crate::visuals::Visuals;

pub const EMPTY_TEXT: &str = "Rien pour l'instant… continue de récolter ✦";

pub fn render(buf: &mut Buffer, area: Rect, ctx: &Ctx, vis: &mut Visuals, ui: &mut Ui) {
    let focused = ui.focus == Panel::Upgrades;
    let snap = ctx.snap;
    let n = snap.upgrades.len();
    let mut spans = vec![Span::styled(" Améliorations ", theme::title_style(focused))];
    if n > 0 {
        spans.push(Span::styled(format!("({n}) "), theme::fg(theme::MUTED)));
    }
    let inner = draw::panel(buf, area, Line::from(spans), focused);
    ui.upg_rows.clear();
    if inner.is_empty() {
        return;
    }
    let now = ctx.now;
    if n == 0 {
        let pulse = 0.5 + 0.5 * (now * 1.2).sin();
        let c = theme::MUTED.lerp(theme::GREY, pulse * 0.6);
        let txt = draw::truncate(EMPTY_TEXT, inner.width.saturating_sub(2) as usize);
        draw::text(
            buf,
            inner.x + 1,
            inner.y + inner.height / 2,
            &txt,
            theme::fg(c),
            inner.right(),
        );
        return;
    }
    ui.upg_sel = ui.upg_sel.min(n - 1);
    let has_desc = inner.height >= 4;
    let list_h = if has_desc {
        inner.height - 2
    } else {
        inner.height
    } as usize;
    keep_visible(ui.upg_sel, &mut ui.upg_scroll, list_h);
    if n <= list_h {
        ui.upg_scroll = 0;
    }

    for (row, (idx, u)) in snap
        .upgrades
        .iter()
        .enumerate()
        .skip(ui.upg_scroll)
        .take(list_h)
        .enumerate()
    {
        let y = inner.y + row as u16;
        let Some(def) = ctx.catalog.upgrade(u.id) else {
            continue;
        };
        ui.upg_rows.push((u.id, y));
        let selected = idx == ui.upg_sel;
        let line = Rect::new(inner.x, y, inner.width, 1);
        if selected {
            draw::fill_bg(buf, line, theme::SELECT_BG);
        }
        let ui_i = u.id.index();
        let fail = (now - vis.upg_fail.get(ui_i).copied().unwrap_or(f64::MIN)) / 0.3;
        let failing = (0.0..1.0).contains(&fail);
        let cost = fmt::short(u.cost);
        let cost_color = if failing && ((now * 14.0) as i64) % 2 == 0 {
            theme::RED
        } else if u.affordable {
            theme::GREEN
        } else {
            theme::GREY
        };
        let cursor_color = if focused { theme::FOCUS } else { theme::MUTED };
        let mut x = draw::text(
            buf,
            inner.x,
            y,
            if selected { "▶ " } else { "  " },
            theme::bold(cursor_color),
            inner.right(),
        );
        let cost_x = inner
            .right()
            .saturating_sub(cost.chars().count() as u16 + 1);
        let name = draw::truncate(&def.name, cost_x.saturating_sub(x + 1) as usize);
        let name_style = if selected {
            theme::bold(theme::TEXT)
        } else {
            theme::fg(theme::TEXT)
        };
        x = draw::text(buf, x, y, &name, name_style, cost_x);
        let _ = x;
        draw::text(
            buf,
            cost_x,
            y,
            &cost,
            theme::bold(cost_color),
            inner.right(),
        );

        let flash = (now - vis.upg_flash.get(ui_i).copied().unwrap_or(f64::MIN)) / 0.4;
        if (0.0..1.0).contains(&flash) {
            effects::tint_bg(
                buf,
                line,
                theme::GREEN.dim(0.5),
                1.0 - ease_out_cubic(flash),
            );
        }
        if failing && !ctx.reduced {
            let dx =
                ((fail * std::f64::consts::TAU * 2.0).sin() * 2.0 * (1.0 - fail)).round() as i32;
            effects::offset(buf, line, dx, 0);
        }
    }

    if has_desc {
        let sep_y = inner.bottom() - 2;
        let sep: String = "─".repeat(inner.width.saturating_sub(2) as usize);
        draw::text(
            buf,
            inner.x + 1,
            sep_y,
            &sep,
            theme::fg(theme::BORDER),
            inner.right(),
        );
        if let Some(def) = snap
            .upgrades
            .get(ui.upg_sel)
            .and_then(|u| ctx.catalog.upgrade(u.id))
        {
            let desc = draw::truncate(&def.description, inner.width.saturating_sub(2) as usize);
            draw::text(
                buf,
                inner.x + 1,
                sep_y + 1,
                &desc,
                theme::fg(theme::CYAN),
                inner.right(),
            );
        }
    }
}
