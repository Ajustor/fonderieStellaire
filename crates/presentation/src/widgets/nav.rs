//! Navigation du terminal : barre d'onglets (bordure basse de l'en-tête),
//! menu central (`Échap`) et panneau d'aide (`?`). Seul ce que le joueur a
//! découvert y figure.

use application::GameSnapshot;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Widget};

use super::Ctx;
use crate::draw;
use crate::fx::easing::ease_out_cubic;
use crate::fx::effects;
use crate::layout::centered;
use crate::nav::{self, Group, MenuEntry, ScreenId};
use crate::theme;
use crate::ui::Popup;

/// Libellé d'un onglet selon le niveau de détail (0 : nom et lettre,
/// 1 : nom court et lettre, 2 : nom court).
fn tab_label(s: ScreenId, level: u8) -> String {
    let name = if level == 0 { s.name() } else { s.short() };
    match s.key() {
        Some(k) if level < 2 => format!("{name} {k}"),
        _ => name.to_string(),
    }
}

/// Segments de la barre : (texte, écran ou séparateur).
fn tab_segments(screens: &[ScreenId], level: u8) -> Vec<(String, Option<ScreenId>)> {
    let mut out = Vec::new();
    let mut group: Option<Group> = None;
    for &s in screens {
        if let Some(g) = group {
            out.push((if g == s.group() { " · " } else { " │ " }.to_string(), None));
        }
        group = Some(s.group());
        out.push((tab_label(s, level), Some(s)));
    }
    out
}

fn width(segments: &[(String, Option<ScreenId>)]) -> usize {
    segments.iter().map(|(t, _)| t.chars().count()).sum()
}

/// Barre d'onglets dans la bordure basse de `header` (rien si un seul écran).
pub fn render_tabs(buf: &mut Buffer, header: Rect, snap: &GameSnapshot, current: ScreenId) {
    if header.height < 3 {
        return;
    }
    let line = Rect::new(header.x, header.bottom() - 1, header.width, 1);
    render_tabs_on(buf, line, snap, current);
}

/// Barre d'onglets sur une ligne de bordure (`line`, une ligne de haut).
pub fn render_tabs_on(buf: &mut Buffer, line: Rect, snap: &GameSnapshot, current: ScreenId) {
    let header = line;
    let screens = nav::available(snap);
    if screens.len() < 2 || header.width < 20 || header.height == 0 {
        return;
    }
    let y = header.y;
    let room = header.width.saturating_sub(6) as usize;
    let hint = " Tab › ";
    let segments = (0..3)
        .map(|level| tab_segments(&screens, level))
        .find(|s| width(s) + 2 <= room);
    let mut x = header.x + 2;
    let right = header.right().saturating_sub(2);
    let Some(segments) = segments else {
        // Trop étroit : l'écran courant seul, et comment passer au suivant.
        let label = format!(" {} ", current.name());
        x = draw::text(buf, x, y, &label, theme::bold(theme::GOLD), right);
        draw::text(buf, x, y, hint, theme::fg(theme::MUTED), right);
        return;
    };
    x = draw::text(buf, x, y, " ", theme::fg(theme::BORDER), right);
    for (text, screen) in &segments {
        let style = match screen {
            Some(s) if *s == current => theme::bold(theme::BLACK).bg(theme::GOLD.color()),
            Some(_) => theme::fg(theme::TEXT),
            None => theme::fg(theme::BORDER),
        };
        x = draw::text(buf, x, y, text, style, right);
    }
    draw::text(buf, x, y, " ", theme::fg(theme::BORDER), right);
}

/// Fenêtre centrée commune au menu et à l'aide ; renvoie l'intérieur.
fn frame(
    buf: &mut Buffer,
    area: Rect,
    ctx: &Ctx,
    popup: &Popup,
    title: &str,
    width: u16,
    height: u16,
) -> Option<Rect> {
    let s = ease_out_cubic(popup.scale(ctx.now));
    if s <= 0.0 {
        return None;
    }
    effects::fade(buf, area, 0.6 * s);
    let w = ((width.min(area.width.saturating_sub(2)) as f64 * s).round() as u16).max(2);
    let h = ((height.min(area.height.saturating_sub(2)) as f64 * s).round() as u16).max(2);
    let rect = centered(area, w, h);
    Clear.render(rect, buf);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::bold(theme::CYAN))
        .title(Line::from(Span::styled(format!(" {title} "), theme::bold(theme::GOLD))).centered());
    let inner = block.inner(rect);
    block.render(rect, buf);
    (s >= 0.95 && inner.height >= 3).then_some(inner)
}

