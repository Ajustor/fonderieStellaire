//! Police « gros chiffres » sur 3 lignes, avec quelques lettres pour les annonces.

/// Glyphe : 3 lignes de même largeur.
pub type Glyph = [&'static str; 3];

pub fn glyph(c: char) -> Option<Glyph> {
    Some(match c {
        '0' => ["█▀█", "█ █", "▀▀▀"],
        '1' => [" █ ", " █ ", " ▀ "],
        '2' => ["▀▀█", "█▀▀", "▀▀▀"],
        '3' => ["▀▀█", " ▀█", "▀▀▀"],
        '4' => ["█ █", "▀▀█", "  ▀"],
        '5' => ["█▀▀", "▀▀█", "▀▀▀"],
        '6' => ["█▀▀", "█▀█", "▀▀▀"],
        '7' => ["▀▀█", "  █", "  ▀"],
        '8' => ["█▀█", "█▀█", "▀▀▀"],
        '9' => ["█▀█", "▀▀█", "▀▀▀"],
        '.' => [" ", " ", "▀"],
        ' ' => ["  ", "  ", "  "],
        '+' => ["   ", "▄█▄", " ▀ "],
        '-' => ["   ", "▀▀▀", "   "],
        'A' => ["█▀█", "█▀█", "▀ ▀"],
        'B' => ["█▀▄", "█▀▄", "▀▀ "],
        'C' => ["█▀▀", "█  ", "▀▀▀"],
        'H' => ["█ █", "█▀█", "▀ ▀"],
        'I' => ["▀█▀", " █ ", "▀▀▀"],
        'U' => ["█ █", "█ █", "▀▀▀"],
        'E' => ["█▀▀", "█▀▀", "▀▀▀"],
        'F' => ["█▀▀", "█▀ ", "▀  "],
        'G' => ["█▀▀", "█ █", "▀▀▀"],
        'M' => ["█▀▄▀█", "█ ▀ █", "▀   ▀"],
        'N' => ["█▄ █", "█ ▀█", "▀  ▀"],
        'R' => ["█▀█", "█▀▄", "▀ ▀"],
        'S' => ["█▀▀", "▀▀█", "▀▀▀"],
        'T' => ["▀█▀", " █ ", " ▀ "],
        _ => return None,
    })
}

fn glyph_width(g: &Glyph) -> u16 {
    g[0].chars().count() as u16
}

/// Largeur affichée d'un texte (1 colonne d'espace entre deux glyphes).
pub fn text_width(text: &str) -> u16 {
    let mut w = 0u16;
    let mut first = true;
    for c in text.chars() {
        if let Some(g) = glyph(c) {
            if !first {
                w += 1;
            }
            w += glyph_width(&g);
            first = false;
        }
    }
    w
}

/// Appelle `put(dx, dy, char_index, ch)` pour chaque cellule non vide du texte.
/// `dx` est la colonne relative, `char_index` l'index du caractère source.
pub fn layout(text: &str, mut put: impl FnMut(u16, u16, usize, char)) {
    let mut x = 0u16;
    let mut first = true;
    for (i, c) in text.chars().enumerate() {
        let Some(g) = glyph(c) else { continue };
        if !first {
            x += 1;
        }
        first = false;
        for (dy, row) in g.iter().enumerate() {
            for (dx, ch) in row.chars().enumerate() {
                if ch != ' ' {
                    put(x + dx as u16, dy as u16, i, ch);
                }
            }
        }
        x += glyph_width(&g);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths() {
        assert_eq!(text_width("0"), 3);
        assert_eq!(text_width("12"), 7);
        assert_eq!(text_width("1.5"), 3 + 1 + 1 + 1 + 3);
        assert_eq!(text_width("1 0"), 3 + 1 + 2 + 1 + 3);
        assert_eq!(text_width("+3 FRAGMENTS"), text_width("+3 FRAGMENTS"));
    }

    #[test]
    fn all_glyph_rows_have_same_width() {
        for c in "0123456789. +-ABCEFGHIMNRSTU".chars() {
            let g = glyph(c).unwrap();
            let w = g[0].chars().count();
            assert!(g.iter().all(|r| r.chars().count() == w), "{c}");
        }
    }

    #[test]
    fn layout_renders_rows() {
        let mut grid = vec![vec![' '; 7]; 3];
        layout("10", |x, y, _, ch| grid[y as usize][x as usize] = ch);
        let rows: Vec<String> = grid.iter().map(|r| r.iter().collect()).collect();
        assert_eq!(rows, [" █  █▀█", " █  █ █", " ▀  ▀▀▀"]);
    }
}
