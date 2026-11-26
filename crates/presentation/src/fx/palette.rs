//! Modes de couleur pour l'accessibilité, appliqués à l'écran déjà dessiné :
//! daltonien (les verts deviennent bleus, les rouges orangés : plus de paires
//! rouge/vert à distinguer) et contraste élevé (texte plus clair, fonds plus
//! sombres).

use ratatui::buffer::Buffer;
use ratatui::style::Color;

use super::color::{hsl_to_rgb, rgb_to_hsl};
use super::Rgb;

/// Couleur vue en mode daltonien.
pub fn daltonize(c: Rgb) -> Rgb {
    let (h, s, l) = rgb_to_hsl(c);
    if s < 0.12 {
        return c;
    }
    let h2 = if (70.0..170.0).contains(&h) {
        // Verts → bleu ciel.
        205.0
    } else if !(25.0..330.0).contains(&h) {
        // Rouges → orange franc.
        28.0
    } else {
        h
    };
    hsl_to_rgb(h2, s, l)
}

/// Couleur de texte en contraste élevé : plus claire et plus saturée.
pub fn contrast_fg(c: Rgb) -> Rgb {
    let (h, s, l) = rgb_to_hsl(c);
    hsl_to_rgb(h, (s * 1.15).min(1.0), l.max(0.62))
}

/// Fond en contraste élevé : plus sombre.
pub fn contrast_bg(c: Rgb) -> Rgb {
    let (h, s, l) = rgb_to_hsl(c);
    hsl_to_rgb(h, s, l * 0.5)
}

/// Applique le mode (0 normal, 1 daltonien, 2 contraste élevé) à tout l'écran.
pub fn apply(buf: &mut Buffer, mode: u8) {
    if mode == 0 {
        return;
    }
    let area = buf.area;
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let Some(cell) = buf.cell_mut((x, y)) else {
                continue;
            };
            if let Some(fg) = Rgb::from_color(cell.fg) {
                let fg = if mode == 1 {
                    daltonize(fg)
                } else {
                    contrast_fg(fg)
                };
                cell.set_fg(fg.color());
            }
            if cell.bg != Color::Reset {
                if let Some(bg) = Rgb::from_color(cell.bg) {
                    let bg = if mode == 1 {
                        daltonize(bg)
                    } else {
                        contrast_bg(bg)
                    };
                    cell.set_bg(bg.color());
                }
            }
        }
    }
}

/// Nom du mode, pour les Options.
pub fn mode_name(mode: u8) -> &'static str {
    match mode {
        1 => "daltonien",
        2 => "contraste élevé",
        _ => "normales",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greens_and_reds_are_told_apart() {
        let green = daltonize(Rgb(120, 230, 140));
        let red = daltonize(Rgb(255, 80, 90));
        let (hg, _, _) = rgb_to_hsl(green);
        let (hr, _, _) = rgb_to_hsl(red);
        assert!((hg - 205.0).abs() < 2.0, "{hg}");
        assert!((hr - 28.0).abs() < 2.0, "{hr}");
        // Gris : inchangé.
        assert_eq!(daltonize(Rgb(128, 128, 128)), Rgb(128, 128, 128));
        // Contraste : un texte sombre devient lisible.
        assert!(contrast_fg(Rgb(80, 80, 120)).luma() > Rgb(80, 80, 120).luma());
    }
}
