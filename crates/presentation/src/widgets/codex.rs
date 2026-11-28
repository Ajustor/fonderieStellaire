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
/// Détail de la production : chaque multiplicateur global et le total.
pub fn production_lines(ctx: &Ctx) -> Vec<(String, Style)> {
    let snap = ctx.snap;
    let mut out = vec![(
        format!(
            "PRODUCTION  ×{} au total",
            fmt::short(snap.permanent_multiplier * snap.storm_multiplier)
        ),
        theme::bold(theme::GOLD),
    )];
    for (name, factor) in &snap.production_sources {
        out.push((
            format!(" ×{:<8} {name}", fmt::short(*factor)),
            theme::fg(if *factor < 1.0 {
                theme::ORANGE
            } else {
                theme::TEXT
            }),
        ));
    }
    if snap.storm_multiplier > 1.0 {
        out.push((
            format!(
                " ×{:<8} Tempête solaire (temporaire)",
                fmt::short(snap.storm_multiplier)
            ),
            theme::fg(theme::CYAN),
        ));
    }
    if snap.production_sources.is_empty() && snap.storm_multiplier <= 1.0 {
        out.push((
            " Aucun bonus pour l'instant.".into(),
            theme::fg(theme::GREY),
        ));
    }
    out.push((
        " Hors améliorations propres à chaque générateur et paliers (×2 à 25, 50, 100…).".into(),
        theme::fg(theme::MUTED),
    ));
    out
}

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
        format!(" {}{}", "█".repeat(filled), "░".repeat(bar_w - filled),),
        theme::fg(theme::GOLD),
    ));
    out.push((
        format!(
            " ● Générateurs {}/{} · succès {}/{} · Atelier {}/{}",
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
            " ▲ Améliorations ce cycle : {}/{} · Supernovas : {}",
            pr.upgrades_bought, pr.upgrades_total, snap.supernovas,
        ),
        text,
    ));
    out.push((
        format!(
            " ★ Constellations : {} niv. · Temps de jeu : {}",
            snap.constellation_levels,
            fmt::duration(snap.play_time)
        ),
        text,
    ));
    out.push((String::new(), Style::default()));

    out.extend(stats_lines(ctx));
    out.push((String::new(), Style::default()));

    out.extend(production_lines(ctx));
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
        "◆ Défis [D] : un cycle avec une contrainte, contre une récompense permanente.".into(),
        "● Comètes rares : dorée (×77 pendant 7 s), noire (+1 fragment, ou elle te vole).".into(),
        "✶ Big Crunch [K] (15 Supernovas) : tout le niveau Supernova repart de zéro contre de la matière noire : production ×2, ×4, ×8… et fragments +25 % par Crunch.".into(),
        "✶ Laboratoire [L] (10 Big Crunch) : recherches en temps réel, une à la fois, qui avancent même jeu fermé : automatismes, conservation, puissance.".into(),
        "◎ Univers parallèles [V] (10 Big Crunch) : choisis les règles de chaque cycle au Big Crunch ; atteins leur objectif pour gagner leur Clé (bonus permanent).".into(),
        "✷ Big Bang [G] (30 Big Crunch) : les Big Crunch repartent de zéro contre des particules primordiales (production ×10 et +25 % de fragments chacune). Le Laboratoire, les Clés et le Grand Attracteur restent.".into(),
        "▼ Forge primordiale [G] : des particules pour forger 3 générateurs de fin de jeu, présents dans tous les cycles.".into(),
        "◆ Grand Attracteur [G] : 7 étapes payées en particules et construites en temps réel, même jeu fermé. La dernière clôt l'Acte II.".into(),
        "▸ Carte stellaire [X] (Acte III, après l'éveil du Grand Attracteur) : envoie des flottes explorer des systèmes, en temps réel. Plus un système est dangereux, plus l'expédition est longue et risquée, et plus elle rapporte : production, fragments, hors-ligne, comètes, flottes en plus ou un gros gain immédiat.".into(),
        "◆ Reliques [R] : rapportées par les expéditions (toujours en prospection, vers un système déjà exploré). 16 à collectionner, 4 raretés ; équipe-les (3 à 5 emplacements) et réunis 3 reliques d'un même ensemble pour son bonus. Un doublon devient un Éclat (+1 % de production).".into(),
        "✹ Événements cosmiques (Acte III) : toutes les 2 à 4 h de jeu, un dilemme. Certains choix sont risqués. Tu as 1 h pour répondre ([Z] pour y revenir).".into(),
        "✶ Convergence : carte complète et 16 reliques réunies, lance-la depuis la Fonderie sur la carte [X]. 48 h plus tard, c'est la fin de l'Acte III.".into(),
        "◈ Mégastructures [B] (Acte IV, après la Convergence) : la matière exotique s'accumule en temps réel, même jeu fermé ; bâtis sur une grille de 9×5. Les voisines comptent : Relais, Forges-mondes près des Collecteurs, Archives près des Forges, Phares au bord, Cœur entouré de variété.".into(),
        "✧ Défi du jour [J] (Acte IV) : dix minutes dans un univers de poche, avec deux règles tirées de la date (les mêmes pour tous). Le premier défi du jour rapporte de la matière exotique ; le code de score se partage.".into(),
        "○ Hors-ligne : ta fonderie produit à 50 % pendant ton absence (12 h max).".into(),
        "· Raccourcis : [A] tout acheter · [T] automate · [PgUp]/[PgDn] journal.".into(),
        "· Sauvegarde automatique toutes les 30 s et en quittant.".into(),
    ];
    for m in mechanics {
        out.push((format!(" {m}"), text));
    }
    out
}

