//! Fenêtre des Défis (touche D) : un cycle avec une contrainte, contre une
//! récompense permanente. Lancer ou abandonner demande une confirmation.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Widget};

use application::ChallengeView;

use super::Ctx;
use crate::draw;
use crate::fmt;
use crate::fx::easing::ease_out_cubic;
use crate::fx::effects;
use crate::fx::Rgb;
use crate::layout::centered;
use crate::theme;
use crate::ui::Popup;

/// Statut affiché à droite d'un Défi, et sa couleur.
fn status(ctx: &Ctx, v: &ChallengeView, goal: f64, unlock: u32) -> (String, Rgb) {
    if v.done {
        ("● réussi".into(), theme::GREEN)
    } else if v.active {
        let pct = ctx
            .snap
            .active_challenge
            .map(|a| fmt::percent(a.progress))
            .unwrap_or_default();
        (format!("en cours · {pct}"), theme::CYAN)
    } else if !v.unlocked {
        (
            format!(
                "après {unlock} Supernova{}",
                if unlock > 1 { "s" } else { "" }
            ),
            theme::GREY,
        )
    } else {
        (format!("objectif {} ✦", fmt::short(goal)), theme::GOLD)
    }
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
    effects::fade(buf, area, 0.6 * s);
    let height = (snap.challenges.len() as u16 + 10).min(area.height.saturating_sub(2));
    let width = area.width.saturating_sub(4).min(84);
    let w = ((width as f64 * s).round() as u16).max(2);
    let h = ((height as f64 * s).round() as u16).max(2);
    let rect = centered(area, w, h);
    Clear.render(rect, buf);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::bold(theme::PINK))
        .title(Line::from(Span::styled(" ◆ DÉFIS ◆ ", theme::bold(theme::GOLD))).centered());
    let inner = block.inner(rect);
    block.render(rect, buf);
    if s < 0.95 || inner.height < 8 {
        return;
    }
    let right = inner.right();
    let max = inner.width.saturating_sub(2) as usize;
    draw::text(
        buf,
        inner.x + 1,
        inner.y,
        &draw::truncate(
            "Un Défi relance le cycle avec une contrainte. Réussis l'objectif : récompense pour toujours.",
            max,
        ),
        theme::fg(theme::MUTED),
        right,
    );

    let selected = selected.min(snap.challenges.len().saturating_sub(1));
    for (i, v) in snap.challenges.iter().enumerate() {
        let y = inner.y + 2 + i as u16;
        if y >= inner.bottom().saturating_sub(5) {
            break;
        }
        let Some(def) = ctx.catalog.challenge(v.id) else {
            continue;
        };
        let is_sel = i == selected;
        if is_sel {
            draw::fill_bg(buf, Rect::new(inner.x, y, inner.width, 1), theme::SELECT_BG);
        }
        let (st, color) = status(ctx, v, def.goal, def.unlock_supernovas);
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
        let name_style = if v.unlocked {
            theme::bold(theme::TEXT)
        } else {
            theme::fg(theme::GREY)
        };
        draw::text(buf, x, y, def.name, name_style, right);
        draw::text_right(buf, right - 1, y, &st, theme::bold(color), inner.x);
    }

    // Détail : contrainte, objectif, récompense, action.
    let sep_y = inner.bottom() - 5;
    let sep: String = "─".repeat(inner.width.saturating_sub(2) as usize);
    draw::text(
        buf,
        inner.x + 1,
        sep_y,
        &sep,
        theme::fg(theme::BORDER),
        right,
    );
    if let Some((v, def)) = snap
        .challenges
        .get(selected)
        .and_then(|v| ctx.catalog.challenge(v.id).map(|d| (v, d)))
    {
        let rule = format!(
            "Contrainte : {} · Objectif : {} ✦ récoltés dans le cycle",
            def.rule,
            fmt::short(def.goal)
        );
        draw::text(
            buf,
            inner.x + 1,
            sep_y + 1,
            &draw::truncate(&rule, max),
            theme::fg(theme::CYAN),
            right,
        );
        let reward = format!("Récompense : {}", def.reward_text);
        draw::text(
            buf,
            inner.x + 1,
            sep_y + 2,
            &draw::truncate(&reward, max),
            theme::bold(theme::GOLD),
            right,
        );
        let busy = snap.active_challenge.is_some();
        let (hint, color) = if v.done {
            (
                "Déjà réussi : la récompense est acquise.".to_string(),
                theme::GREEN,
            )
        } else if v.active {
            let time = snap
                .active_challenge
                .and_then(|a| a.time_left)
                .map(|t| format!(" · il reste {}", fmt::countdown(t)))
                .unwrap_or_default();
            if armed {
                (
                    "Entrée de nouveau pour ABANDONNER (le cycle continue sans contrainte)".into(),
                    theme::ORANGE,
                )
            } else {
                (format!("En cours{time} · [Entrée] abandonner"), theme::CYAN)
            }
        } else if !v.unlocked {
            (
                format!("Débloqué après {} Supernovas.", def.unlock_supernovas),
                theme::GREY,
            )
        } else if busy {
            ("Un autre Défi est en cours.".to_string(), theme::GREY)
        } else if armed {
            (
                "Entrée de nouveau pour LANCER : le cycle repart de zéro !".to_string(),
                theme::ORANGE,
            )
        } else {
            (
                "[Entrée] lancer (une Supernova a lieu d'abord si elle rapporte des fragments)"
                    .to_string(),
                theme::MUTED,
            )
        };
        draw::text(
            buf,
            inner.x + 1,
            sep_y + 3,
            &draw::truncate(&hint, max),
            theme::bold(color),
            right,
        );
    }

    let footer = draw::truncate(
        "[↑↓] choisir   [Entrée] lancer / abandonner   [D] / [Échap] fermer",
        max,
    );
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
