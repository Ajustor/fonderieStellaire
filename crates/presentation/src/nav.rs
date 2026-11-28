//! Navigation entre les écrans de la version terminal : barre d'onglets
//! (`Tab` / `Maj+Tab`), menu central (`Échap`) et panneau d'aide (`?`).
//!
//! Les écrans restent des fenêtres par-dessus la fonderie ; ce module dit
//! lesquels sont débloqués, lequel est ouvert, et passe de l'un à l'autre.
//! Rien de ce que le joueur n'a pas découvert n'est proposé.

use application::GameSnapshot;

use crate::app::App;
use crate::input::Intent;
use crate::ui::Popup;

/// Groupe d'écrans dans la barre d'onglets (mêmes groupes que le mobile).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Foundry,
    Prestige,
    Cosmos,
    More,
}

impl Group {
    pub fn name(self) -> &'static str {
        match self {
            Group::Foundry => "Fonderie",
            Group::Prestige => "Prestige",
            Group::Cosmos => "Cosmos",
            Group::More => "Plus",
        }
    }
}

/// Un écran de la barre d'onglets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenId {
    Foundry,
    Shop,
    Challenges,
    Lab,
    Universes,
    Cosmogony,
    StarMap,
    Relics,
    Mega,
    Daily,
    Info,
    Options,
    Leaderboard,
}

/// Ordre de la barre (et de `Tab`).
pub const ALL: [ScreenId; 13] = [
    ScreenId::Foundry,
    ScreenId::Shop,
    ScreenId::Challenges,
    ScreenId::Lab,
    ScreenId::Universes,
    ScreenId::Cosmogony,
    ScreenId::StarMap,
    ScreenId::Relics,
    ScreenId::Mega,
    ScreenId::Daily,
    ScreenId::Info,
    ScreenId::Options,
    ScreenId::Leaderboard,
];

