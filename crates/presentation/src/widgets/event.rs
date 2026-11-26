//! Événement cosmique (Acte III) : une fenêtre qui descend du haut de l'écran,
//! son titre qui se décrypte comme un signal capté, et les choix possibles.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Widget};

use domain::cosmic::effect_text;
use domain::EventChoice;

use super::Ctx;
use crate::draw;
use crate::fmt;
use crate::fx::easing::ease_out_back;
use crate::fx::effects;
use crate::fx::rng::hash3;
use crate::fx::Rgb;
use crate::theme;

const SIGNAL: Rgb = Rgb(120, 230, 255);
const GLYPHS: [char; 6] = ['▓', '▒', '░', '◇', '·', '╳'];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EventPopup {
    pub opened_at: f64,
    pub sel: usize,
}

/// Ce qu'un choix rapporte, risque compris.
pub fn choice_text(c: &EventChoice) -> String {
    match c.risk {
        Some((chance, fail)) => format!(
            "{} ({} de chances, sinon : {})",
            effect_text(c.effect),
            fmt::percent(chance),
            effect_text(fail)
        ),
        None => effect_text(c.effect),
    }
}

pub fn render(buf: &mut Buffer, area: Rect, ctx: &Ctx, popup: &EventPopup) {
    let Some((index, left)) = ctx.snap.pending_event else {
        return;
    };
    let Some(def) = ctx.catalog.events.get(index) else {
        return;
    };
    let t = ctx.now - popup.opened_at;
    effects::fade(buf, area, (t / 0.3).clamp(0.0, 1.0) * 0.6);
    let width = area.width.saturating_sub(4).min(84);
    let text_w = width.saturating_sub(4) as usize;
    let mut lines: Vec<(String, Style)> = Vec::new();
    for l in draw::wrap(def.text, text_w) {
        lines.push((l, theme::fg(theme::TEXT)));
    }
    lines.push((String::new(), Style::default()));
    let choices_start = lines.len();
    let mut choice_rows = Vec::new();
    for (i, c) in def.choices.iter().enumerate() {
        choice_rows.push(lines.len());
        let mark = if i == popup.sel { "▶ " } else { "  " };
        for (k, l) in draw::wrap(&format!("{mark}{}", c.label), text_w)
            .into_iter()
            .enumerate()
        {
            let _ = k;
            lines.push((
                l,
                theme::bold(if i == popup.sel {
                    theme::WHITE
                } else {
                    theme::TEXT
                }),
            ));
        }
        for l in draw::wrap(&format!("    → {}", choice_text(c)), text_w) {
            let color = if c.risk.is_some() {
                theme::ORANGE
            } else {
                theme::CYAN
            };
            lines.push((l, theme::fg(color)));
        }
    }
    let _ = choices_start;
    lines.push((String::new(), Style::default()));
    lines.push((
        format!(
            "Réponds avant {} — sinon l'occasion passe.",
            fmt::duration(left)
        ),
        theme::fg(theme::MUTED),
    ));
    // Bordures + titre + blanc + lignes + pied.
    let height = (lines.len() as u16 + 5).min(area.height.saturating_sub(2));
    // Descend du haut de l'écran avec un rebond.
    let drop = ease_out_back((t / 0.5).clamp(0.0, 1.0));
    let final_y = area.y + area.height.saturating_sub(height) / 2;
    let y = (area.y as f64 - height as f64
        + (final_y as f64 - area.y as f64 + height as f64) * drop)
        .round()
        .max(area.y as f64) as u16;
    let x = area.x + area.width.saturating_sub(width) / 2;
    let rect = area.intersection(Rect::new(x, y, width, height));
    Clear.render(rect, buf);
    let pulse = 0.5 + 0.5 * (ctx.now * 3.0).sin();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::bold(SIGNAL.lerp(Rgb(190, 110, 255), pulse)))
        .title(
            Line::from(Span::styled(
                " ✹ ÉVÉNEMENT COSMIQUE ✹ ",
                theme::bold(theme::GOLD),
            ))
            .centered(),
        );
    let inner = block.inner(rect);
    block.render(rect, buf);
    if inner.height < 4 || drop < 0.95 {
        return;
    }
    // Titre qui se décrypte, lettre après lettre.
    let title = def.title.to_uppercase();
    let tw = title.chars().count() as u16;
    let tx = inner.x + inner.width.saturating_sub(tw) / 2;
    let resolve = ((t - 0.4) / 0.8).clamp(0.0, 1.0);
    let n = title.chars().count().max(1);
    for (i, ch) in title.chars().enumerate() {
        let done = (i as f64 / n as f64) < resolve;
        let (g, c) = if done || ch == ' ' {
            (ch, theme::GOLD)
        } else {
            let k = hash3(i as i64, (ctx.now * 20.0) as i64, 9);
            (
                GLYPHS[(k * GLYPHS.len() as f64) as usize % GLYPHS.len()],
                SIGNAL.dim(0.4),
            )
        };
        draw::put_styled(
            buf,
            inner,
            tx as i32 + i as i32,
            inner.y as i32,
            g,
            theme::bold(c),
        );
    }
    let footer_y = inner.bottom() - 1;
    for (i, (l, st)) in lines.iter().enumerate() {
        let ly = inner.y + 2 + i as u16;
        if ly >= footer_y {
            draw::audit::record(l, 0);
            continue;
        }
        if choice_rows.get(popup.sel) == Some(&i) {
            draw::fill_bg(
                buf,
                Rect::new(inner.x, ly, inner.width, 1),
                theme::SELECT_BG,
            );
        }
        draw::text(buf, inner.x + 1, ly, l, *st, inner.right());
    }
    let long = "[↑↓] choisir   [Entrée] décider   [Échap] plus tard ([Z] pour revenir)";
    let footer = if long.chars().count() <= inner.width as usize {
        long
    } else {
        "[↑↓] [Entrée] décider  [Échap] plus tard"
    };
    let fw = footer.chars().count() as u16;
    draw::text(
        buf,
        inner.x + inner.width.saturating_sub(fw) / 2,
        footer_y,
        footer,
        theme::fg(theme::GREY),
        inner.right(),
    );
}
