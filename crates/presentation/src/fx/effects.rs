//! Post-effets appliqués directement sur une zone du `Buffer` après le rendu.

use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::Rect;
use ratatui::style::Color;

use super::color::{Rgb, DEFAULT_BG, DEFAULT_FG};
use super::rng::hash3;

const GLITCH_GLYPHS: [char; 8] = ['▓', '▒', '░', '█', '╳', '◆', '▚', '▞'];

fn clip(buf: &Buffer, area: Rect) -> Rect {
    buf.area.intersection(area)
}

fn fg_rgb(c: Color) -> Rgb {
    Rgb::from_color(c).unwrap_or(DEFAULT_FG)
}

/// Rapproche le texte (et le fond) de `color`.
pub fn flash(buf: &mut Buffer, area: Rect, color: Rgb, amount: f64) {
    if amount <= 0.01 {
        return;
    }
    let area = clip(buf, area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.fg = fg_rgb(cell.fg).lerp(color, amount).color();
                let bg = Rgb::from_color(cell.bg).unwrap_or(DEFAULT_BG);
                cell.bg = bg.lerp(color, amount * 0.85).color();
            }
        }
    }
}

/// Colore uniquement le fond (sans toucher au texte).
pub fn tint_bg(buf: &mut Buffer, area: Rect, color: Rgb, amount: f64) {
    if amount <= 0.01 {
        return;
    }
    let area = clip(buf, area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                let bg = Rgb::from_color(cell.bg).unwrap_or(DEFAULT_BG);
                cell.bg = bg.lerp(color, amount).color();
            }
        }
    }
}

/// Assombrit la zone (`amount = 1` → noir).
pub fn fade(buf: &mut Buffer, area: Rect, amount: f64) {
    if amount <= 0.001 {
        return;
    }
    let area = clip(buf, area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.fg = fg_rgb(cell.fg).dim(amount).color();
                if let Some(bg) = Rgb::from_color(cell.bg) {
                    cell.bg = bg.dim(amount).color();
                }
            }
        }
    }
}

/// Décale le contenu de la zone de (`dx`, `dy`) cellules ; les bords exposés sont vidés.
/// Sert au shake et aux glissements de panneaux.
pub fn offset(buf: &mut Buffer, area: Rect, dx: i32, dy: i32) {
    if dx == 0 && dy == 0 {
        return;
    }
    let area = clip(buf, area);
    if area.is_empty() {
        return;
    }
    let (w, h) = (area.width as i32, area.height as i32);
    // Parcours dans l'ordre qui évite d'écraser une source pas encore copiée.
    for k in 0..h {
        let ry = if dy > 0 { h - 1 - k } else { k };
        for j in 0..w {
            let rx = if dx > 0 { w - 1 - j } else { j };
            let (sx, sy) = (rx - dx, ry - dy);
            let dst = (area.x + rx as u16, area.y + ry as u16);
            let src_cell = if sx >= 0 && sx < w && sy >= 0 && sy < h {
                buf.cell((area.x + sx as u16, area.y + sy as u16)).cloned()
            } else {
                None
            };
            if let Some(cell) = buf.cell_mut(dst) {
                match src_cell {
                    Some(c) => *cell = c,
                    None => {
                        cell.reset();
                    }
                }
            }
        }
    }
}

/// Décalage de secousse pour une intensité et un instant donnés.
pub fn shake_offset(t: f64, intensity: f64) -> (i32, i32) {
    if intensity <= 0.0 {
        return (0, 0);
    }
    let dx = ((t * 53.0).sin() * intensity * 2.0).round() as i32;
    let dy = ((t * 41.0 + 1.3).sin() * intensity * 0.8).round() as i32;
    (dx, dy)
}

/// Remplace aléatoirement des caractères par des glyphes parasites.
pub fn glitch(buf: &mut Buffer, area: Rect, density: f64, seed: i64) {
    let area = clip(buf, area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if hash3(x as i64, y as i64, seed) < density {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    let g =
                        (hash3(y as i64, x as i64, seed + 7) * GLITCH_GLYPHS.len() as f64) as usize;
                    cell.set_char(GLITCH_GLYPHS[g.min(GLITCH_GLYPHS.len() - 1)]);
                }
            }
        }
    }
}

/// Bande lumineuse diagonale centrée sur `pos` (en colonnes, mesurée sur `x + 2y`).
pub fn shimmer(buf: &mut Buffer, area: Rect, pos: f64, width: f64, strength: f64) {
    let area = clip(buf, area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let d = ((x - area.x) as f64 + 2.0 * (y - area.y) as f64 - pos).abs();
            if d < width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    if cell.symbol() != " " {
                        let k = (1.0 - d / width) * strength;
                        cell.fg = fg_rgb(cell.fg).glow(k).color();
                    }
                }
            }
        }
    }
}