impl ScreenId {
    pub fn name(self) -> &'static str {
        match self {
            ScreenId::Foundry => "Fonderie",
            ScreenId::Shop => "Atelier",
            ScreenId::Challenges => "Défis",
            ScreenId::Lab => "Laboratoire",
            ScreenId::Universes => "Univers",
            ScreenId::Cosmogony => "Cosmogonie",
            ScreenId::StarMap => "Carte stellaire",
            ScreenId::Relics => "Reliques",
            ScreenId::Mega => "Mégastructures",
            ScreenId::Daily => "Défi du jour",
            ScreenId::Info => "Infos",
            ScreenId::Options => "Options",
            ScreenId::Leaderboard => "Classement",
        }
    }

    /// Nom court (barre d'onglets étroite).
    pub fn short(self) -> &'static str {
        match self {
            ScreenId::Lab => "Labo",
            ScreenId::Cosmogony => "Cosmo",
            ScreenId::StarMap => "Carte",
            ScreenId::Mega => "Méga",
            ScreenId::Daily => "Jour",
            ScreenId::Leaderboard => "Classt",
            other => other.name(),
        }
    }

    /// Touche qui l'ouvre depuis la fonderie (le Classement n'en a pas).
    pub fn key(self) -> Option<char> {
        Some(match self {
            ScreenId::Foundry | ScreenId::Leaderboard => return None,
            ScreenId::Shop => 'E',
            ScreenId::Challenges => 'D',
            ScreenId::Lab => 'L',
            ScreenId::Universes => 'V',
            ScreenId::Cosmogony => 'G',
            ScreenId::StarMap => 'X',
            ScreenId::Relics => 'R',
            ScreenId::Mega => 'B',
            ScreenId::Daily => 'J',
            ScreenId::Info => 'I',
            ScreenId::Options => 'O',
        })
    }

    pub fn group(self) -> Group {
        match self {
            ScreenId::Foundry => Group::Foundry,
            ScreenId::Shop | ScreenId::Challenges | ScreenId::Lab | ScreenId::Universes => {
                Group::Prestige
            }
            ScreenId::Cosmogony
            | ScreenId::StarMap
            | ScreenId::Relics
            | ScreenId::Mega
            | ScreenId::Daily => Group::Cosmos,
            ScreenId::Info | ScreenId::Options | ScreenId::Leaderboard => Group::More,
        }
    }

    /// Une ligne d'explication (menu).
    pub fn blurb(self) -> &'static str {
        match self {
            ScreenId::Foundry => "Générateurs, améliorations et récolte",
            ScreenId::Shop => "Bonus permanents payés en fragments",
            ScreenId::Challenges => "Un cycle contraint, une récompense",
            ScreenId::Lab => "Recherches payées en matière noire",
            ScreenId::Universes => "Univers parallèles et leurs Clés",
            ScreenId::Cosmogony => "Big Bang, forge, Grand Attracteur",
            ScreenId::StarMap => "Expéditions vers les systèmes voisins",
            ScreenId::Relics => "Collection, emplacements et ensembles",
            ScreenId::Mega => "Bâtir sur la grille",
            ScreenId::Daily => "Dix minutes dans un univers de poche",
            ScreenId::Info => "Progression, production, succès",
            ScreenId::Options => "Couleurs, synchro, recommencer",
            ScreenId::Leaderboard => "Meilleures fonderies, Défi du jour",
        }
    }

    /// Débloqué (et donc proposé) dans cette partie.
    pub fn unlocked(self, snap: &GameSnapshot) -> bool {
        match self {
            ScreenId::Foundry | ScreenId::Info | ScreenId::Options => true,
            ScreenId::Shop => snap.supernovas > 0 || snap.fragments > 0.0 || snap.crunches > 0,
            ScreenId::Challenges => snap.challenges.iter().any(|c| c.unlocked),
            ScreenId::Lab => snap.lab_unlocked,
            ScreenId::Cosmogony => snap.universes_unlocked,
            ScreenId::Universes => snap.universes_unlocked,
            ScreenId::StarMap | ScreenId::Relics => snap.starmap_unlocked,
            ScreenId::Mega => snap.megastructures_unlocked,
            ScreenId::Daily => snap.daily_unlocked || snap.daily.is_some(),
            // Le classement passe par la synchronisation : proposé dès que
            // la fonderie a vraiment commencé.
            ScreenId::Leaderboard => snap.supernovas > 0 || snap.crunches > 0,
        }
    }

    /// Intention qui ouvre l'écran (le Classement s'ouvre à part).
    fn intent(self) -> Option<Intent> {
        Some(match self {
            ScreenId::Foundry | ScreenId::Leaderboard => return None,
            ScreenId::Shop => Intent::Shop,
            ScreenId::Challenges => Intent::Challenges,
            ScreenId::Lab => Intent::Lab,
            ScreenId::Universes => Intent::Universes,
            ScreenId::Cosmogony => Intent::Cosmos,
            ScreenId::StarMap => Intent::StarMap,
            ScreenId::Relics => Intent::Relics,
            ScreenId::Mega => Intent::Megastructures,
            ScreenId::Daily => Intent::Daily,
            ScreenId::Info => Intent::Info,
            ScreenId::Options => Intent::Options,
        })
    }
}

/// Écrans débloqués, dans l'ordre de la barre.
pub fn available(snap: &GameSnapshot) -> Vec<ScreenId> {
    ALL.into_iter().filter(|s| s.unlocked(snap)).collect()
}

/// Écran voisin de `current` dans `screens` (`step` = ±1, en boucle).
pub fn neighbour(screens: &[ScreenId], current: ScreenId, step: isize) -> ScreenId {
    if screens.is_empty() {
        return ScreenId::Foundry;
    }
    let n = screens.len() as isize;
    let i = screens.iter().position(|&s| s == current).unwrap_or(0) as isize;
    screens[(i + step).rem_euclid(n) as usize]
}

