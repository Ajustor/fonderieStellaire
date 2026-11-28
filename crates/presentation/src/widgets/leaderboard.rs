//! Classement en ligne (Options › Classement) : les 20 premiers, général ou
//! du Défi du jour, la ligne du joueur, et l'adresse du site. Les textes
//! servent aussi à l'écran mobile.

use application::{Board, Entry, Standings};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Widget};

use super::Ctx;
use crate::draw;
use crate::fmt;
use crate::fx::Rgb;
use crate::theme;
use crate::ui::Popup;

/// Ce que l'écran montre du classement (préparé par `App`).
#[derive(Clone, Debug, PartialEq)]
pub struct LeaderboardView {
    /// Un relais est configuré dans cette version.
    pub available: bool,
    /// Cet appareil est synchronisé (condition pour entrer au classement).
    pub linked: bool,
    pub pseudo: Option<String>,
    pub board: Board,
    pub standings: Option<Standings>,
    pub busy: bool,
    pub error: Option<String>,
    /// Secondes depuis la dernière mise à jour du classement.
    pub fetched_ago: Option<f64>,
    pub site: String,
}

impl LeaderboardView {
    /// Valeur courte pour la ligne des Options.
    pub fn short(&self) -> String {
        match (&self.pseudo, self.linked, self.available) {
            (_, _, false) => "indisponible".into(),
            (Some(p), _, _) => p.clone(),
            (None, true, _) => "sans pseudo".into(),
            (None, false, _) => "non synchronisé".into(),
        }
    }

    /// Explication quand le joueur n'apparaît pas au classement.
    pub fn how_to_join(&self, touch: bool) -> Option<&'static str> {
        match (self.pseudo.is_some(), self.linked) {
            (true, _) => None,
            (false, true) if touch => Some(
                "Choisis un pseudo pour apparaître au classement (facultatif).",
            ),
            (false, true) => Some(
                "Choisis un pseudo pour apparaître au classement : [Entrée], ou [N] dans la fenêtre de synchronisation (facultatif).",
            ),
            (false, false) if touch => Some(
                "Pour y apparaître : synchronise ta partie (Sauver › Synchroniser entre appareils), puis choisis un pseudo.",
            ),
            (false, false) => Some(
                "Pour y apparaître : synchronise ta partie (Options › Synchronisation), puis choisis un pseudo.",
            ),
        }
    }

    /// État de la liste (chargement, erreur, mise à jour).
    pub fn status(&self) -> (String, Rgb) {
        if !self.available {
            return (
                "Classement indisponible dans cette version.".into(),
                theme::GREY,
            );
        }
        if let Some(e) = &self.error {
            return (format!("⚠ {e}"), theme::ORANGE);
        }
        match (self.busy, self.fetched_ago) {
            (true, None) => ("Chargement du classement…".into(), theme::CYAN),
            (_, Some(ago)) => (
                format!("Mis à jour il y a {}", fmt::duration(ago.max(1.0))),
                theme::GREY,
            ),
            (false, None) => ("Classement pas encore chargé.".into(), theme::GREY),
        }
    }
}

pub fn board_name(board: Board) -> &'static str {
    match board {
        Board::General => "Général",
        Board::Daily => "Défi du jour",
    }
}

/// Acte en chiffres romains (5 : Cosmos éveillé).
pub fn act_name(act: u8) -> &'static str {
    match act {
        0 | 1 => "Acte I",
        2 => "Acte II",
        3 => "Acte III",
        4 => "Acte IV",
        _ => "Cosmos éveillé",
    }
}

/// Résultat d'une ligne : avancée (général) ou score (Défi du jour).
pub fn entry_result(e: &Entry, board: Board) -> String {
    match board {
        Board::General => format!("{} · {}", act_name(e.act), fmt::percent(e.progress)),
        Board::Daily => format!("{} ✦", fmt::short(e.score.unwrap_or(0.0))),
    }
}