/// Fait disparaître progressivement les cellules (`progress = 1` → tout effacé).
pub fn dissolve(buf: &mut Buffer, area: Rect, progress: f64, seed: i64) {
    if progress <= 0.0 {
        return;
    }
    let area = clip(buf, area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let h = hash3(x as i64, y as i64, seed);
            if let Some(cell) = buf.cell_mut((x, y)) {
                if h < progress {
                    cell.reset();
                } else if h < progress + 0.08 && cell.symbol() != " " {
                    cell.set_char('░');
                }
            }
        }
    }
}

/// Préfixe visible d'un texte écrit en machine à écrire à `cps` caractères/s.
pub fn typewriter(text: &str, elapsed: f64, cps: f64) -> &str {
    if elapsed <= 0.0 {
        return "";
    }
    let n = (elapsed * cps) as usize;
    match text.char_indices().nth(n) {
        Some((i, _)) => &text[..i],
        None => text,
    }
}

/// Positions du contour d'un rectangle, dans le sens horaire depuis le coin haut-gauche.
pub fn perimeter_len(area: Rect) -> usize {
    if area.width < 2 || area.height < 2 {
        return 0;
    }
    2 * (area.width as usize + area.height as usize) - 4
}

pub fn perimeter_point(area: Rect, i: usize) -> (u16, u16) {
    let w = area.width as usize;
    let h = area.height as usize;
    let i = i % perimeter_len(area).max(1);
    if i < w {
        (area.x + i as u16, area.y)
    } else if i < w + h - 1 {
        (area.right() - 1, area.y + (i - w + 1) as u16)
    } else if i < 2 * w + h - 2 {
        (
            area.right() - 1 - (i - (w + h - 1) + 1) as u16,
            area.bottom() - 1,
        )
    } else {
        (area.x, area.bottom() - 1 - (i - (2 * w + h - 2) + 1) as u16)
    }
}

/// Applique une fonction à chaque cellule de bordure (avec sa position sur le contour).
pub fn for_each_border_cell(buf: &mut Buffer, area: Rect, mut f: impl FnMut(usize, &mut Cell)) {
    let area = clip(buf, area);
    let n = perimeter_len(area);
    for i in 0..n {
        let p = perimeter_point(area, i);
        if let Some(cell) = buf.cell_mut(p) {
            f(i, cell);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Style;

    #[test]
    fn typewriter_prefix() {
        assert_eq!(typewriter("Bonjour", 0.0, 10.0), "");
        assert_eq!(typewriter("Bonjour", 0.3, 10.0), "Bon");
        assert_eq!(typewriter("été", 0.2, 10.0), "ét");
        assert_eq!(typewriter("abc", 10.0, 10.0), "abc");
    }

    #[test]
    fn offset_shifts_content() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 1));
        buf.set_string(0, 0, "abcde", Style::default());
        offset(&mut buf, Rect::new(0, 0, 5, 1), 2, 0);
        let s: String = (0..5).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert_eq!(s, "  abc");
        offset(&mut buf, Rect::new(0, 0, 5, 1), -3, 0);
        let s: String = (0..5).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert_eq!(s, "bc   ");
    }

    #[test]
    fn perimeter_walks_the_border() {
        let r = Rect::new(2, 3, 4, 3);
        let n = perimeter_len(r);
        assert_eq!(n, 10);
        let pts: Vec<_> = (0..n).map(|i| perimeter_point(r, i)).collect();
        assert_eq!(pts[0], (2, 3));
        assert_eq!(pts[3], (5, 3));
        assert_eq!(pts[5], (5, 5));
        assert_eq!(pts[8], (2, 5));
        assert_eq!(pts[9], (2, 4));
        let mut uniq = pts.clone();
        uniq.sort();
        uniq.dedup();
        assert_eq!(uniq.len(), n);
    }

    #[test]
    fn dissolve_full_clears() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 2));
        buf.set_string(0, 0, "abcd", Style::default());
        dissolve(&mut buf, Rect::new(0, 0, 4, 2), 1.0, 3);
        assert!((0..4).all(|x| buf[(x, 0)].symbol() == " "));
    }

    #[test]
    fn fade_darkens() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 1, 1));
        buf[(0, 0)].set_fg(Color::Rgb(200, 100, 50));
        fade(&mut buf, Rect::new(0, 0, 1, 1), 0.5);
        assert_eq!(buf[(0, 0)].fg, Color::Rgb(100, 50, 25));
    }
}
