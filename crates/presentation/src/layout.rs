//! Découpage de l'écran de jeu.

use ratatui::layout::Rect;

pub const MIN_WIDTH: u16 = 80;
pub const MIN_HEIGHT: u16 = 24;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GameLayout {
    pub header: Rect,
    pub generators: Rect,
    pub sky: Rect,
    pub upgrades: Rect,
    pub production: Rect,
    pub journal: Rect,
    pub help: Rect,
}

impl GameLayout {
    /// Panneaux dans l'ordre d'apparition en cascade.
    pub fn panels(&self) -> [Rect; 7] {
        [
            self.header,
            self.generators,
            self.sky,
            self.upgrades,
            self.production,
            self.journal,
            self.help,
        ]
    }
}

pub fn is_too_small(area: Rect) -> bool {
    area.width < MIN_WIDTH || area.height < MIN_HEIGHT
}

/// Hauteur visée pour le panneau des améliorations quand il a le focus : la
/// liste et l'encart de détail complet tiennent sans rien couper.
const UPGRADES_FOCUSED_H: u16 = 18;

/// Découpe l'écran. `expand` (0 → 1) fait grandir le panneau des améliorations
/// aux dépens du ciel quand il a le focus (animé par l'appelant).
pub fn game_layout(area: Rect, expand: f64) -> GameLayout {
    let header_h = 5u16.min(area.height);
    let help_h = 1u16;
    let bottom_h = if area.height >= 32 {
        6
    } else if area.height >= 28 {
        5
    } else {
        4
    };
    let middle_h = area.height.saturating_sub(header_h + help_h + bottom_h);

    let left_w = (area.width as u32 * 55 / 100) as u16;
    let right_w = area.width - left_w;

    let header = Rect::new(area.x, area.y, area.width, header_h);
    let mid_y = area.y + header_h;

    let sky_normal = ((middle_h as u32 * 55 / 100) as u16).clamp(6.min(middle_h), 12);
    let mut sky_focused = middle_h.saturating_sub(UPGRADES_FOCUSED_H).min(sky_normal);
    if sky_focused < 4 {
        sky_focused = 0;
    }
    let e = expand.clamp(0.0, 1.0);
    let sky_h = (sky_normal as f64 + (sky_focused as f64 - sky_normal as f64) * e).round() as u16;
    let upgrades_h = middle_h - sky_h;

    let generators = Rect::new(area.x, mid_y, left_w, middle_h);
    let sky = Rect::new(area.x + left_w, mid_y, right_w, sky_h);
    let upgrades = Rect::new(area.x + left_w, mid_y + sky_h, right_w, upgrades_h);

    let bottom_y = mid_y + middle_h;
    let production = Rect::new(area.x, bottom_y, left_w, bottom_h);
    let journal = Rect::new(area.x + left_w, bottom_y, right_w, bottom_h);
    let help = Rect::new(area.x, bottom_y + bottom_h, area.width, help_h);

    GameLayout {
        header,
        generators,
        sky,
        upgrades,
        production,
        journal,
        help,
    }
}

/// Rectangle centré de taille donnée (bornée par `area`).
pub fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(w: u16, h: u16) {
        let area = Rect::new(0, 0, w, h);
        let l = game_layout(area, 0.0);
        let f = game_layout(area, 1.0);
        assert!(f.upgrades.height >= l.upgrades.height);
        assert_eq!(
            f.sky.height + f.upgrades.height,
            l.sky.height + l.upgrades.height
        );
        for r in l.panels() {
            assert!(area.contains(r.as_position()) || r.is_empty(), "{r:?}");
            assert!(r.right() <= w && r.bottom() <= h, "{r:?} dans {w}×{h}");
        }
        assert_eq!(l.help.bottom(), h);
        assert!(l.sky.height >= 6);
        assert!(l.upgrades.height >= 5);
        assert!(l.generators.height >= 12);
    }

    #[test]
    fn layouts_fit() {
        check(80, 24);
        check(100, 30);
        check(120, 40);
        check(200, 60);
    }

    #[test]
    fn too_small() {
        assert!(is_too_small(Rect::new(0, 0, 79, 30)));
        assert!(is_too_small(Rect::new(0, 0, 100, 23)));
        assert!(!is_too_small(Rect::new(0, 0, 80, 24)));
    }
}
