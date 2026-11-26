//! Barre d'aide (dernière ligne) : touches en gras accentué, libellés en gris.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use domain::BuyMode;

use crate::draw;
use crate::fx::Rgb;
use crate::theme;

struct Item {
    key: &'static str,
    label: String,
    /// Plus petit = plus prioritaire quand la place manque.
    priority: u8,
}

fn items(mode: BuyMode) -> Vec<Item> {
    let it = |key, label: &str, priority| Item {
        key,
        label: label.to_string(),
        priority,
    };
    vec![
        it("Espace", "Récolter", 0),
        it("↑↓", "Choisir", 10),
        it("Entrée", "Acheter", 1),
        it("A", "Tout", 5),
        it("Tab", "Panneau", 11),
        it("M", mode.label(), 2),
        it("C", "Comète", 8),
        it("P", "Nova", 3),
        it("E", "Atelier", 6),
        it("D", "Défis", 9),
        it("I", "Infos", 7),
        it("S", "Sauver", 12),
        it("F", "Effets", 13),
        it("Q", "Quitter", 4),
    ]
}

fn item_width(i: &Item) -> u16 {
    (i.key.chars().count() + 2 + 1 + i.label.chars().count() + 2) as u16
}

pub fn render(buf: &mut Buffer, area: Rect, mode: BuyMode, hold: bool, mode_flash: f64, now: f64) {
    if area.is_empty() {
        return;
    }
    let mut all = items(mode);
    if hold {
        all[0].label = "Récolter (maintenir)".into();
    }
    // Choisit les éléments par priorité jusqu'à remplir la largeur.
    let mut order: Vec<usize> = (0..all.len()).collect();
    order.sort_by_key(|&i| all[i].priority);
    let mut keep = vec![false; all.len()];
    let mut used = 1u16;
    for i in order {
        let w = item_width(&all[i]);
        if used + w <= area.width {
            keep[i] = true;
            used += w;
        }
    }
    let mut x = area.x + 1;
    let max = area.right();
    for (i, item) in all.iter().enumerate() {
        if !keep[i] {
            continue;
        }
        let mut key_color: Rgb = theme::GOLD;
        let mut label_color: Rgb = theme::GREY;
        if item.key == "M" {
            let p = (now - mode_flash) / 0.5;
            if (0.0..1.0).contains(&p) {
                key_color = theme::WHITE.lerp(theme::GOLD, p);
                label_color = theme::WHITE.lerp(theme::GREY, p);
            }
        }
        x = draw::text(buf, x, area.y, "[", theme::fg(theme::BORDER), max);
        x = draw::text(buf, x, area.y, item.key, theme::bold(key_color), max);
        x = draw::text(buf, x, area.y, "] ", theme::fg(theme::BORDER), max);
        x = draw::text(buf, x, area.y, &item.label, theme::fg(label_color), max);
        x = draw::text(buf, x, area.y, "  ", theme::fg(label_color), max);
    }
}
