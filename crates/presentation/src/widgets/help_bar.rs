//! Barre d'aide (dernière ligne) : touches en gras accentué, libellés en gris.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use application::GameSnapshot;

use crate::draw;
use crate::fx::Rgb;
use crate::theme;

struct Item {
    key: &'static str,
    label: String,
    /// Plus petit = plus prioritaire quand la place manque.
    priority: u8,
}

/// Touches utiles maintenant : seules les actions possibles sont proposées
/// (la comète quand elle passe, la Supernova quand elle rapporte…). Les autres
/// touches marchent toujours, elles ne sont simplement pas rappelées.
fn items(snap: &GameSnapshot) -> Vec<Item> {
    let it = |key, label: &str, priority| Item {
        key,
        label: label.to_string(),
        priority,
    };
    let mut v = vec![it(
        "Espace",
        if snap.hold_harvest {
            "Récolter (maintenir)"
        } else {
            "Récolter"
        },
        0,
    )];
    v.push(it("Entrée", "Acheter", 1));
    if snap.comet_progress.is_some() {
        v.push(it("C", "Comète !", 0));
    }
    if snap.pending_event.is_some() {
        v.push(it("Z", "Événement !", 0));
    }
    if snap.fragments_available >= 1.0 {
        v.push(it("P", "Nova", 2));
    }
    if snap.crunch_available {
        v.push(it("U", "Crunch", 2));
    }
    if snap.upgrades.iter().any(|u| u.affordable) {
        v.push(it("A", "Tout", 3));
    }
    if snap.generators.iter().any(|g| g.owned > 0) {
        v.push(it("M", snap.buy_mode.label(), 6));
    }
    v.push(it("Tab", "Panneau", 9));
    if snap.supernovas > 0 || snap.fragments > 0.0 || snap.crunches > 0 {
        v.push(it("E", "Atelier", 7));
    }
    if snap.challenges.iter().any(|c| c.unlocked) {
        v.push(it("D", "Défis", 8));
    }
    if snap.lab_unlocked {
        v.push(it("L", "Labo", 8));
        v.push(it("G", "Cosmos", 8));
    }
    if snap.universes_unlocked {
        v.push(it("V", "Univers", 9));
    }
    if snap.daily.is_some() {
        v.push(it("Y", "Défi en cours !", 0));
    } else if snap.daily_unlocked {
        v.push(it("Y", "Défi du jour", 6));
    }
    if snap.megastructures_unlocked {
        v.push(it("W", "Méga", 6));
    }
    if snap.starmap_unlocked {
        v.push(it("X", "Carte", 7));
        v.push(it("H", "Reliques", 8));
    }
    v.push(it("I", "Infos", 5));
    v.push(it("R", "Options", 4));
    v.push(it("Q", "Quitter", 4));
    v
}

fn item_width(i: &Item) -> u16 {
    (i.key.chars().count() + 2 + 1 + i.label.chars().count() + 2) as u16
}

/// Durée de l'éclat d'une touche qui vient d'apparaître.
const NEW_GLOW: f64 = 1.5;

/// `seen` : touches déjà affichées et depuis quand (pour faire briller les
/// nouvelles venues un instant).
pub fn render(
    buf: &mut Buffer,
    area: Rect,
    snap: &GameSnapshot,
    seen: &mut Vec<(&'static str, f64)>,
    mode_flash: f64,
    now: f64,
) {
    if area.is_empty() {
        return;
    }
    let all = items(snap);
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
    // Les touches qui disparaissent pourront rebriller à leur retour.
    seen.retain(|(k, _)| all.iter().any(|i| i.key == *k));
    let mut x = area.x + 1;
    let max = area.right();
    for (i, item) in all.iter().enumerate() {
        if !keep[i] {
            continue;
        }
        let since = match seen.iter().find(|(k, _)| *k == item.key) {
            Some(&(_, t)) => now - t,
            None => {
                seen.push((item.key, now));
                0.0
            }
        };
        let mut key_color: Rgb = theme::GOLD;
        let mut label_color: Rgb = theme::GREY;
        // Une action qui vient d'apparaître brille un instant.
        if since < NEW_GLOW && now > NEW_GLOW {
            let p = since / NEW_GLOW;
            let pulse = 0.5 + 0.5 * (since * 10.0).cos();
            key_color = theme::WHITE.lerp(theme::GOLD, p.max(1.0 - pulse));
            label_color = theme::TEXT.lerp(theme::GREY, p);
        }
        if item.key == "M" {
            let p = (now - mode_flash) / 0.5;
            if (0.0..1.0).contains(&p) {
                key_color = theme::WHITE.lerp(theme::GOLD, p);
                label_color = theme::WHITE.lerp(theme::GREY, p);
            }
        }
        // Urgent (comète, événement) : le libellé reste en évidence.
        if item.label.ends_with('!') {
            label_color = theme::TEXT.lerp(theme::CYAN, 0.5 + 0.5 * (now * 4.0).sin());
        }
        x = draw::text(buf, x, area.y, "[", theme::fg(theme::BORDER), max);
        x = draw::text(buf, x, area.y, item.key, theme::bold(key_color), max);
        x = draw::text(buf, x, area.y, "] ", theme::fg(theme::BORDER), max);
        x = draw::text(buf, x, area.y, &item.label, theme::fg(label_color), max);
        x = draw::text(buf, x, area.y, "  ", theme::fg(label_color), max);
    }
}
