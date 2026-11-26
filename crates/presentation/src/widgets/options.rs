//! Options (touche R) : images par seconde, effets réduits, emplacement de la
//! sauvegarde, et « Recommencer une nouvelle partie » (double confirmation).
//! Plus l'animation d'effacement qui suit la confirmation.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Widget};

use super::Ctx;
use crate::draw;
use crate::fx::easing::ease_out_cubic;
use crate::fx::effects;
use crate::fx::rng::hash3;
use crate::fx::Rgb;
use crate::layout::centered;
use crate::theme;
use crate::ui::Popup;

/// Nombre de lignes réglables.
pub const ITEMS: usize = 3;
/// Durée de l'effacement ; la nouvelle partie commence à la fin.
pub const WIPE_DURATION: f64 = 2.6;
const EMBER: Rgb = Rgb(255, 110, 60);

/// Informations affichées sous les réglages.
pub struct OptionsInfo<'a> {
    pub save_path: &'a str,
    pub version: &'a str,
}

pub fn render(
    buf: &mut Buffer,
    area: Rect,
    ctx: &Ctx,
    popup: &Popup,
    selected: usize,
    armed: bool,
    info: &OptionsInfo,
) {
    let s = ease_out_cubic(popup.scale(ctx.now));
    if s <= 0.0 {
        return;
    }
    let snap = ctx.snap;
    effects::fade(buf, area, 0.6 * s);
    let width = area.width.saturating_sub(4).min(80);
    let text_w = width.saturating_sub(4) as usize;

    let fps = if snap.frame_rate == 0 {
        30
    } else {
        snap.frame_rate
    };
    let rows: [(&str, String, Rgb); ITEMS] = [
        ("Images par seconde", format!("{fps}"), theme::CYAN),
        (
            "Effets réduits",
            if snap.reduced_effects { "oui" } else { "non" }.to_string(),
            theme::CYAN,
        ),
        (
            "Recommencer une nouvelle partie",
            "tout effacer".to_string(),
            theme::RED,
        ),
    ];
    let help = match selected {
        0 => "60, 30, 20 ou 15 images par seconde. 60 rend les animations très fluides si ton terminal suit ; baisse-les s'il saccade : le jeu tourne pareil, seules les animations changent.".to_string(),
        1 => "Moins de secousses, de flashs et de nébuleuses animées. Raccourci : [F].".to_string(),
        _ if armed => "ATTENTION : Entrée de nouveau pour TOUT effacer (fragments, Big Crunch, Laboratoire, Clés, particules, succès). C'est définitif. [Échap] pour annuler.".to_string(),
        _ => "Efface ta sauvegarde et repart de zéro. Pour en garder une copie avant : fonderie-stellaire --export fichier".to_string(),
    };
    let mut detail: Vec<(String, Rgb)> = draw::wrap(&help, text_w)
        .into_iter()
        .map(|l| {
            (
                l,
                if selected == 2 && armed {
                    theme::ORANGE
                } else {
                    theme::MUTED
                },
            )
        })
        .collect();
    // Hauteur fixe : celle du texte d'aide le plus long.
    let detail_h = 4.max(detail.len());
    detail.resize(detail_h, (String::new(), theme::MUTED));
    let mut infos = draw::wrap(&format!("Sauvegarde : {}", info.save_path), text_w);
    infos.extend(draw::wrap(
        &format!(
            "Version {} · mises à jour automatiques au lancement",
            info.version
        ),
        text_w,
    ));
    infos.extend(draw::wrap(
        "Changer d'ordinateur : --export fichier, puis --import fichier sur l'autre.",
        text_w,
    ));

    // bordures + réglages + blanc + séparateur + aide + blanc + infos + pied
    let height = (2 + ITEMS + 1 + 1 + detail_h + 1 + infos.len() + 1) as u16;
    let height = height.min(area.height.saturating_sub(2));
    let w = ((width as f64 * s).round() as u16).max(2);
    let h = ((height as f64 * s).round() as u16).max(2);
    let rect = centered(area, w, h);
    Clear.render(rect, buf);
    let border = if selected == 2 && armed {
        theme::RED.lerp(EMBER, 0.5 + 0.5 * (ctx.now * 6.0).sin())
    } else {
        theme::CYAN
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::bold(border))
        .title(Line::from(Span::styled(" ⚙ OPTIONS ", theme::bold(theme::GOLD))).centered());
    let inner = block.inner(rect);
    block.render(rect, buf);
    if s < 0.95 || inner.height < 6 {
        return;
    }
    let right = inner.right();
    let mut y = inner.y;
    for (i, (name, value, color)) in rows.iter().enumerate() {
        let is_sel = i == selected;
        if is_sel {
            draw::fill_bg(buf, Rect::new(inner.x, y, inner.width, 1), theme::SELECT_BG);
        }
        let x = draw::text(
            buf,
            inner.x + 1,
            y,
            if is_sel { "▶ " } else { "  " },
            theme::bold(theme::FOCUS),
            right,
        );
        let name_color = if i == 2 { theme::RED } else { theme::TEXT };
        let vx = right.saturating_sub(1 + value.chars().count() as u16);
        draw::text(
            buf,
            x,
            y,
            name,
            theme::bold(name_color),
            vx.saturating_sub(1),
        );
        draw::text_right(buf, right - 1, y, value, theme::bold(*color), inner.x);
        y += 1;
    }
    y += 1;
    let sep: String = "─".repeat(inner.width.saturating_sub(2) as usize);
    draw::text(buf, inner.x + 1, y, &sep, theme::fg(theme::BORDER), right);
    y += 1;
    for (l, c) in &detail {
        draw::text(buf, inner.x + 1, y, l, theme::fg(*c), right);
        y += 1;
    }
    y += 1;
    for l in &infos {
        if y >= inner.bottom().saturating_sub(1) {
            draw::audit::record(l, 0);
            continue;
        }
        draw::text(buf, inner.x + 1, y, l, theme::fg(theme::GREY), right);
        y += 1;
    }
    let long = "[↑↓] choisir   [Entrée] changer   [R] / [Échap] fermer";
    let footer = if long.chars().count() <= inner.width as usize {
        long
    } else {
        "[↑↓] [Entrée] [Échap]"
    };
    let fw = footer.chars().count() as u16;
    draw::text(
        buf,
        inner.x + inner.width.saturating_sub(fw) / 2,
        inner.bottom() - 1,
        footer,
        theme::fg(theme::GREY),
        right,
    );
}

