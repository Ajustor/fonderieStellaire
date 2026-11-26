//! Popup de confirmation de Supernova : scale-in depuis le centre, fond assombri.

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

const WIDTH: u16 = 58;
const HEIGHT: u16 = 11;

pub fn render(buf: &mut Buffer, area: Rect, ctx: &Ctx, popup: &Popup) {
    let s = ease_out_cubic(popup.scale(ctx.now));
    if s <= 0.0 {
        return;
    }
    effects::fade(buf, area, 0.6 * s);
    let w = ((WIDTH as f64 * s).round() as u16).max(2);
    let h = ((HEIGHT as f64 * s).round() as u16).max(2);
    let rect = centered(area, w, h);
    Clear.render(rect, buf);
    let glow = 0.5 + 0.5 * (ctx.now * 3.0).sin();
    let border = theme::ORANGE.lerp(theme::PINK, glow);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::bold(border))
        .title(Line::from(Span::styled(" ✦ SUPERNOVA ✦ ", theme::bold(theme::GOLD))).centered());
    let inner = block.inner(rect);
    block.render(rect, buf);
    if s < 0.95 || inner.height < 7 {
        return;
    }
    let snap = ctx.snap;
    let gained = snap.fragments_available;
    let new_bonus = domain::prestige::fragment_multiplier(snap.fragments + gained) - 1.0;
    let lines: [(String, ratatui::style::Style); 7] = [
        (
            "Faire exploser ta fonderie en Supernova ?".into(),
            theme::bold(theme::TEXT),
        ),
        (String::new(), theme::fg(theme::TEXT)),
        (
            format!("Fragments gagnés      +{}", fmt::short(gained)),
            theme::bold(theme::PINK),
        ),
        (
            format!(
                "Bonus de production   +{} → +{}",
                fmt::percent(snap.fragment_bonus),
                fmt::percent(new_bonus)
            ),
            theme::bold(theme::CYAN),
        ),
        (String::new(), theme::fg(theme::TEXT)),
        (
            "Tout repart de zéro, sauf fragments, succès et stats.".into(),
            theme::fg(theme::MUTED),
        ),
        (String::new(), theme::fg(theme::TEXT)),
    ];
    for (i, (text, style)) in lines.iter().enumerate() {
        let y = inner.y + i as u16;
        let tw = text.chars().count() as u16;
        let x = inner.x + inner.width.saturating_sub(tw) / 2;
        draw::text(buf, x, y, text, *style, inner.right());
    }
    let y = inner.y + 7;
    let yes = "[O] Oui";
    let no = "[N] Non";
    let total = (yes.len() + no.len() + 6) as u16;
    let x = inner.x + inner.width.saturating_sub(total) / 2;
    let x = draw::text(buf, x, y, yes, theme::bold(theme::GREEN), inner.right());
    draw::text(buf, x + 6, y, no, theme::bold(theme::RED), inner.right());
}

/// Confirmation du Big Crunch : ce qui est perdu, ce qui est gardé, ce qu'on gagne.
pub fn render_crunch(buf: &mut Buffer, area: Rect, ctx: &Ctx, popup: &Popup) {
    let s = ease_out_cubic(popup.scale(ctx.now));
    if s <= 0.0 {
        return;
    }
    effects::fade(buf, area, 0.7 * s);
    let w = ((72.0 * s).round() as u16).max(2);
    let h = ((13.0 * s).round() as u16).max(2);
    let rect = centered(area, w, h);
    Clear.render(rect, buf);
    let glow = 0.5 + 0.5 * (ctx.now * 2.0).sin();
    let border = theme::PINK.lerp(crate::fx::Rgb(170, 90, 255), glow);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::bold(border))
        .title(Line::from(Span::styled(" ✶ BIG CRUNCH ✶ ", theme::bold(theme::GOLD))).centered());
    let inner = block.inner(rect);
    block.render(rect, buf);
    if s < 0.95 || inner.height < 9 {
        return;
    }
    let snap = ctx.snap;
    let next = snap.crunches + 1;
    let prod_after = 1.0 + domain::catalog::balance::CRUNCH_PRODUCTION * next as f64;
    let frag_after = 1.0 + domain::catalog::balance::CRUNCH_FRAGMENTS * next as f64;
    let lines: [(String, ratatui::style::Style); 9] = [
        (
            "Replier tout l'univers sur lui-même ?".into(),
            theme::bold(theme::TEXT),
        ),
        (String::new(), theme::fg(theme::TEXT)),
        (
            format!("Matière noire : {} → {}", snap.crunches, next),
            theme::bold(theme::PINK),
        ),
        (
            format!(
                "Production ×{} → ×{}   ·   fragments +{} → +{}",
                fmt::short(snap.crunch_production),
                fmt::short(prod_after),
                fmt::percent(snap.crunch_fragments - 1.0),
                fmt::percent(frag_after - 1.0)
            ),
            theme::bold(theme::CYAN),
        ),
        (String::new(), theme::fg(theme::TEXT)),
        (
            "Perdu : Supernovas, fragments, Atelier, Constellations, Défis.".into(),
            theme::fg(theme::ORANGE),
        ),
        (
            "Gardé : succès, statistiques, Récolte continue.".into(),
            theme::fg(theme::GREEN),
        ),
        (String::new(), theme::fg(theme::TEXT)),
        (String::new(), theme::fg(theme::TEXT)),
    ];
    for (i, (text, style)) in lines.iter().enumerate() {
        let y = inner.y + i as u16;
        let text = crate::draw::truncate(text, inner.width.saturating_sub(2) as usize);
        let tw = text.chars().count() as u16;
        let x = inner.x + inner.width.saturating_sub(tw) / 2;
        draw::text(buf, x, y, &text, *style, inner.right());
    }
    let y = inner.y + 9;
    let yes = "[O] Oui";
    let no = "[N] Non";
    let total = (yes.len() + no.len() + 6) as u16;
    let x = inner.x + inner.width.saturating_sub(total) / 2;
    let x = draw::text(buf, x, y, yes, theme::bold(theme::GREEN), inner.right());
    draw::text(buf, x + 6, y, no, theme::bold(theme::RED), inner.right());
}
