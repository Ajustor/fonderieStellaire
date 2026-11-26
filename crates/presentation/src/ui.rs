//! État d'interface purement visuel : focus, sélections, défilement, popup.

use domain::UpgradeId;

use crate::layout::GameLayout;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Panel {
    #[default]
    Generators,
    Upgrades,
}

impl Panel {
    pub fn toggle(self) -> Self {
        match self {
            Panel::Generators => Panel::Upgrades,
            Panel::Upgrades => Panel::Generators,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Popup {
    pub opened_at: f64,
    pub closing_at: Option<f64>,
}

/// Durée d'ouverture / fermeture d'une popup.
pub const POPUP_ANIM: f64 = 0.15;

impl Popup {
    /// Échelle courante (0 → fermée, 1 → ouverte).
    pub fn scale(&self, now: f64) -> f64 {
        let open = ((now - self.opened_at) / POPUP_ANIM).clamp(0.0, 1.0);
        match self.closing_at {
            Some(c) => open.min(1.0 - ((now - c) / POPUP_ANIM).clamp(0.0, 1.0)),
            None => open,
        }
    }

    pub fn is_closed(&self, now: f64) -> bool {
        matches!(self.closing_at, Some(c) if now - c >= POPUP_ANIM)
    }
}

#[derive(Debug, Default)]
pub struct Ui {
    pub focus: Panel,
    /// Index dans la liste des générateurs visibles.
    pub gen_sel: usize,
    pub gen_scroll: usize,
    /// Index dans la liste des améliorations proposées.
    pub upg_sel: usize,
    pub upg_scroll: usize,
    pub popup: Option<Popup>,
    /// La popup de confirmation concerne le Big Crunch (et non une Supernova).
    pub popup_crunch: bool,
    /// Écran « Infos » (touche I) et son défilement.
    pub codex: Option<Popup>,
    pub codex_scroll: usize,
    /// Atelier stellaire (touche E) et sa sélection.
    pub shop: Option<Popup>,
    pub shop_sel: usize,
    /// Fenêtre des Défis (touche D), sélection, et confirmation armée.
    pub challenges: Option<Popup>,
    pub challenges_sel: usize,
    pub challenge_armed: bool,
    /// Laboratoire (touche L) et sa sélection.
    pub lab: Option<Popup>,
    pub lab_sel: usize,
    /// Options (touche R), sélection, et confirmation armée de l'effacement.
    pub options: Option<Popup>,
    pub options_sel: usize,
    pub options_armed: bool,
    /// Effacement de la partie en cours : instant de départ.
    pub wipe_at: Option<f64>,
    /// Cosmogonie (touche G), sélection, et confirmation armée du Big Bang.
    pub cosmos: Option<Popup>,
    pub cosmos_sel: usize,
    pub cosmos_armed: bool,
    /// Écran des portails (choix d'univers au Big Crunch, ou consultation).
    pub portal: Option<crate::widgets::portals::PortalState>,
    /// Lignes remontées dans le journal (0 = dernières entrées).
    pub journal_scroll: usize,
    pub debug: bool,
    pub layout: GameLayout,
    /// Ligne écran de chaque générateur affiché (pour les étincelles d'achat).
    pub gen_rows: [Option<u16>; crate::theme::GEN_SLOTS],
    /// Ligne écran de chaque amélioration affichée.
    pub upg_rows: Vec<(UpgradeId, u16)>,
    /// Instant du dernier changement de focus.
    pub focus_changed_at: f64,
}

/// Ajuste le défilement pour garder `sel` visible dans une fenêtre de `visible` éléments.
pub fn keep_visible(sel: usize, scroll: &mut usize, visible: usize) {
    let visible = visible.max(1);
    if sel < *scroll {
        *scroll = sel;
    } else if sel >= *scroll + visible {
        *scroll = sel + 1 - visible;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrolling_keeps_selection_visible() {
        let mut scroll = 0;
        keep_visible(5, &mut scroll, 3);
        assert_eq!(scroll, 3);
        keep_visible(1, &mut scroll, 3);
        assert_eq!(scroll, 1);
        keep_visible(2, &mut scroll, 3);
        assert_eq!(scroll, 1);
    }

    #[test]
    fn popup_scale() {
        let mut p = Popup {
            opened_at: 1.0,
            closing_at: None,
        };
        assert_eq!(p.scale(1.0), 0.0);
        assert_eq!(p.scale(2.0), 1.0);
        p.closing_at = Some(3.0);
        assert!(p.scale(3.075) < 0.6);
        assert!(p.is_closed(3.2));
    }
}
