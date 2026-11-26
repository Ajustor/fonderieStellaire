//! En-tête : gros compteur animé, production, tempête, fragments.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;

use super::Ctx;
use crate::bigfont;
use crate::draw;
use crate::fmt;
use crate::fx::easing::ease_out_cubic;
use crate::fx::Rgb;
use crate::theme;
use crate::visuals::Visuals;

pub const TITLE: &str = " ✦ FONDERIE STELLAIRE ✦ ";

/// Écrit un titre de panneau en dégradé animé à partir de `x`.
pub fn gradient_title(buf: &mut Buffer, x: u16, y: u16, text: &str, now: f64, max_x: u16) {
    for (i, ch) in text.chars().enumerate() {
        let cx = x + i as u16;
        if cx >= max_x {
            break;
        }
        let c = theme::logo_color(i as f64 * 2.0, now);
        draw::put_styled(buf, buf.area, cx as i32, y as i32, ch, theme::bold(c));
    }
}

pub fn render(buf: &mut Buffer, area: Rect, ctx: &Ctx, vis: &mut Visuals) {
    let inner = draw::panel(buf, area, Line::default(), false);
    gradient_title(
        buf,
        area.x + 2,
        area.y,
        TITLE,
        ctx.now,
        area.right().saturating_sub(1),
    );
    // Badge de l'univers en cours (hors univers d'origine), qui scintille.
    let mut badge_w = 0u16;
    if let Some(def) = ctx
        .catalog
        .universe(ctx.snap.universe)
        .filter(|u| u.key_reward.is_some())
    {
        let color =
            theme::universe_color(def.id.index()).glow(0.2 * (0.5 + 0.5 * (ctx.now * 2.5).sin()));
        let badge = format!(" {} {} ", def.symbol, def.name);
        badge_w = badge.chars().count() as u16;
        draw::text(
            buf,
            area.x + 2 + TITLE.chars().count() as u16,
            area.y,
            &badge,
            theme::bold(color),
            area.right().saturating_sub(2),
        );
    }
    if inner.height < 3 || inner.width < 20 {
        return;
    }
    let snap = ctx.snap;
    let now = ctx.now;

    // Progression globale, sur la bordure haute à droite.
    // Pendant un Défi, la bordure montre son avancement à la place.
    let (progress, progress_color) = match snap.active_challenge {
        Some(a) => {
            let name = ctx.catalog.challenge(a.id).map(|c| c.name).unwrap_or("?");
            let time = a
                .time_left
                .map(|t| format!(" · {}", fmt::countdown(t)))
                .unwrap_or_default();
            (
                format!(" ◆ DÉFI {name} {}{time} [D] ", fmt::percent(a.progress)),
                theme::PINK,
            )
        }
        None => {
            // Chantiers en cours (Laboratoire, Grand Attracteur) devant la
            // progression : du plus détaillé au plus court, selon la place.
            let research = snap.research.and_then(|r| {
                ctx.catalog
                    .lab_node(r.id)
                    .map(|d| (d.name, fmt::duration(r.remaining)))
            });
            let build = snap.attractor_build.and_then(|b| {
                ctx.catalog
                    .attractor_stage(b.stage)
                    .map(|s| (s.name, fmt::duration(b.remaining)))
            });
            let fleets = snap
                .fleets
                .iter()
                .map(|f| f.remaining)
                .fold(None, |a: Option<f64>, r| Some(a.map_or(r, |a| a.min(r))))
                .map(|next| (snap.fleets.len(), fmt::duration(next)));
            let boost = snap
                .boosts
                .iter()
                .filter(|b| b.kind == domain::BoostKind::Production)
                .fold(None, |a: Option<(f64, f64)>, b| {
                    Some(a.map_or((b.factor, b.left), |(f, l)| (f * b.factor, l.min(b.left))))
                })
                .map(|(f, l)| format!(" ✹ ×{} {} ·", fmt::short(f), fmt::duration(l)))
                .unwrap_or_default();
            let event = if snap.pending_event.is_some() {
                " ✹ événement [Z] ·"
            } else {
                ""
            };
            let converge = snap
                .convergence
                .map(|(l, _)| format!(" ✶ Convergence {} ·", fmt::duration(l)))
                .unwrap_or_default();
            let pct = fmt::percent(snap.progress.completion);
            let room = area
                .width
                .saturating_sub(TITLE.chars().count() as u16 + 6)
                .saturating_sub(badge_w) as usize;
            let mut variants = Vec::new();
            let full = |named: bool| {
                let mut s = String::new();
                if let Some((n, d)) = &research {
                    if named {
                        s.push_str(&format!(" ✶ {n} {d} [L] ·"));
                    } else {
                        s.push_str(&format!(" ✶ {d} ·"));
                    }
                }
                if let Some((n, d)) = &build {
                    if named {
                        s.push_str(&format!(" ◇ {n} {d} [G] ·"));
                    } else {
                        s.push_str(&format!(" ◇ {d} ·"));
                    }
                }
                if let Some((n, d)) = &fleets {
                    if named {
                        s.push_str(&format!(" ▸ {n} flotte(s) {d} [X] ·"));
                    } else {
                        s.push_str(&format!(" ▸ {d} ·"));
                    }
                }
                s
            };
            variants.push(format!(
                "{event}{converge}{boost}{} progression {pct} [I] ",
                full(true)
            ));
            variants.push(format!(
                "{event}{converge}{boost}{} progression {pct} [I] ",
                full(false)
            ));
            variants.push(format!("{event}{boost}{} {pct} ", full(false)));
            variants.push(format!("{event} {pct} "));
            variants.push(format!(" {pct} "));
            let text = variants
                .iter()
                .find(|v| v.chars().count() <= room)
                .cloned()
                .unwrap_or_default();
            (text, theme::GOLD)
        }
    };
    draw::text_right(
        buf,
        area.right().saturating_sub(2),
        area.y,
        &progress,
        theme::fg(progress_color),
        area.x + TITLE.chars().count() as u16 + 3,
    );

    // --- Colonne de droite (prod, tempête, supernova, fragments) ---
    let right = inner.right().saturating_sub(1);
    // Début du texte de droite sur chacune des 3 lignes.
    let mut row_limit = [right; 3];
    let prod = format!("+{} /s", fmt::short(snap.production));
    if snap.storm_remaining > 0.0 {
        let beat = (now * std::f64::consts::PI * 2.0).sin().abs();
        let golden = snap.storm_multiplier >= 50.0;
        let base = if golden { theme::GOLD } else { theme::ORANGE };
        let c = base.lerp(theme::WHITE, 0.35 * beat);
        let label = if golden { "FRÉNÉSIE" } else { "TEMPÊTE" };
        let storm = format!(
            "×{} {label} {}",
            fmt::short(snap.storm_multiplier),
            fmt::countdown(snap.storm_remaining)
        );
        let x = draw::text_right(buf, right, inner.y, &storm, theme::bold(c), inner.x);
        let x = draw::text_right(
            buf,
            x.saturating_sub(3),
            inner.y,
            &prod,
            theme::bold(theme::GREEN),
            inner.x,
        );
        row_limit[0] = x;
    } else {
        let x = draw::text_right(
            buf,
            right,
            inner.y,
            &prod,
            theme::bold(theme::GREEN),
            inner.x,
        );
        row_limit[0] = x;
    }
    let n = snap.fragments_available;
    let nova = format!(
        "Supernova : +{} fragment{}",
        fmt::short(n),
        if n >= 2.0 { "s" } else { "" }
    );
    let nova_color = if n >= 1.0 {
        theme::PINK.lerp(theme::WHITE, 0.25 * (0.5 + 0.5 * (now * 3.0).sin()))
    } else {
        theme::GREY
    };
    let x = draw::text_right(
        buf,
        right,
        inner.y + 1,
        &nova,
        theme::fg(nova_color),
        inner.x,
    );
    row_limit[1] = x;
    // Ressources de prestige : de la version la plus détaillée à la plus
    // compacte, selon la place laissée au gros compteur (jamais coupée).
    let frag = {
        // level 0 : tout · 1 : sans les « libres » · 2 : sans les bonus · 3 : minimal
        let line = |level: u8| {
            let sep = if level >= 2 { "  " } else { "   " };
            let mut s = String::new();
            if snap.particles > 0 {
                s.push_str(&format!(
                    "{} {}",
                    crate::widgets::cosmos::PARTICLE,
                    snap.particles
                ));
                if level < 2 {
                    s.push_str(&format!(" (×{})", fmt::short(snap.particle_production)));
                }
                s.push_str(sep);
            }
            if snap.crunches > 0 {
                s.push_str(&format!("✶ {}", snap.crunches));
                if level < 2 {
                    s.push_str(&format!(" (×{})", fmt::short(snap.crunch_production)));
                }
                if level == 0 && snap.lab_unlocked && snap.dark_matter_spendable > 0 {
                    s.push_str(&format!(" · {} libres [L]", snap.dark_matter_spendable));
                }
                s.push_str(sep);
            }
            if snap.autobuy && level < 3 {
                s.push_str("▲ auto");
                s.push_str(sep);
            }
            s.push_str(&format!("◆ {}", fmt::short(snap.fragments)));
            if level < 2 {
                s.push_str(&format!(" (+{})", fmt::percent(snap.fragment_bonus)));
            }
            if level == 0 && snap.fragments_spendable >= 1.0 {
                s.push_str(&format!(
                    " · {} libres [E]",
                    fmt::short(snap.fragments_spendable)
                ));
            }
            s.push_str(&format!(
                "{sep}★ {}/{}",
                snap.achievements_unlocked, snap.achievements_total
            ));
            s
        };
        // Place réservée au gros compteur et à son mot d'échelle.
        let (digits, _) = fmt::big(snap.stardust);
        let counter = bigfont::text_width(&digits) + 16;
        let room = inner.width.saturating_sub(counter + 2) as usize;
        (0..4)
            .map(line)
            .find(|l| l.chars().count() <= room)
            .unwrap_or_else(|| line(3))
    };
    let x = draw::text_right(
        buf,
        right,
        inner.y + 2,
        &frag,
        theme::fg(theme::CYAN),
        inner.x,
    );
    row_limit[2] = x;
    let right_min = row_limit.iter().copied().min().unwrap_or(right);

    // --- Gros compteur ---
    let shown = vis.counter_value(now, snap.stardust);
    let (digits, word) = fmt::big(shown);
    vis.track_digits(&digits, now);
    let x0 = inner.x + 1;
    let total_chars = digits.chars().count();
    let bump = ((now - vis.bump_at) / 0.15).clamp(0.0, 1.0);
    let bump_glow = if bump < 1.0 {
        0.65 * (1.0 - ease_out_cubic(bump))
    } else {
        0.0
    };
    let changed = &vis.digit_changed;
    let number_w = bigfont::text_width(&digits);
    let max_x = right_min.saturating_sub(2);
    bigfont::layout(&digits, |dx, dy, ci, ch| {
        let x = x0 + dx;
        if x >= max_x {
            return;
        }
        let mut c: Rgb = theme::logo_color(dx as f64, now);
        let from_right = total_chars - 1 - ci;
        if let Some(&t) = changed.get(from_right) {
            let p = (now - t) / 0.25;
            if (0.0..1.0).contains(&p) {
                c = c.glow(0.45 * (1.0 - p));
            }
        }
        c = c.glow(bump_glow);
        draw::put(buf, inner, x as i32, (inner.y + dy) as i32, ch, c);
    });

    // --- Mot d'unité et valeur d'un clic ---
    // Chaque ligne a sa propre limite : une ligne de droite longue (fragments)
    // ne fait plus disparaître les deux autres.
    let wx = x0 + number_w + 2;
    let unit_word = word.as_str();
    let is_scale = !unit_word.starts_with("poussi");
    let click = format!("clic : +{}", fmt::short(snap.click_value));
    // Chaque ligne propose des variantes, de la plus complète à la plus courte :
    // on prend la première qui tient en entier, sans jamais couper un mot.
    let unit_short = if is_scale { "de poussière" } else { "" };
    let unit_long = if is_scale {
        "de poussières d'étoile"
    } else {
        ""
    };
    let click_short = format!("+{}", fmt::short(snap.click_value));
    let lines: [(Vec<&str>, Style); 3] = [
        (vec![&click, &click_short], theme::fg(theme::MUTED)),
        (vec![unit_word], theme::bold(theme::GOLD)),
        (
            vec![unit_long, unit_short],
            Style::default().fg(theme::MUTED.color()),
        ),
    ];
    for (row, (variants, style)) in lines.iter().enumerate() {
        let limit = row_limit[row].saturating_sub(2);
        let room = limit.saturating_sub(wx) as usize;
        if let Some(text) = variants
            .iter()
            .find(|t| !t.is_empty() && t.chars().count() <= room)
        {
            draw::text(buf, wx, inner.y + row as u16, text, *style, limit);
        }
    }
}
