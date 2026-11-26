//! Écran « Infos » (touche I) : succès obtenus ou non, et comment marche chaque mécanique.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Widget};

use domain::catalog::balance::{ACHIEVEMENT_BONUS, FRAGMENT_BONUS};

use super::Ctx;
use crate::draw;
use crate::fmt;
use crate::fx::easing::ease_out_cubic;
use crate::fx::effects;
use crate::layout::centered;
use crate::theme;
use crate::ui::Popup;

/// Lignes du contenu (texte, style), recalculées à l'ouverture.
pub fn lines(ctx: &Ctx) -> Vec<(String, Style)> {
    let snap = ctx.snap;
    let mut out: Vec<(String, Style)> = Vec::new();
    let text = theme::fg(theme::TEXT);
    let pr = snap.progress;

    // --- Où on en est ---
    out.push((
        format!("PROGRESSION  {}", fmt::percent(pr.completion)),
        theme::bold(theme::GOLD),
    ));
    let bar_w = 30usize;
    let filled = ((pr.completion.clamp(0.0, 1.0)) * bar_w as f64).round() as usize;
    out.push((
        format!(
            " {}{}  générateurs {}/{} · succès {}/{} · Atelier {}/{}",
            "█".repeat(filled),
            "░".repeat(bar_w - filled),
            pr.generators_found,
            pr.generators_total,
            snap.achievements_unlocked,
            snap.achievements_total,
            pr.perks_owned,
            pr.perks_total
        ),
        theme::fg(theme::GOLD),
    ));
    out.push((
        format!(
            " ✦ Récolté : {} ce cycle · {} depuis toujours",
            fmt::short(snap.total_run),
            fmt::short(snap.total_all)
        ),
        text,
    ));
    let missing = (pr.next_fragment_at - snap.total_run).max(0.0);
    out.push((
        format!(
            " ◆ Prochain fragment : encore {} ✦ (à {} récoltés ce cycle)",
            fmt::short(missing),
            fmt::short(pr.next_fragment_at)
        ),
        text,
    ));
    out.push((
        format!(
            " ▲ Améliorations ce cycle : {}/{} · Supernovas : {} · Constellations : {} niv. · Temps : {}",
            pr.upgrades_bought,
            pr.upgrades_total,
            snap.supernovas,
            snap.constellation_levels,
            fmt::duration(snap.play_time)
        ),
        text,
    ));
    out.push((String::new(), Style::default()));

    let unlocked = snap.achievements_unlocked;
    out.push((
        format!(
            "SUCCÈS  {}/{}  (+{} de production)",
            unlocked,
            snap.achievements_total,
            fmt::percent(unlocked as f64 * ACHIEVEMENT_BONUS)
        ),
        theme::bold(theme::GOLD),
    ));
    for (i, a) in ctx.catalog.achievements.iter().enumerate() {
        let done = snap.achievements.get(i).copied().unwrap_or(false);
        let (mark, style) = if done {
            ("★", theme::fg(theme::GOLD))
        } else {
            ("○", theme::fg(theme::GREY))
        };
        out.push((format!(" {mark} {} — {}", a.name, a.description), style));
    }
    out.push((String::new(), Style::default()));
    out.push(("COMMENT ÇA MARCHE".into(), theme::bold(theme::CYAN)));
    let mechanics = [
        "✧ Générateurs : produisent en continu ; chaque achat coûte 15 % de plus.".to_string(),
        "◆ Paliers : production ×2 à 25, 50, 100, 200, 300, 400 et 500 exemplaires.".into(),
        "▲ Améliorations : multiplient un générateur, toute la production ou tes clics.".into(),
        "● Comètes : toutes les 1m30 à 4m ; [C] pour la capturer (tempête ×7 ou pluie).".into(),
        format!(
            "✦ Supernova : repart de zéro contre des fragments (+{} chacun, puis moins vite).",
            fmt::percent(FRAGMENT_BONUS)
        ),
        "✦ Chaque Supernova rend la suivante un peu plus exigeante (+25 % de seuil).".into(),
        "◎ Nouveaux générateurs : Trou blanc (1 Supernova), Quasars (5), Multivers (15).".into(),
        format!(
            "★ Succès : +{} de production chacun, pour toujours.",
            fmt::percent(ACHIEVEMENT_BONUS)
        ),
        "◆ Atelier [E] : dépense tes fragments en bonus permanents (le bonus de prod reste)."
            .into(),
        "★ Constellations [E] : niveaux sans fin pour tes fragments, de plus en plus chers.".into(),
        "○ Hors-ligne : ta fonderie produit à 50 % pendant ton absence (12 h max).".into(),
        "· Raccourcis : [A] tout acheter · [T] automate · [PgUp]/[PgDn] journal.".into(),
        "· Sauvegarde automatique toutes les 30 s et en quittant.".into(),
    ];
    for m in mechanics {
        out.push((format!(" {m}"), text));
    }
    out
}

/// Nombre de lignes visibles pour une zone donnée (pour borner le défilement).
pub fn visible_rows(area: Rect) -> usize {
    rect(area).height.saturating_sub(4) as usize
}

fn rect(area: Rect) -> Rect {
    centered(
        area,
        area.width.saturating_sub(4).min(90),
        area.height.saturating_sub(2),
    )
}

pub fn render(buf: &mut Buffer, area: Rect, ctx: &Ctx, popup: &Popup, scroll: &mut usize) {
    let s = ease_out_cubic(popup.scale(ctx.now));
    if s <= 0.0 {
        return;
    }
    effects::fade(buf, area, 0.6 * s);
    let full = rect(area);
    let w = ((full.width as f64 * s).round() as u16).max(2);
    let h = ((full.height as f64 * s).round() as u16).max(2);
    let r = centered(area, w, h);
    Clear.render(r, buf);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::bold(theme::CYAN))
        .title(Line::from(Span::styled(" ✦ INFOS ✦ ", theme::bold(theme::GOLD))).centered());
    let inner = block.inner(r);
    block.render(r, buf);
    if s < 0.95 || inner.height < 3 {
        return;
    }
    let content = lines(ctx);
    let rows = inner.height.saturating_sub(2) as usize;
    let max_scroll = content.len().saturating_sub(rows);
    *scroll = (*scroll).min(max_scroll);
    let width = inner.width.saturating_sub(2) as usize;
    for (i, (text, style)) in content.iter().skip(*scroll).take(rows).enumerate() {
        let t = draw::truncate(text, width);
        draw::text(
            buf,
            inner.x + 1,
            inner.y + i as u16,
            &t,
            *style,
            inner.right(),
        );
    }
    let more = if max_scroll > 0 {
        format!(
            "  ({}/{})",
            *scroll + rows.min(content.len()),
            content.len()
        )
    } else {
        String::new()
    };
    let footer = format!("[↑↓] défiler   [I] / [Échap] fermer{more}");
    let fw = footer.chars().count() as u16;
    let x = inner.x + inner.width.saturating_sub(fw) / 2;
    draw::text(
        buf,
        x,
        inner.bottom() - 1,
        &footer,
        theme::fg(theme::GREY),
        inner.right(),
    );
}