/// Effacement : l'écran se consume de haut en bas, un front de braises le
/// parcourt, puis il ne reste que le noir et une phrase.
pub fn render_wipe(buf: &mut Buffer, area: Rect, now: f64, start: f64) {
    let t = (now - start).max(0.0);
    let p = (t / (WIPE_DURATION - 0.6)).clamp(0.0, 1.0);
    let h = area.height.max(1) as f64;
    for row in 0..area.height {
        let y = area.y + row;
        // Le front descend ; chaque ligne se dissout quand il la dépasse.
        let local = ((p * 1.35 - row as f64 / h) * 3.0).clamp(0.0, 1.0);
        if local <= 0.0 {
            continue;
        }
        let line = Rect::new(area.x, y, area.width, 1);
        effects::dissolve(buf, line, local, 31 + row as i64);
        effects::fade(buf, line, local);
        // Braises sur le front.
        if local < 0.9 {
            for x in area.left()..area.right() {
                let k = hash3(x as i64, row as i64, (now * 12.0) as i64);
                if k < 0.12 * (1.0 - local) {
                    let c = EMBER.lerp(theme::GOLD, k * 6.0).dim(local * 0.5);
                    draw::put(buf, area, x as i32, y as i32, '▀', c);
                }
            }
        }
    }
    if t > WIPE_DURATION - 1.2 {
        let a = ((t - (WIPE_DURATION - 1.2)) / 0.5).clamp(0.0, 1.0);
        let text = "Tout s'efface… une nouvelle forge t'attend.";
        let w = text.chars().count() as u16;
        let x = area.x + area.width.saturating_sub(w) / 2;
        draw::text(
            buf,
            x,
            area.y + area.height / 2,
            text,
            theme::bold(theme::MUTED.dim(1.0 - a)),
            area.right(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wipe_consumes_the_screen() {
        let area = Rect::new(0, 0, 60, 20);
        let mut buf = Buffer::empty(area);
        for y in 0..20 {
            draw::text(&mut buf, 0, y, &"#".repeat(60), theme::fg(theme::TEXT), 60);
        }
        render_wipe(&mut buf, area, WIPE_DURATION, 0.0);
        let text: String = (0..20)
            .map(|y| {
                (0..60)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect();
        assert!(!text.contains('#'), "tout est effacé");
        assert!(text.contains("une nouvelle forge"));
    }
}
