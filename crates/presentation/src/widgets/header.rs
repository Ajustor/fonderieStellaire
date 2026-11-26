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
    if inner.height < 3 || inner.width < 20 {
        return;
    }
    let snap = ctx.snap;
    let now = ctx.now;

    // Progression globale, sur la bordure haute à droite.
    let progress = format!(
        " progression {} [I] ",
        fmt::percent(snap.progress.completion)
    );
    draw::text_right(
        buf,
        area.right().saturating_sub(2),
        area.y,
        &progress,
        theme::fg(theme::GOLD),
        area.x + TITLE.chars().count() as u16 + 3,
    );

    // --- Colonne de droite (prod, tempête, supernova, fragments) ---
    let right = inner.right().saturating_sub(1);
    // Début du texte de droite sur chacune des 3 lignes.
    let mut row_limit = [right; 3];
    let prod = format!("+{} /s", fmt::short(snap.production));
    if snap.storm_remaining > 0.0 {
        let beat = (now * std::f64::consts::PI * 2.0).sin().abs();
        let c = theme::ORANGE.lerp(theme::WHITE, 0.35 * beat);
        let storm = format!("×7 TEMPÊTE {}", fmt::countdown(snap.storm_remaining));
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
    let auto = if snap.autobuy { "▲ auto   " } else { "" };
    let free = if snap.fragments_spendable >= 1.0 {
        format!(" · {} libres [E]", fmt::short(snap.fragments_spendable))
    } else {
        String::new()
    };
    let frag = format!(
        "{auto}◆ {} (+{}){free}   ★ {}/{}",
        fmt::short(snap.fragments),
        fmt::percent(snap.fragment_bonus),
        snap.achievements_unlocked,
        snap.achievements_total
    );
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
    let lines: [(&str, Style); 3] = [
        (&click, theme::fg(theme::MUTED)),
        (unit_word, theme::bold(theme::GOLD)),
        (
            if is_scale {
                "de poussières d'étoile"
            } else {
                ""
            },
            Style::default().fg(theme::MUTED.color()),
        ),
    ];
    for (row, (text, style)) in lines.iter().enumerate() {
        let limit = row_limit[row].saturating_sub(2);
        if text.is_empty() || wx + 6 > limit {
            continue;
        }
        let shown = draw::truncate(text, (limit - wx) as usize);
        draw::text(buf, wx, inner.y + row as u16, &shown, *style, limit);
    }
}
