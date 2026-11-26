//! Petits utilitaires de dessin direct dans le `Buffer`.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Widget};

use crate::fx::Rgb;
use crate::theme;

/// Écrit un caractère s'il tombe dans `clip`.
pub fn put(buf: &mut Buffer, clip: Rect, x: i32, y: i32, ch: char, fg: Rgb) {
    if x < clip.x as i32
        || y < clip.y as i32
        || x >= clip.right() as i32
        || y >= clip.bottom() as i32
    {
        return;
    }
    if let Some(cell) = buf.cell_mut((x as u16, y as u16)) {
        cell.set_char(ch);
        cell.set_fg(fg.color());
    }
}

/// Écrit un caractère avec un style complet s'il tombe dans `clip`.
pub fn put_styled(buf: &mut Buffer, clip: Rect, x: i32, y: i32, ch: char, style: Style) {
    if x < clip.x as i32
        || y < clip.y as i32
        || x >= clip.right() as i32
        || y >= clip.bottom() as i32
    {
        return;
    }
    if let Some(cell) = buf.cell_mut((x as u16, y as u16)) {
        cell.set_char(ch);
        cell.set_style(style);
    }
}

/// Vrai si la cellule est vide (espace).
pub fn is_blank(buf: &Buffer, x: i32, y: i32) -> bool {
    if x < 0 || y < 0 {
        return false;
    }
    buf.cell((x as u16, y as u16))
        .map(|c| c.symbol() == " ")
        .unwrap_or(false)
}

/// Audit des textes coupés : les tests l'activent pour vérifier qu'aucune
/// information n'est perdue faute de place.
pub mod audit {
    use std::cell::{Cell, RefCell};

    thread_local! {
        static ON: Cell<bool> = const { Cell::new(false) };
        static CLIPS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    /// Démarre l'audit (et vide les coupures déjà notées).
    pub fn start() {
        ON.with(|o| o.set(true));
        CLIPS.with(|c| c.borrow_mut().clear());
    }

    /// Arrête l'audit et renvoie les textes coupés (sans doublons).
    pub fn take() -> Vec<String> {
        ON.with(|o| o.set(false));
        let mut v = CLIPS.with(|c| std::mem::take(&mut *c.borrow_mut()));
        v.sort();
        v.dedup();
        v
    }

    /// Note un texte qui ne tient pas dans `room` colonnes.
    pub(crate) fn record(s: &str, room: usize) {
        if !ON.with(|o| o.get()) {
            return;
        }
        // Seule compte une coupure qui fait disparaître autre chose que des espaces.
        let lost: String = s.chars().skip(room).collect();
        if lost.trim().is_empty() {
            return;
        }
        CLIPS.with(|c| c.borrow_mut().push(s.trim().to_string()));
    }
}

/// Écrit une chaîne sans dépasser `max_x` (exclu). Renvoie la colonne suivante.
pub fn text(buf: &mut Buffer, x: u16, y: u16, s: &str, style: Style, max_x: u16) -> u16 {
    if y < buf.area.bottom() {
        audit::record(s, max_x.saturating_sub(x) as usize);
    }
    if x >= max_x {
        return x;
    }
    let (nx, _) = buf.set_stringn(x, y, s, (max_x - x) as usize, style);
    nx
}

/// Écrit une chaîne alignée à droite, se terminant juste avant `right` (exclu).
pub fn text_right(buf: &mut Buffer, right: u16, y: u16, s: &str, style: Style, min_x: u16) -> u16 {
    let w = s.chars().count() as u16;
    let x = right.saturating_sub(w).max(min_x);
    text(buf, x, y, s, style, right);
    x
}

/// Coupe une chaîne à `n` caractères (avec `…` si coupée).
pub fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    audit::record(s, n.saturating_sub(1));
    if n == 0 {
        return String::new();
    }
    let mut out: String = s.chars().take(n - 1).collect();
    out.push('…');
    out
}

/// Coupe une chaîne sans la signaler à l'audit : réservé aux listes dont le
/// texte complet est affiché ailleurs (encart de détail de la sélection).
pub fn fit(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    if n == 0 {
        return String::new();
    }
    let mut out: String = s.chars().take(n - 1).collect();
    out.push('…');
    out
}

