//! Widgets de l'écran de jeu. Ils lisent le `GameSnapshot` et l'état visuel,
//! et dessinent directement dans le `Buffer`.

pub mod ceremony;
pub mod challenges;
pub mod codex;
pub mod cosmos;
pub mod daily;
pub mod event;
pub mod generators;
pub mod header;
pub mod help_bar;
pub mod journal;
pub mod lab;
pub mod mega;
pub mod modal;
pub mod options;
pub mod portals;
pub mod relics;
pub mod shop;
pub mod sky;
pub mod sparkline;
pub mod starmap;
pub mod toast;
pub mod upgrades;

use application::GameSnapshot;
use domain::Catalog;
use ratatui::layout::Rect;

/// Contexte de rendu partagé par tous les widgets.
pub struct Ctx<'a> {
    pub snap: &'a GameSnapshot,
    pub catalog: &'a Catalog,
    /// Secondes depuis le lancement.
    pub now: f64,
    /// Durée de la frame précédente.
    pub dt: f64,
    pub reduced: bool,
}

/// Zone intérieure d'un panneau bordé.
pub fn inner(area: Rect) -> Rect {
    Rect::new(
        area.x.saturating_add(1),
        area.y.saturating_add(1),
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    )
}