/// Lettre et libellé d'une ligne du menu.
fn menu_row(e: MenuEntry) -> (String, &'static str, &'static str) {
    match e {
        MenuEntry::Screen(s) => (
            s.key().map(String::from).unwrap_or_default(),
            s.name(),
            s.blurb(),
        ),
        MenuEntry::Event => ("Z".into(), "Événement", "Un choix t'attend"),
        MenuEntry::Help => ("?".into(), "Aide", "Toutes les touches"),
        MenuEntry::Quit => ("Q".into(), "Quitter", "Sauvegarde et quitte"),
    }
}

pub fn render_menu(buf: &mut Buffer, area: Rect, ctx: &Ctx, popup: &Popup, sel: usize) {
    let entries = nav::menu_entries(ctx.snap);
    // Une ligne par entrée, plus un titre de groupe quand il change.
    let mut rows: Vec<(Option<usize>, String)> = Vec::new();
    let mut group: Option<&str> = None;
    for (i, e) in entries.iter().enumerate() {
        let g = match e {
            MenuEntry::Screen(s) => s.group().name(),
            _ => "",
        };
        if group != Some(g) {
            if group.is_some() {
                rows.push((None, String::new()));
            }
            if !g.is_empty() {
                rows.push((None, g.to_uppercase()));
            }
            group = Some(g);
        }
        rows.push((Some(i), String::new()));
    }
    let footer = "[↑↓] choisir   [Entrée] ou lettre : ouvrir   [Échap] fermer";
    let height = rows.len() as u16 + 4;
    let Some(inner) = frame(buf, area, ctx, popup, "MENU", 66, height) else {
        return;
    };
    let right = inner.right();
    let sel = sel.min(entries.len().saturating_sub(1));
    // Si la fenêtre est trop basse, on garde la ligne choisie visible.
    let list_h = inner.height.saturating_sub(2) as usize;
    let sel_row = rows.iter().position(|(i, _)| *i == Some(sel)).unwrap_or(0);
    let skip = (sel_row + 1).saturating_sub(list_h);
    for (k, (entry, title)) in rows.iter().enumerate().skip(skip).take(list_h) {
        let y = inner.y + (k - skip) as u16;
        let Some(i) = entry else {
            draw::text(buf, inner.x + 2, y, title, theme::bold(theme::MUTED), right);
            continue;
        };
        let (key, name, blurb) = menu_row(entries[*i]);
        let chosen = *i == sel;
        if chosen {
            draw::fill_bg(buf, Rect::new(inner.x, y, inner.width, 1), theme::SELECT_BG);
        }
        let x = draw::text(
            buf,
            inner.x + 2,
            y,
            &format!("{key:>1}"),
            theme::bold(theme::GOLD),
            right,
        );
        let name_style = if chosen {
            theme::bold(theme::GOLD)
        } else {
            theme::bold(theme::TEXT)
        };
        let x = draw::text(buf, x + 2, y, name, name_style, right);
        let bx = (inner.x + 22).max(x + 2);
        // Fenêtre étroite : l'explication se coupe proprement (« … »).
        let room = right.saturating_sub(bx + 1) as usize;
        let blurb = draw::truncate(blurb, room);
        draw::text(buf, bx, y, &blurb, theme::fg(theme::MUTED), right);
    }
    let fy = inner.bottom() - 1;
    let footer = if footer.chars().count() <= inner.width as usize - 2 {
        footer
    } else {
        "[↑↓] [Entrée] [Échap]"
    };
    draw::text(buf, inner.x + 1, fy, footer, theme::fg(theme::GREY), right);
}