/// Découpe un texte en lignes d'au plus `width` caractères, aux espaces
/// (un mot trop long est coupé net). Les sauts de ligne sont respectés.
pub fn wrap(s: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    for para in s.split('\n') {
        let mut cur = String::new();
        let mut cur_w = 0;
        for word in para.split(' ') {
            let mut word = word.to_string();
            let mut ww = word.chars().count();
            let sep = usize::from(cur_w > 0);
            if cur_w + sep + ww <= width {
                if sep == 1 {
                    cur.push(' ');
                }
                cur.push_str(&word);
                cur_w += sep + ww;
                continue;
            }
            if cur_w > 0 {
                lines.push(std::mem::take(&mut cur));
            }
            while ww > width {
                let head: String = word.chars().take(width).collect();
                word = word.chars().skip(width).collect();
                ww -= width;
                lines.push(head);
            }
            cur = word;
            cur_w = ww;
        }
        lines.push(cur);
    }
    lines
}

/// Écrit un texte sur plusieurs lignes (au plus `max_lines`) à partir de
/// `(x, y)`, dans `width` colonnes. Renvoie le nombre de lignes écrites.
pub fn paragraph(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    width: u16,
    max_lines: u16,
    s: &str,
    style: Style,
) -> u16 {
    let lines = wrap(s, width as usize);
    let mut n = 0;
    for (i, line) in lines.iter().enumerate() {
        if i as u16 >= max_lines {
            // Ce qui ne tient pas est signalé à l'audit.
            audit::record(&lines[i..].join(" "), 0);
            break;
        }
        text(buf, x, y + i as u16, line, style, x + width);
        n += 1;
    }
    n
}

/// Dessine un panneau arrondi et renvoie sa zone intérieure.
pub fn panel<'a>(buf: &mut Buffer, area: Rect, title: impl Into<Line<'a>>, focused: bool) -> Rect {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border_style(focused))
        .title(title.into());
    let inner = block.inner(area);
    block.render(area, buf);
    inner
}

/// Remplit le fond d'une zone.
pub fn fill_bg(buf: &mut Buffer, area: Rect, bg: Rgb) {
    let area = buf.area.intersection(area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(c) = buf.cell_mut((x, y)) {
                c.set_bg(bg.color());
            }
        }
    }
}

/// Efface une zone (cellules remises à zéro).
pub fn clear(buf: &mut Buffer, area: Rect) {
    let area = buf.area.intersection(area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(c) = buf.cell_mut((x, y)) {
                c.reset();
            }
        }
    }
}

/// Vrai si le caractère est un trait de bordure de boîte.
pub fn is_box_char(s: &str) -> bool {
    matches!(s, "─" | "│" | "╭" | "╮" | "╰" | "╯" | "┌" | "┐" | "└" | "┘")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncation() {
        assert_eq!(truncate("Bonjour", 10), "Bonjour");
        assert_eq!(truncate("Bonjour", 4), "Bon…");
        assert_eq!(truncate("Éé", 1), "…");
    }

    #[test]
    fn wrapping() {
        assert_eq!(wrap("un deux trois", 7), vec!["un deux", "trois"]);
        assert_eq!(wrap("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
        assert_eq!(wrap("a\nb", 10), vec!["a", "b"]);
        assert_eq!(wrap("", 10), vec![""]);
        for l in wrap(
            "Petite sonde autonome qui aspire la poussière en orbite basse.",
            20,
        ) {
            assert!(l.chars().count() <= 20);
        }
    }

    #[test]
    fn put_is_clipped() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 4));
        let clip = Rect::new(1, 1, 2, 2);
        put(&mut buf, clip, 0, 0, 'x', Rgb(1, 1, 1));
        put(&mut buf, clip, 1, 1, 'y', Rgb(1, 1, 1));
        put(&mut buf, clip, -5, 99, 'z', Rgb(1, 1, 1));
        assert_eq!(buf[(0, 0)].symbol(), " ");
        assert_eq!(buf[(1, 1)].symbol(), "y");
    }
}
