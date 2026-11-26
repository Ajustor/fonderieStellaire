//! Sérialisation d'un `Buffer` en grille de cellules brutes, pour un affichage
//! hors terminal (Android dessine ces cellules lui-même).
//!
//! Format (petit-boutiste) :
//! - en-tête : `cols: u16`, `rows: u16`, `n: u16` (nombre de lignes envoyées) ;
//! - puis `n` fois : `row: u16` suivi de `cols` cellules de [`CELL_BYTES`] octets :
//!   `char: u32` (0 = suite d'un caractère large), `fg: u32`, `bg: u32` (ARGB,
//!   0 = couleur par défaut), `mods: u8` (voir [`BOLD`]…).
//!
//! Seules les lignes qui ont changé depuis l'image précédente sont envoyées.

use ratatui::style::{Color, Modifier};

pub use ratatui::buffer::Buffer;
pub use ratatui::layout::Rect;

pub const HEADER_BYTES: usize = 6;
pub const CELL_BYTES: usize = 13;

pub const BOLD: u8 = 1;
pub const DIM: u8 = 2;
pub const ITALIC: u8 = 4;
pub const UNDERLINED: u8 = 8;
pub const REVERSED: u8 = 16;

/// Couleur ARGB opaque ; `0` pour la couleur par défaut du thème.
pub fn argb(color: Color) -> u32 {
    let rgb = |r: u8, g: u8, b: u8| 0xFF00_0000 | (r as u32) << 16 | (g as u32) << 8 | b as u32;
    match color {
        Color::Reset => 0,
        Color::Rgb(r, g, b) => rgb(r, g, b),
        Color::Black => rgb(0, 0, 0),
        Color::Red => rgb(205, 49, 49),
        Color::Green => rgb(13, 188, 121),
        Color::Yellow => rgb(229, 229, 16),
        Color::Blue => rgb(36, 114, 200),
        Color::Magenta => rgb(188, 63, 188),
        Color::Cyan => rgb(17, 168, 205),
        Color::Gray => rgb(204, 204, 204),
        Color::DarkGray => rgb(118, 118, 118),
        Color::LightRed => rgb(241, 76, 76),
        Color::LightGreen => rgb(35, 209, 139),
        Color::LightYellow => rgb(245, 245, 67),
        Color::LightBlue => rgb(59, 142, 234),
        Color::LightMagenta => rgb(214, 112, 214),
        Color::LightCyan => rgb(41, 184, 219),
        Color::White => rgb(255, 255, 255),
        Color::Indexed(i) => indexed(i),
    }
}

/// Palette xterm 256 couleurs (cube 6×6×6 et niveaux de gris).
fn indexed(i: u8) -> u32 {
    let (r, g, b) = match i {
        0..=15 => {
            let named = [
                Color::Black,
                Color::Red,
                Color::Green,
                Color::Yellow,
                Color::Blue,
                Color::Magenta,
                Color::Cyan,
                Color::Gray,
                Color::DarkGray,
                Color::LightRed,
                Color::LightGreen,
                Color::LightYellow,
                Color::LightBlue,
                Color::LightMagenta,
                Color::LightCyan,
                Color::White,
            ];
            return argb(named[i as usize]);
        }
        16..=231 => {
            let n = i - 16;
            let level = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            (level(n / 36), level((n / 6) % 6), level(n % 6))
        }
        _ => {
            let v = 8 + (i - 232) * 10;
            (v, v, v)
        }
    };
    0xFF00_0000 | (r as u32) << 16 | (g as u32) << 8 | b as u32
}

fn mods(m: Modifier) -> u8 {
    let mut out = 0;
    if m.contains(Modifier::BOLD) {
        out |= BOLD;
    }
    if m.contains(Modifier::DIM) {
        out |= DIM;
    }
    if m.contains(Modifier::ITALIC) {
        out |= ITALIC;
    }
    if m.contains(Modifier::UNDERLINED) {
        out |= UNDERLINED;
    }
    if m.contains(Modifier::REVERSED) {
        out |= REVERSED;
    }
    out
}

/// Écrit dans `out` les lignes de `buf` qui diffèrent de `prev` (toutes si
/// `prev` est absent ou d'une autre taille).
pub fn encode(buf: &Buffer, prev: Option<&Buffer>, out: &mut Vec<u8>) {
    out.clear();
    let area = buf.area;
    let (cols, rows) = (area.width as usize, area.height as usize);
    let prev = prev.filter(|p| p.area == area);
    out.extend_from_slice(&(cols as u16).to_le_bytes());
    out.extend_from_slice(&(rows as u16).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    let mut sent: u16 = 0;
    for row in 0..rows {
        let line = &buf.content[row * cols..(row + 1) * cols];
        if prev.is_some_and(|p| &p.content[row * cols..(row + 1) * cols] == line) {
            continue;
        }
        sent += 1;
        out.extend_from_slice(&(row as u16).to_le_bytes());
        for cell in line {
            let ch = if cell.skip {
                ' ' as u32
            } else {
                cell.symbol().chars().next().map_or(0, |c| c as u32)
            };
            out.extend_from_slice(&ch.to_le_bytes());
            out.extend_from_slice(&argb(cell.fg).to_le_bytes());
            out.extend_from_slice(&argb(cell.bg).to_le_bytes());
            out.push(mods(cell.modifier));
        }
    }
    out[4..6].copy_from_slice(&sent.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Style;

    fn cell_at(out: &[u8], line: usize, cols: usize, col: usize) -> (u32, u32, u32, u8) {
        let base = HEADER_BYTES + line * (2 + cols * CELL_BYTES) + 2 + col * CELL_BYTES;
        let u = |o: usize| u32::from_le_bytes(out[base + o..base + o + 4].try_into().unwrap());
        (u(0), u(4), u(8), out[base + 12])
    }

    #[test]
    fn encodes_cells_and_sends_only_changed_rows() {
        let area = Rect::new(0, 0, 4, 3);
        let mut a = Buffer::empty(area);
        a.set_string(
            1,
            1,
            "✦",
            Style::default()
                .fg(Color::Rgb(255, 128, 0))
                .add_modifier(Modifier::BOLD),
        );
        let mut out = Vec::new();
        encode(&a, None, &mut out);
        assert_eq!(&out[0..6], &[4, 0, 3, 0, 3, 0]);
        assert_eq!(out.len(), HEADER_BYTES + 3 * (2 + 4 * CELL_BYTES));
        assert_eq!(cell_at(&out, 1, 4, 1), ('✦' as u32, 0xFFFF_8000, 0, BOLD));
        assert_eq!(cell_at(&out, 0, 4, 0).0, ' ' as u32);

        let mut b = a.clone();
        b.set_string(0, 2, "x", Style::default());
        encode(&b, Some(&a), &mut out);
        assert_eq!(&out[4..6], &[1, 0], "une seule ligne a changé");
        assert_eq!(&out[6..8], &[2, 0]);
        assert_eq!(cell_at(&out, 0, 4, 0).0, 'x' as u32);
    }

    #[test]
    fn indexed_colors_follow_the_xterm_palette() {
        assert_eq!(argb(Color::Indexed(16)), 0xFF00_0000);
        assert_eq!(argb(Color::Indexed(231)), 0xFFFF_FFFF);
        assert_eq!(argb(Color::Indexed(232)), 0xFF08_0808);
    }
}
