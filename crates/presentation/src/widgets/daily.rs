//! Défi du jour (touche J, Acte IV) : la fenêtre du défi, la transition vers
//! l'univers de poche (un iris qui s'ouvre), et la carte du résultat.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Widget};

use super::Ctx;
use crate::draw;
use crate::fmt;
use crate::fx::easing::{ease_in_cubic, ease_out_back, ease_out_cubic};
use crate::fx::effects;
use crate::fx::Rgb;
use crate::layout::centered;
use crate::theme;
use crate::ui::Popup;

pub const POCKET: Rgb = Rgb(110, 230, 255);
/// Durée de la transition et de la carte du résultat.
pub const IRIS: f64 = 1.2;
pub const RESULT: f64 = 6.0;

/// Date du jour (jours depuis 1970) : « 26/11/2026 ».
pub fn date_of(day: u64) -> String {
    // Algorithme « civil from days » (calendrier grégorien proleptique).
    let z = day as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{d:02}/{m:02}/{y}")
}

pub(crate) fn lines(ctx: &Ctx, width: usize, today: u64) -> Vec<(String, Style)> {
    let snap = ctx.snap;
    let mut out = Vec::new();
    let mut push = |t: String, s: Style| {
        for l in draw::wrap(&t, width) {
            out.push((l, s));
        }
    };
    push(
        format!("Défi du {}", date_of(today)),
        theme::bold(theme::GOLD),
    );
    push(String::new(), Style::default());
    push(
        "Règles du jour, les mêmes pour tous :".into(),
        theme::fg(theme::MUTED),
    );
    for r in &snap.daily_rules {
        push(format!("  ✧ {r}"), theme::bold(POCKET));
    }
    push(String::new(), Style::default());
    push(
        "Dix minutes dans un univers de poche, à partir de zéro : récolte le plus de poussière possible. Ta partie principale attend de côté et continue d'avancer en temps réel.".into(),
        theme::fg(theme::TEXT),
    );
    push(String::new(), Style::default());
    if snap.daily_best > 0.0 {
        push(
            format!("Record du jour : {} ✦", fmt::short(snap.daily_best)),
            theme::bold(theme::GOLD),
        );
    }
    let reward = if snap.daily_rewarded {
        "Récompense du jour déjà touchée : rejoue pour battre ton record.".to_string()
    } else {
        "Premier défi du jour : jusqu'à 12 ◈ de matière exotique selon ton score.".to_string()
    };
    push(reward, theme::fg(theme::CYAN));
    match snap.daily {
        Some(d) => push(
            format!(
                "Défi en cours : encore {} · score {} ✦ · [Entrée] terminer maintenant",
                fmt::countdown(d.left),
                fmt::short(d.score)
            ),
            theme::bold(theme::ORANGE),
        ),
        None => push(
            "[Entrée] entrer dans l'univers de poche".into(),
            theme::bold(theme::GOLD),
        ),
    }
    out
}