/// Ligne du menu central.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuEntry {
    Screen(ScreenId),
    /// Événement cosmique en attente (`Z`).
    Event,
    Help,
    Quit,
}

/// Lignes du menu : écrans débloqués (hors fonderie), événement en attente,
/// aide, quitter.
pub fn menu_entries(snap: &GameSnapshot) -> Vec<MenuEntry> {
    let mut v: Vec<MenuEntry> = available(snap)
        .into_iter()
        .filter(|&s| s != ScreenId::Foundry)
        .map(MenuEntry::Screen)
        .collect();
    if snap.pending_event.is_some() {
        v.push(MenuEntry::Event);
    }
    v.push(MenuEntry::Help);
    v.push(MenuEntry::Quit);
    v
}

impl App {
    /// Écran de la barre actuellement ouvert (Fonderie si aucun).
    pub fn current_screen(&self) -> ScreenId {
        let ui = &self.ui;
        let open = |p: &Option<Popup>| p.is_some_and(|p| p.closing_at.is_none());
        if open(&ui.codex) {
            ScreenId::Info
        } else if open(&ui.shop) {
            ScreenId::Shop
        } else if open(&ui.challenges) {
            ScreenId::Challenges
        } else if open(&ui.lab) {
            ScreenId::Lab
        } else if open(&ui.daily) {
            ScreenId::Daily
        } else if open(&ui.mega) {
            ScreenId::Mega
        } else if open(&ui.relics) {
            ScreenId::Relics
        } else if open(&ui.leaderboard) {
            ScreenId::Leaderboard
        } else if open(&ui.options) {
            ScreenId::Options
        } else if open(&ui.cosmos) {
            ScreenId::Cosmogony
        } else if ui.starmap.is_some() {
            ScreenId::StarMap
        } else if ui.portal.is_some_and(|p| !p.crunch) {
            ScreenId::Universes
        } else {
            ScreenId::Foundry
        }
    }

    /// Une fenêtre qui n'est pas un écran de la barre (confirmation, choix
    /// d'un événement, saisie…) bloque `Tab`.
    pub(crate) fn nav_blocked(&self) -> bool {
        let ui = &self.ui;
        ui.popup.is_some()
            || ui.event.is_some()
            || ui.portal.is_some_and(|p| p.crunch)
            || ui.pseudo_input.is_some()
            || ui.options_armed
            || ui.cosmos_armed
            || ui.challenge_armed
    }

    /// Ferme l'écran de la barre ouvert, sans animation.
    fn close_screens(&mut self) {
        let ui = &mut self.ui;
        ui.codex = None;
        ui.shop = None;
        ui.challenges = None;
        ui.lab = None;
        ui.daily = None;
        ui.mega = None;
        ui.relics = None;
        ui.leaderboard = None;
        ui.options = None;
        ui.options_sync = false;
        ui.cosmos = None;
        ui.starmap = None;
        if ui.portal.is_some_and(|p| !p.crunch) {
            ui.portal = None;
        }
    }

    /// Ouvre `screen` à la place de l'écran courant.
    pub(crate) fn open_screen(&mut self, screen: ScreenId) {
        self.close_screens();
        self.ui.menu = None;
        if screen == ScreenId::Leaderboard {
            self.open_leaderboard();
        } else if let Some(intent) = screen.intent() {
            self.on_intent(intent);
        }
    }

    /// `Tab` / `Maj+Tab` : écran suivant ou précédent de la barre.
    pub(crate) fn cycle_screen(&mut self, step: isize) {
        if self.nav_blocked() {
            return;
        }
        let screens = available(&self.snap);
        let next = neighbour(&screens, self.current_screen(), step);
        self.open_screen(next);
    }

    /// `Échap` dans la fonderie : menu central.
    pub(crate) fn open_menu(&mut self) {
        self.ui.menu = Some(Popup {
            opened_at: self.now,
            closing_at: None,
        });
        self.ui.menu_sel = 0;
    }

