//! Fenêtre des Défis (touche D) : un cycle avec une contrainte, contre une
//! récompense permanente. Lancer ou abandonner demande une confirmation.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Widget};

use application::ChallengeView;

use super::Ctx;
use crate::draw;
use crate::fmt;
use crate::fx::Rgb;
use crate::theme;
use crate::ui::Popup;

/// Statut affiché à droite d'un Défi, et sa couleur.
pub(crate) fn status(ctx: &Ctx, v: &ChallengeView, goal: f64, unlock: u32) -> (String, Rgb) {
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
    let snap = ctx.snap;
    let width = ctx.frame.width(area, 92);
    // Texte à une largeur de lecture raisonnable, même en onglet.
    let text_w = width.min(92).saturating_sub(4) as usize;
    let intro = draw::wrap(
        "Un Défi relance le cycle avec une contrainte. Réussis l'objectif : récompense pour toujours.",
        text_w,
    );
    let selected = selected.min(snap.challenges.len().saturating_sub(1));
    // Hauteur fixe : celle du détail le plus long, pour que la fenêtre ne
    // saute pas quand on change de sélection.
    let detail_h = (0..snap.challenges.len())
        .map(|i| detail_lines(ctx, i, armed, text_w).len())
        .max()
        .unwrap_or(0) as u16;
    let list_h = snap.challenges.len() as u16;
    // intro + blanc + liste + séparateur + détail + blanc + pied + bordures
    let height = (intro.len() as u16 + 1 + list_h + 1 + detail_h + 1 + 1 + 2)
        .min(area.height.saturating_sub(2));
    let height = ctx.frame.height(area, height);
    let Some((rect, ready)) =
        ctx.frame
            .place_faded(buf, area, ctx.now, popup, (width, height), 0.6)
    else {
        return;
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::bold(theme::PINK))
        .title(Line::from(Span::styled(" ◆ DÉFIS ◆ ", theme::bold(theme::GOLD))).centered());
    let inner = block.inner(rect);
    block.render(rect, buf);
    if !ready || inner.height < 8 {
        return;
    }
    let right = inner.right();
    let mut y = inner.y;
    for l in &intro {
        draw::text(buf, inner.x + 1, y, l, theme::fg(theme::MUTED), right);
        y += 1;
    }
    y += 1;

    for (i, v) in snap.challenges.iter().enumerate() {
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
        let st_x = right.saturating_sub(1 + st.chars().count() as u16);
        draw::text(buf, x, y, def.name, name_style, st_x.saturating_sub(1));
        draw::text_right(buf, right - 1, y, &st, theme::bold(color), inner.x);
        y += 1;
    }

    // Détail : contrainte, objectif, récompense, action.
    let sep: String = "─".repeat(inner.width.saturating_sub(2) as usize);
    draw::text(buf, inner.x + 1, y, &sep, theme::fg(theme::BORDER), right);
    y += 1;
    let footer_y = inner.bottom() - 1;
    for (text, style) in detail_lines(ctx, selected, armed, text_w) {
        if y >= footer_y {
            draw::audit::record(&text, 0);
            continue;
        }
        draw::text(buf, inner.x + 1, y, &text, style, right);
        y += 1;
    }

    let footer = "[↑↓] choisir   [Entrée] lancer / abandonner   [D] / [Échap] fermer";
    let footer = if footer.chars().count() <= inner.width as usize {
        footer
    } else {
        "[↑↓] choisir  [Entrée] valider  [Échap] fermer"
    };
    let fw = footer.chars().count() as u16;
    draw::text(
        buf,
        inner.x + inner.width.saturating_sub(fw) / 2,
        footer_y,
        footer,
        theme::fg(theme::GREY),
        right,
    );
}

/// Lignes du détail du Défi `index` (renvoyées à la ligne à `width`).
pub(crate) fn detail_lines(
    ctx: &Ctx,
    index: usize,
    armed: bool,
    width: usize,
) -> Vec<(String, ratatui::style::Style)> {
    let snap = ctx.snap;
    let Some((v, def)) = snap
        .challenges
        .get(index)
        .and_then(|v| ctx.catalog.challenge(v.id).map(|d| (v, d)))
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut push = |text: &str, style| {
        for l in draw::wrap(text, width) {
            out.push((l, style));
        }
    };
    push(
        &format!("Contrainte : {}", def.rule),
        theme::fg(theme::CYAN),
    );
    push(
        &format!(
            "Objectif : {} ✦ récoltés dans le cycle",
            fmt::short(def.goal)
        ),
        theme::fg(theme::CYAN),
    );
    push(
        &format!("Récompense : {}", def.reward_text),
        theme::bold(theme::GOLD),
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
    push(&hint, theme::bold(color));
    out
}