pub fn render(buf: &mut Buffer, area: Rect, ctx: &Ctx, popup: &Popup, today: u64) {
    let width = ctx.frame.width(area, 80);
    let content = lines(ctx, width.min(80).saturating_sub(4) as usize, today);
    let height = (content.len() as u16 + 3).min(area.height.saturating_sub(2));
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
        .border_style(theme::bold(
            POCKET.lerp(theme::GOLD, 0.5 + 0.5 * (ctx.now * 1.7).sin()),
        ))
        .title(Line::from(Span::styled(" ✧ DÉFI DU JOUR ✧ ", theme::bold(theme::GOLD))).centered());
    let inner = block.inner(rect);
    block.render(rect, buf);
    if !ready || inner.height < 4 {
        return;
    }
    let footer_y = inner.bottom() - 1;
    for (i, (l, st)) in content.iter().enumerate() {
        let y = inner.y + i as u16;
        if y >= footer_y {
            draw::audit::record(l, 0);
            continue;
        }
        draw::text(buf, inner.x + 1, y, l, *st, inner.right());
    }
    let footer = "[Entrée] valider   [J] / [Échap] fermer";
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

/// Transition : un iris rectangulaire s'ouvre depuis le centre sur l'univers
/// de poche (ou sur la partie principale, au retour).
pub fn render_iris(buf: &mut Buffer, area: Rect, now: f64, start: f64) {
    let t = (now - start) / IRIS;
    if !(0.0..1.0).contains(&t) {
        return;
    }
    let p = ease_out_cubic(t);
    let cx = area.x as f64 + area.width as f64 / 2.0;
    let cy = area.y as f64 + area.height as f64 / 2.0;
    let hw = p * (area.width as f64 / 2.0 + 2.0);
    let hh = p * (area.height as f64 / 2.0 + 1.0);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let dx = (x as f64 + 0.5 - cx).abs();
            let dy = (y as f64 + 0.5 - cy).abs();
            let outside = dx > hw || dy > hh;
            let edge = (dx - hw).abs() < 1.0 || (dy - hh).abs() < 0.6;
            if outside {
                if let Some(c) = buf.cell_mut((x, y)) {
                    c.reset();
                }
            } else if edge {
                draw::put(
                    buf,
                    area,
                    x as i32,
                    y as i32,
                    '█',
                    POCKET.lerp(theme::WHITE, 1.0 - t),
                );
            }
        }
    }
}

/// Résultat du défi.
#[derive(Clone, Debug, PartialEq)]
pub struct DailyResult {
    pub start: f64,
    pub score: f64,
    pub best: f64,
    pub reward: f64,
    pub code: String,
}

pub fn render_result(buf: &mut Buffer, area: Rect, now: f64, r: &DailyResult) {
    let t = now - r.start;
    if !(0.0..RESULT).contains(&t) {
        return;
    }
    let open = ease_out_back((t / 0.5).clamp(0.0, 1.0));
    let fade = if t > RESULT - 0.6 {
        ease_in_cubic((t - (RESULT - 0.6)) / 0.6)
    } else {
        0.0
    };
    effects::fade(buf, area, 0.5 * (1.0 - fade));
    let full_w = 50u16.min(area.width.saturating_sub(4));
    let w = ((full_w as f64 * open).round() as u16).clamp(2, full_w);
    let rect = centered(area, w, 10);
    Clear.render(rect, buf);
    draw::card(
        buf,
        area,
        rect,
        POCKET
            .lerp(theme::GOLD, 0.5 + 0.5 * (now * 3.0).sin())
            .dim(fade),
    );
    if open < 0.9 {
        return;
    }
    let record = r.score >= r.best && r.best > 0.0;
    let reward = if r.reward > 0.0 {
        format!("+{} ◈ de matière exotique", fmt::short(r.reward))
    } else {
        "Récompense du jour déjà touchée".to_string()
    };
    let shown_score = r.score * ease_out_cubic(((t - 0.4) / 1.2).clamp(0.0, 1.0));
    let rows: [(String, Rgb); 6] = [
        ("DÉFI DU JOUR TERMINÉ".into(), theme::MUTED),
        (
            format!("Score : {} ✦", fmt::short(shown_score)),
            theme::GOLD,
        ),
        (
            if record {
                "★ Record du jour !".to_string()
            } else {
                format!("Record du jour : {} ✦", fmt::short(r.best))
            },
            if record { theme::PINK } else { theme::TEXT },
        ),
        (reward, theme::CYAN),
        (String::new(), theme::TEXT),
        (format!("Code à partager : {}", r.code), POCKET),
    ];
    let inner_w = rect.width.saturating_sub(4) as usize;
    for (i, (text, c)) in rows.iter().enumerate() {
        let shown = draw::fit(text, inner_w);
        let tw = shown.chars().count() as u16;
        let x = rect.x + rect.width.saturating_sub(tw) / 2;
        draw::text(
            buf,
            x,
            rect.y + 2 + i as u16,
            &shown,
            theme::bold(c.dim(fade)),
            rect.right() - 1,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(date_of(0), "01/01/1970");
        assert_eq!(date_of(20_418), "26/11/2025");
        assert_eq!(date_of(11_016), "29/02/2000");
    }
}