/// Statistiques : courbe de la production sur toute la partie (échelle
/// logarithmique), record, et temps de jeu des grandes étapes.
fn stats_lines(ctx: &Ctx) -> Vec<(String, Style)> {
    let snap = ctx.snap;
    let mut out: Vec<(String, Style)> = vec![("STATISTIQUES".into(), theme::bold(theme::GOLD))];
    let h = &snap.history_long;
    const COLS: usize = 56;
    const ROWS: usize = 5;
    const BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    if h.len() >= 2 {
        // Une colonne = le maximum de sa tranche d'historique.
        let cols: Vec<f32> = (0..COLS)
            .map(|c| {
                let a = c * h.len() / COLS;
                let b = ((c + 1) * h.len() / COLS).max(a + 1).min(h.len());
                h[a..b].iter().map(|&(_, p)| p).fold(f32::MIN, f32::max)
            })
            .collect();
        let lo = cols.iter().copied().fold(f32::MAX, f32::min);
        let hi = cols.iter().copied().fold(f32::MIN, f32::max).max(lo + 1.0);
        for row in (0..ROWS).rev() {
            let line: String = cols
                .iter()
                .map(|&v| {
                    let level =
                        ((v - lo) / (hi - lo) * (ROWS * 8) as f32) as i32 - (row * 8) as i32;
                    BLOCKS[level.clamp(0, 8) as usize]
                })
                .collect();
            let label = if row == ROWS - 1 {
                format!(" 1e{}/s", hi.round() as i32)
            } else if row == 0 {
                format!(" 1e{}/s", lo.round() as i32)
            } else {
                String::new()
            };
            let c = theme::CYAN.lerp(theme::GOLD, row as f64 / ROWS as f64);
            out.push((format!(" {line}{label}"), theme::fg(c)));
        }
        let span = h.last().map(|l| l.0).unwrap_or(0.0) - h[0].0;
        out.push((
            format!(
                " Production sur {} de jeu (échelle logarithmique)",
                fmt::duration(span.max(0.0))
            ),
            theme::fg(theme::MUTED),
        ));
    } else {
        out.push((
            " La courbe de production se dessine au fil des heures de jeu.".into(),
            theme::fg(theme::MUTED),
        ));
    }
    out.push((
        format!(
            " ▲ Record de production : {}/s",
            fmt::short(snap.best_production)
        ),
        theme::fg(theme::TEXT),
    ));
    let names = [
        "1re Supernova",
        "1er Big Crunch",
        "1er Big Bang",
        "Fin de l'Acte II",
        "Fin de l'Acte III",
        "Fin de l'Acte IV",
    ];
    let reached: Vec<String> = names
        .iter()
        .zip(snap.milestones.iter())
        .filter(|(_, &t)| t > 0.0)
        .map(|(n, &t)| format!("{n} : {}", fmt::duration(t)))
        .collect();
    if !reached.is_empty() {
        out.push((
            format!(" ◆ Étapes (temps de jeu) : {}", reached.join(" · ")),
            theme::fg(theme::TEXT),
        ));
    }
    out
}

/// Renvoie un texte à la ligne ; les lignes suivantes sont alignées sous le
/// texte qui suit le symbole de tête (« ★ Nom — … »).
fn hanging_wrap(text: &str, width: usize) -> Vec<String> {
    let lead = text.chars().take_while(|c| *c == ' ').count();
    let rest: Vec<char> = text.chars().skip(lead).collect();
    let indent = if rest.len() > 2 && rest[1] == ' ' && !rest[0].is_alphanumeric() {
        lead + 2
    } else {
        lead
    };
    let first = draw::wrap(text, width);
    if first.len() <= 1 {
        return first;
    }
    let head = first[0].clone();
    let tail = text.chars().skip(head.chars().count()).collect::<String>();
    let pad = " ".repeat(indent);
    let mut out = vec![head];
    for l in draw::wrap(tail.trim_start(), width.saturating_sub(indent).max(10)) {
        out.push(format!("{pad}{l}"));
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
    let width = inner.width.saturating_sub(2) as usize;
    // Renvoi à la ligne avec retrait sous le texte (après le symbole).
    let content: Vec<(String, Style)> = lines(ctx)
        .into_iter()
        .flat_map(|(text, style)| {
            hanging_wrap(&text, width)
                .into_iter()
                .map(move |l| (l, style))
        })
        .collect();
    let rows = inner.height.saturating_sub(2) as usize;
    let max_scroll = content.len().saturating_sub(rows);
    *scroll = (*scroll).min(max_scroll);
    for (i, (text, style)) in content.iter().skip(*scroll).take(rows).enumerate() {
        draw::text(
            buf,
            inner.x + 1,
            inner.y + i as u16,
            text,
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
