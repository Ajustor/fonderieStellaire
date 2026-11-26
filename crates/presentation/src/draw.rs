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

/// Écrit une chaîne sans dépasser `max_x` (exclu). Renvoie la colonne suivante.
pub fn text(buf: &mut Buffer, x: u16, y: u16, s: &str, style: Style, max_x: u16) -> u16 {
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
    if n == 0 {
        return String::new();
    }
    let mut out: String = s.chars().take(n - 1).collect();
    out.push('…');
    out
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
