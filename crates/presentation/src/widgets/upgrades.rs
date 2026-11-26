//! Panneau des améliorations : liste triée par coût + description de la sélection.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};

use super::Ctx;
use application::{GameSnapshot, NextUpgradeView};
use domain::{Effect, Requirement, UpgradeId};

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
        let max = inner.width.saturating_sub(2) as usize;
        let txt = draw::truncate(EMPTY_TEXT, max);
        let y = inner.y + inner.height.saturating_sub(3) / 2;
        draw::text(buf, inner.x + 1, y, &txt, theme::fg(c), inner.right());
        if let Some(next) = snap.next_upgrade {
            render_next(buf, inner, y + 1, ctx, next);
        }
        return;
    }
    ui.upg_sel = ui.upg_sel.min(n - 1);
    // Encart de détail : séparateur + effet chiffré (+ texte d'ambiance si la place le permet).
    let desc_h: u16 = if inner.height >= 7 {
        3
    } else if inner.height >= 4 {
        2
    } else {
        0
    };
    let list_h = (inner.height - desc_h) as usize;
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

    if desc_h > 0 {
        if let Some(u) = snap.upgrades.get(ui.upg_sel) {
            render_detail(buf, inner, desc_h, ctx, u.id);
        }
    }
}

/// Gain concret apporté par une amélioration, d'après la production actuelle.
pub fn impact(effect: Effect, snap: &GameSnapshot) -> Option<String> {
    match effect {
        Effect::GeneratorMultiplier { generator, factor } => {
            let prod = snap
                .generators
                .iter()
                .find(|g| g.id == generator)
                .map(|g| g.production)
                .unwrap_or(0.0);
            Some(format!("+{}/s", fmt::short(prod * (factor - 1.0))))
        }
        Effect::GlobalMultiplier(factor) => Some(format!(
            "+{}/s",
            fmt::short(snap.production * (factor - 1.0))
        )),
        Effect::ClickPercent(pct) => {
            Some(format!("+{} par clic", fmt::short(snap.production * pct)))
        }
        Effect::HoldHarvest => None,
    }
}

/// Encart « qui fait quoi » de l'amélioration sélectionnée.
fn render_detail(buf: &mut Buffer, inner: Rect, desc_h: u16, ctx: &Ctx, id: UpgradeId) {
    let Some(def) = ctx.catalog.upgrade(id) else {
        return;
    };
    let sep_y = inner.bottom() - desc_h;
    let sep: String = "─".repeat(inner.width.saturating_sub(2) as usize);
    draw::text(
        buf,
        inner.x + 1,
        sep_y,
        &sep,
        theme::fg(theme::BORDER),
        inner.right(),
    );
    let max = inner.width.saturating_sub(2) as usize;

    // Ligne 1 : l'effet, puis son gain concret maintenant.
    let y = sep_y + 1;
    let gain = impact(def.effect, ctx.snap);
    let gain_w = gain.as_ref().map(|g| g.chars().count() + 3).unwrap_or(0);
    let desc = draw::truncate(&def.description, max.saturating_sub(gain_w));
    let x = draw::text(
        buf,
        inner.x + 1,
        y,
        &desc,
        theme::fg(theme::CYAN),
        inner.right(),
    );
    if let Some(g) = gain {
        let x = draw::text(buf, x, y, " → ", theme::fg(theme::MUTED), inner.right());
        draw::text(buf, x, y, &g, theme::bold(theme::GREEN), inner.right());
    }

    // Ligne 2 : ce que c'est.
    if desc_h >= 3 {
        let flavor = draw::truncate(&def.flavor, max);
        draw::text(
            buf,
            inner.x + 1,
            y + 1,
            &flavor,
            theme::fg(theme::MUTED),
            inner.right(),
        );
    }
}

/// « Prochaine : … » : la prochaine amélioration verrouillée et où on en est.
fn render_next(buf: &mut Buffer, inner: Rect, y: u16, ctx: &Ctx, next: NextUpgradeView) {
    let Some(def) = ctx.catalog.upgrade(next.id) else {
        return;
    };
    let max = inner.width.saturating_sub(2) as usize;
    if y < inner.bottom() {
        let line = draw::truncate(&format!("Prochaine : {}", def.name), max);
        draw::text(
            buf,
            inner.x + 1,
            y,
            &line,
            theme::fg(theme::CYAN),
            inner.right(),
        );
    }
    if y + 1 < inner.bottom() {
        let what = match def.requirement {
            Requirement::GeneratorOwned { generator, .. } => ctx
                .catalog
                .generator(generator)
                .map(|g| g.plural.to_string())
                .unwrap_or_default(),
            Requirement::TotalRun(_) => "✦ récoltés".to_string(),
        };
        let pct = (next.current / next.needed.max(1e-9)).clamp(0.0, 1.0);
        let label = format!(
            " {} / {} {what}",
            fmt::short(next.current),
            fmt::short(next.needed)
        );
        let bar_w = (inner.width as usize)
            .saturating_sub(label.chars().count() + 3)
            .clamp(4, 20);
        let filled = (pct * bar_w as f64).round() as usize;
        let mut x = inner.x + 1;
        for i in 0..bar_w {
            let (ch, c) = if i < filled {
                ('█', theme::CYAN)
            } else {
                ('░', theme::BAR_EMPTY)
            };
            draw::put(buf, inner, x as i32, (y + 1) as i32, ch, c);
            x += 1;
        }
        draw::text(
            buf,
            x,
            y + 1,
            &label,
            theme::fg(theme::MUTED),
            inner.right(),
        );
    }
}