    /// Touches du menu central.
    pub(crate) fn on_menu_intent(&mut self, intent: Intent) {
        let entries = menu_entries(&self.snap);
        let sel = self.ui.menu_sel.min(entries.len().saturating_sub(1));
        match intent {
            Intent::Up => self.ui.menu_sel = sel.saturating_sub(1),
            Intent::Down => self.ui.menu_sel = (sel + 1).min(entries.len().saturating_sub(1)),
            Intent::Buy | Intent::Yes => match entries.get(sel) {
                Some(MenuEntry::Screen(s)) => self.open_screen(*s),
                Some(MenuEntry::Event) => {
                    self.ui.menu = None;
                    self.on_intent(Intent::Event);
                }
                Some(MenuEntry::Help) => {
                    self.ui.menu = None;
                    self.open_help();
                }
                Some(MenuEntry::Quit) => self.quit = true,
                None => {}
            },
            Intent::Escape | Intent::No => self.ui.menu = None,
            Intent::Quit => self.quit = true,
            Intent::Help => {
                self.ui.menu = None;
                self.open_help();
            }
            Intent::NextScreen | Intent::PrevScreen => {
                self.ui.menu = None;
                self.cycle_screen(if intent == Intent::NextScreen { 1 } else { -1 });
            }
            // La lettre d'un écran l'ouvre directement.
            other => {
                if let Some(s) = entries.iter().find_map(|e| match e {
                    MenuEntry::Screen(s) if s.intent() == Some(other) => Some(*s),
                    _ => None,
                }) {
                    self.open_screen(s);
                } else if other == Intent::Event && entries.contains(&MenuEntry::Event) {
                    self.ui.menu = None;
                    self.on_intent(Intent::Event);
                }
            }
        }
    }

    /// `?` / `F1` : panneau d'aide.
    pub(crate) fn open_help(&mut self) {
        self.ui.help = Some(Popup {
            opened_at: self.now,
            closing_at: None,
        });
        self.ui.help_scroll = 0;
    }

    /// Touches du panneau d'aide.
    pub(crate) fn on_help_intent(&mut self, intent: Intent) {
        match intent {
            Intent::Up | Intent::JournalUp => {
                self.ui.help_scroll = self.ui.help_scroll.saturating_sub(1)
            }
            Intent::Down | Intent::JournalDown => self.ui.help_scroll += 1,
            Intent::Quit => self.quit = true,
            _ => self.ui.help = None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_wraps_around_the_unlocked_screens() {
        let s = [ScreenId::Foundry, ScreenId::Shop, ScreenId::Info];
        assert_eq!(neighbour(&s, ScreenId::Foundry, 1), ScreenId::Shop);
        assert_eq!(neighbour(&s, ScreenId::Info, 1), ScreenId::Foundry);
        assert_eq!(neighbour(&s, ScreenId::Foundry, -1), ScreenId::Info);
        // Écran courant absent (verrouillé entre-temps) : depuis le début.
        assert_eq!(neighbour(&s, ScreenId::Mega, 1), ScreenId::Shop);
    }

    #[test]
    fn a_new_game_only_offers_what_is_discovered() {
        let snap = GameSnapshot::default();
        assert_eq!(
            available(&snap),
            [ScreenId::Foundry, ScreenId::Info, ScreenId::Options]
        );
        let menu = menu_entries(&snap);
        assert_eq!(
            menu,
            [
                MenuEntry::Screen(ScreenId::Info),
                MenuEntry::Screen(ScreenId::Options),
                MenuEntry::Help,
                MenuEntry::Quit
            ]
        );
    }

    #[test]
    fn blurbs_fit_the_menu() {
        for s in ALL {
            assert!(s.blurb().chars().count() <= 40, "{}", s.blurb());
        }
    }

    #[test]
    fn letters_are_unique() {
        let mut keys: Vec<char> = ALL.iter().filter_map(|s| s.key()).collect();
        let n = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), n);
    }
}