/// Prestiges d'une ligne du classement général : « ✹12 ✶3 ✷1 »
/// (Supernovas, Big Crunch, Big Bang ; les zéros sont omis).
pub fn entry_prestiges(e: &Entry) -> String {
    [("✹", e.supernovas), ("✶", e.crunches), ("✷", e.big_bangs)]
        .iter()
        .filter(|(_, n)| *n > 0)
        .map(|(g, n)| format!("{g}{n}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Place des prestiges entre le pseudo (qui commence en `pseudo_x`) et le
/// résultat (qui commence en `result_x`) : colonne et texte, s'il reste au
/// moins 8 colonnes au pseudo (classement général seulement).
pub fn prestige_slot(
    e: &Entry,
    board: Board,
    pseudo_x: u16,
    result_x: u16,
) -> Option<(u16, String)> {
    if board != Board::General {
        return None;
    }
    let text = entry_prestiges(e);
    let w = text.chars().count() as u16;
    if w == 0 {
        return None;
    }
    let x = result_x.checked_sub(w + 1)?;
    let pseudo_w = (e.pseudo.chars().count() as u16).min(8);
    (x > pseudo_x + pseudo_w).then_some((x, text))
}

/// Couleur du rang (podium).
pub fn rank_color(rank: u32) -> Rgb {
    match rank {
        1 => theme::GOLD,
        2 => theme::WHITE,
        3 => theme::ORANGE,
        _ => theme::GREY,
    }
}

/// Lignes à afficher : les premiers, puis la ligne du joueur s'il est plus bas.
pub fn visible_entries(s: &Standings, max: usize) -> (Vec<&Entry>, Option<&Entry>) {
    let me_listed = s.entries.iter().take(max).any(|e| e.me);
    let top: Vec<&Entry> = s.entries.iter().take(max).collect();
    let mine = s.me.as_ref().filter(|_| !me_listed);
    (top, mine)
}

fn draw_entry(buf: &mut Buffer, inner: Rect, y: u16, e: &Entry, board: Board) {
    let right = inner.right();
    if e.me {
        draw::fill_bg(buf, Rect::new(inner.x, y, inner.width, 1), theme::SELECT_BG);
    }
    let rank = format!("{:>3}", e.rank);
    let x = draw::text(
        buf,
        inner.x + 1,
        y,
        &rank,
        theme::bold(rank_color(e.rank)),
        right,
    );
    let result = entry_result(e, board);
    let mut rx = right.saturating_sub(1 + result.chars().count() as u16);
    if let Some((px, text)) = prestige_slot(e, board, x + 2, rx) {
        draw::text(buf, px, y, &text, theme::fg(theme::MUTED), rx);
        rx = px;
    }
    let pseudo_color = if e.me { theme::GOLD } else { theme::TEXT };
    draw::text(
        buf,
        x + 2,
        y,
        &e.pseudo,
        theme::bold(pseudo_color),
        rx.saturating_sub(1),
    );
    draw::text_right(buf, right - 1, y, &result, theme::fg(theme::CYAN), inner.x);
}

pub fn render(buf: &mut Buffer, area: Rect, ctx: &Ctx, popup: &Popup, view: &LeaderboardView) {
    let width = ctx.frame.width(area, 72);
    // Texte à une largeur de lecture raisonnable, même en onglet.
    let text_w = width.min(72).saturating_sub(4) as usize;
    let max_h = area.height.saturating_sub(2);

    let join: Vec<String> = view
        .how_to_join(false)
        .map(|t| draw::wrap(t, text_w))
        .unwrap_or_default();
    let (status, status_color) = view.status();
    let status = draw::wrap(&status, text_w);
    let site = draw::wrap(&format!("Tout le classement : {}", view.site), text_w);
    let footer_long = "[←→] général / du jour   [Entrée] pseudo   [Échap] fermer";
    let footer = if footer_long.chars().count() <= text_w {
        footer_long
    } else {
        "[←→] [Entrée] [Échap]"
    };

    // Hauteur : bordures, onglets + blanc, lignes, blanc, état, explication,
    // site, blanc, pied. On réduit d'abord le nombre de lignes.
    let fixed = 2 + 2 + 1 + status.len() + join.len() + site.len() + 1 + 1;
    let room = (max_h as usize).saturating_sub(fixed);
    let max_rows = room.clamp(1, 21);
    let (top, mine) = view
        .standings
        .as_ref()
        .map(|st| visible_entries(st, max_rows.saturating_sub(2).max(1)))
        .unwrap_or_default();
    let list_h = if top.is_empty() {
        1
    } else {
        top.len() + mine.map_or(0, |_| 2)
    };
    let height = ((fixed + list_h) as u16).min(max_h);
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
        .border_style(theme::bold(theme::CYAN))
        .title(Line::from(Span::styled(" ★ CLASSEMENT ", theme::bold(theme::GOLD))).centered());
    let inner = block.inner(rect);
    block.render(rect, buf);
    if !ready || inner.height < 4 {
        return;
    }
    let right = inner.right();
    let mut y = inner.y;

    // Onglets.
    let mut x = inner.x + 1;
    for b in [Board::General, Board::Daily] {
        let label = format!(" {} ", board_name(b));
        let style = if b == view.board {
            theme::bold(theme::BLACK).bg(theme::GOLD.color())
        } else {
            theme::fg(theme::MUTED)
        };
        x = draw::text(buf, x, y, &label, style, right) + 1;
    }
    if let Some(p) = &view.pseudo {
        let me = format!("toi : {p}");
        if x + me.chars().count() as u16 + 2 <= right {
            draw::text_right(buf, right - 1, y, &me, theme::fg(theme::GOLD), x);
        }
    }
    y += 2;

    if top.is_empty() {
        let empty = match view.board {
            Board::General => "Aucun joueur pour l'instant.",
            Board::Daily => "Personne n'a encore joué le défi d'aujourd'hui.",
        };
        let msg = if view.standings.is_some() {
            empty
        } else {
            "…"
        };
        draw::text(buf, inner.x + 2, y, msg, theme::fg(theme::GREY), right);
        y += 1;
    } else {
        for e in &top {
            draw_entry(buf, inner, y, e, view.board);
            y += 1;
        }
        if let Some(e) = mine {
            draw::text(buf, inner.x + 3, y, "…", theme::fg(theme::GREY), right);
            y += 1;
            draw_entry(buf, inner, y, e, view.board);
            y += 1;
        }
    }
    y += 1;
    for l in &status {
        draw::text(buf, inner.x + 1, y, l, theme::fg(status_color), right);
        y += 1;
    }
    for l in &join {
        draw::text(buf, inner.x + 1, y, l, theme::fg(theme::MUTED), right);
        y += 1;
    }
    for l in &site {
        if y >= inner.bottom().saturating_sub(1) {
            draw::audit::record(l, 0);
            continue;
        }
        draw::text(buf, inner.x + 1, y, l, theme::fg(theme::CYAN), right);
        y += 1;
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(rank: u32, me: bool) -> Entry {
        Entry {
            rank,
            pseudo: format!("j{rank}"),
            me,
            act: 2,
            progress: 0.42,
            score: Some(1.5e6),
            ..Entry::default()
        }
    }

    #[test]
    fn my_line_is_added_only_when_below_the_top() {
        let mut s = Standings {
            board: Board::General,
            day: 1,
            total: 30,
            entries: (1..=20).map(|r| entry(r, r == 3)).collect(),
            me: Some(entry(3, true)),
        };
        let (top, mine) = visible_entries(&s, 10);
        assert_eq!(top.len(), 10);
        assert!(mine.is_none(), "déjà dans la liste");
        s.entries[2].me = false;
        s.me = Some(entry(25, true));
        let (_, mine) = visible_entries(&s, 10);
        assert_eq!(mine.map(|e| e.rank), Some(25));
    }

    #[test]
    fn results_read_well() {
        assert_eq!(
            entry_result(&entry(1, false), Board::General),
            "Acte II · 42%"
        );
        assert!(entry_result(&entry(1, false), Board::Daily).ends_with('✦'));
        assert_eq!(act_name(5), "Cosmos éveillé");
    }

    #[test]
    fn prestiges_fit_or_step_aside() {
        let mut e = entry(1, false);
        e.supernovas = 12;
        e.crunches = 3;
        assert_eq!(entry_prestiges(&e), "✹12 ✶3");
        // De la place : juste avant le résultat.
        assert_eq!(
            prestige_slot(&e, Board::General, 10, 40),
            Some((33, "✹12 ✶3".to_string()))
        );
        // Trop étroit : le pseudo garde sa place.
        assert_eq!(prestige_slot(&e, Board::General, 10, 18), None);
        assert_eq!(prestige_slot(&e, Board::Daily, 10, 40), None);
        e.supernovas = 0;
        e.crunches = 0;
        assert_eq!(prestige_slot(&e, Board::General, 10, 40), None);
    }
}