/// Lignes du panneau d'aide : seulement ce qui est découvert.
pub fn help_lines(snap: &GameSnapshot) -> Vec<(String, Style)> {
    let mut out: Vec<(String, Style)> = Vec::new();
    let head = |out: &mut Vec<(String, Style)>, t: &str| {
        if !out.is_empty() {
            out.push((String::new(), Style::default()));
        }
        out.push((t.to_string(), theme::bold(theme::GOLD)));
    };
    let row = |out: &mut Vec<(String, Style)>, k: &str, what: &str| {
        out.push((format!(" {k:<12} {what}"), theme::fg(theme::TEXT)));
    };

    head(&mut out, "FONDERIE");
    row(
        &mut out,
        "Espace",
        if snap.hold_harvest {
            "Récolter (maintenir pour récolter en continu)"
        } else {
            "Récolter"
        },
    );
    row(&mut out, "↑ ↓", "Choisir un générateur / une amélioration");
    row(&mut out, "← →", "Générateurs ↔ améliorations");
    row(&mut out, "Entrée", "Acheter");
    row(&mut out, "1 – 9", "Achat rapide du générateur n° N");
    if snap.generators.iter().any(|g| g.owned > 0) {
        row(&mut out, "M", "Mode d'achat ×1 / ×10 / ×25 / PALIER / MAX");
    }
    if !snap.upgrades.is_empty() {
        row(&mut out, "A", "Acheter toutes les améliorations abordables");
    }
    if snap.meteors_caught > 0 || snap.comet_progress.is_some() {
        row(&mut out, "C", "Capturer la comète qui passe");
    }
    if snap.autobuy_unlocked {
        row(&mut out, "T", "Activer / arrêter l'automate");
    }
    row(
        &mut out,
        "PgUp PgDn",
        "Remonter / redescendre dans le journal",
    );

    let nova = snap.supernovas > 0 || snap.fragments_available >= 1.0;
    let crunch = snap.crunches > 0 || snap.crunch_available;
    if nova || crunch {
        head(&mut out, "PRESTIGE");
        if nova {
            row(&mut out, "P", "Supernova");
        }
        if crunch {
            row(&mut out, "K", "Big Crunch");
        }
    }

    head(&mut out, "ÉCRANS");
    for s in nav::available(snap) {
        if let Some(k) = s.key() {
            row(&mut out, &k.to_string(), s.name());
        }
    }
    if snap.starmap_unlocked || snap.pending_event.is_some() {
        row(&mut out, "Z", "Événement cosmique en attente");
    }

    head(&mut out, "NAVIGATION");
    row(&mut out, "Tab", "Écran suivant (Maj+Tab : précédent)");
    row(
        &mut out,
        "Échap",
        "Fermer l'écran ; dans la fonderie : menu",
    );
    row(&mut out, "? ou F1", "Cette aide");
    row(&mut out, "S", "Sauvegarder maintenant");
    row(&mut out, "F", "Effets complets / réduits");
    row(&mut out, "Q", "Sauvegarder et quitter");
    out
}

pub fn render_help(buf: &mut Buffer, area: Rect, ctx: &Ctx, popup: &Popup, scroll: &mut u16) {
    let lines = help_lines(ctx.snap);
    let height = lines.len() as u16 + 4;
    let Some(inner) = frame(buf, area, ctx, popup, "AIDE", 64, height) else {
        return;
    };
    let right = inner.right();
    let list_h = inner.height.saturating_sub(2);
    let max = (lines.len() as u16).saturating_sub(list_h);
    *scroll = (*scroll).min(max);
    for (k, (text, style)) in lines
        .iter()
        .skip(*scroll as usize)
        .take(list_h as usize)
        .enumerate()
    {
        draw::text(buf, inner.x + 1, inner.y + k as u16, text, *style, right);
    }
    let footer = if max > 0 {
        "[↑↓] défiler   [Échap] fermer"
    } else {
        "[Échap] fermer"
    };
    draw::text(
        buf,
        inner.x + 1,
        inner.bottom() - 1,
        footer,
        theme::fg(theme::GREY),
        right,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_game_hides_what_is_not_discovered() {
        let snap = GameSnapshot::default();
        let text: String = help_lines(&snap)
            .into_iter()
            .map(|(l, _)| l + "\n")
            .collect();
        for hidden in [
            "Supernova",
            "Big Crunch",
            "Atelier",
            "Reliques",
            "comète",
            "automate",
        ] {
            assert!(!text.contains(hidden), "{hidden} visible :\n{text}");
        }
        assert!(text.contains("Récolter") && text.contains("Options"));
    }

    #[test]
    fn tabs_shrink_before_disappearing() {
        let s = [ScreenId::Foundry, ScreenId::Shop, ScreenId::Info];
        let full = tab_segments(&s, 0);
        assert_eq!(
            full.iter().map(|(t, _)| t.as_str()).collect::<String>(),
            "Fonderie │ Atelier E │ Infos I"
        );
        assert!(width(&tab_segments(&s, 2)) < width(&full));
    }
}
