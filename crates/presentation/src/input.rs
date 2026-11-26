//! Traduction des événements clavier en intentions.
//!
//! Sous Windows, crossterm émet `Press` **et** `Release` : on ne garde que `Press`,
//! sinon chaque touche agirait deux fois.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    Harvest,
    /// Répétition d'Espace maintenu (récolte continue, si l'amélioration est achetée).
    HarvestHeld,
    Up,
    Down,
    NextPanel,
    PrevPanel,
    Buy,
    /// Achat rapide du générateur n° N (0 = premier).
    QuickBuy(usize),
    CycleMode,
    CatchComet,
    Supernova,
    Save,
    ToggleEffects,
    ToggleDebug,
    /// `A` : acheter toutes les améliorations abordables.
    BuyAll,
    /// `E` : Atelier stellaire.
    Shop,
    /// `D` : Défis.
    Challenges,
    /// `U` : Big Crunch (confirmation).
    BigCrunch,
    /// `L` : Laboratoire de matière noire.
    Lab,
    /// `V` : univers parallèles (consultation).
    Universes,
    /// `T` : activer / arrêter l'automate.
    ToggleAuto,
    /// `PgUp` / `PgDn` : remonter / redescendre dans le journal.
    JournalUp,
    JournalDown,
    /// Écran « Infos » : succès et mécaniques.
    Info,
    Quit,
    /// `O` dans une popup.
    Yes,
    /// `N` dans une popup.
    No,
    /// `Échap` : ferme une popup, sinon quitte.
    Escape,
    /// Toute autre touche (sert à « appuie sur une touche »).
    Other,
}

/// Sans événements `Release` (la plupart des terminaux Linux), deux appuis plus
/// rapprochés que ceci sont considérés comme la répétition automatique d'une
/// touche maintenue (≈ 25–33 répétitions/s, bien plus vite qu'un humain qui tape).
pub const AUTO_REPEAT_GAP: f64 = 0.075;
/// Avec événements `Release` (Windows), un appui arrivant après ce délai sans
/// répétition est un nouvel appui, même si un `Release` s'est perdu en route.
pub const HOLD_TIMEOUT: f64 = 0.6;

/// Distingue un appui sur Espace d'une touche maintenue.
///
/// - Si le terminal envoie des `Release` (Windows) : exact, la touche est
///   maintenue tant qu'aucun `Release` n'est arrivé.
/// - Sinon : heuristique sur l'intervalle entre deux appuis.
#[derive(Clone, Copy, Debug, Default)]
pub struct HoldDetector {
    releases_seen: bool,
    down: bool,
    last_press: Option<f64>,
}

impl HoldDetector {
    /// À appeler pour tout `Release` reçu (quelle que soit la touche) :
    /// révèle que le terminal signale les relâchements.
    pub fn saw_release(&mut self) {
        self.releases_seen = true;
    }

    /// Classe un événement de la touche Espace ; `t` en secondes (horloge monotone).
    pub fn on_space(&mut self, kind: KeyEventKind, t: f64) -> Option<Intent> {
        let since = self.last_press.map(|p| t - p);
        match kind {
            KeyEventKind::Release => {
                self.releases_seen = true;
                self.down = false;
                None
            }
            KeyEventKind::Repeat => {
                self.down = true;
                self.last_press = Some(t);
                Some(Intent::HarvestHeld)
            }
            KeyEventKind::Press => {
                let held = if self.releases_seen {
                    self.down && since.is_some_and(|s| s < HOLD_TIMEOUT)
                } else {
                    since.is_some_and(|s| s < AUTO_REPEAT_GAP)
                };
                self.down = true;
                self.last_press = Some(t);
                Some(if held {
                    Intent::HarvestHeld
                } else {
                    Intent::Harvest
                })
            }
        }
    }
}

pub fn map_event(event: &Event) -> Option<Intent> {
    match event {
        Event::Key(key) => map_key(key),
        _ => None,
    }
}

pub fn map_key(key: &KeyEvent) -> Option<Intent> {
    if key.kind != KeyEventKind::Press {
        return None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') | KeyCode::Char('C') => Some(Intent::Quit),
            _ => Some(Intent::Other),
        };
    }
    let intent = match key.code {
        KeyCode::Char(' ') => Intent::Harvest,
        KeyCode::Up => Intent::Up,
        KeyCode::Down => Intent::Down,
        KeyCode::Tab | KeyCode::Right => Intent::NextPanel,
        KeyCode::BackTab | KeyCode::Left => Intent::PrevPanel,
        KeyCode::Enter => Intent::Buy,
        KeyCode::Esc => Intent::Escape,
        KeyCode::F(3) => Intent::ToggleDebug,
        KeyCode::PageUp => Intent::JournalUp,
        KeyCode::PageDown => Intent::JournalDown,
        KeyCode::Char(c) => match c.to_ascii_lowercase() {
            'k' => Intent::Up,
            'j' => Intent::Down,
            'b' => Intent::Buy,
            'm' => Intent::CycleMode,
            'c' => Intent::CatchComet,
            'p' => Intent::Supernova,
            's' => Intent::Save,
            'f' => Intent::ToggleEffects,
            'q' => Intent::Quit,
            'i' => Intent::Info,
            'a' => Intent::BuyAll,
            'e' => Intent::Shop,
            'd' => Intent::Challenges,
            'u' => Intent::BigCrunch,
            'l' => Intent::Lab,
            'v' => Intent::Universes,
            't' => Intent::ToggleAuto,
            'o' | 'y' => Intent::Yes,
            'n' => Intent::No,
            d @ '1'..='9' => Intent::QuickBuy((d as u8 - b'1') as usize),
            _ => Intent::Other,
        },
        _ => Intent::Other,
    };
    Some(intent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventState;

    fn key(code: KeyCode, kind: KeyEventKind, mods: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: mods,
            kind,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn release_events_are_ignored() {
        let k = key(
            KeyCode::Char(' '),
            KeyEventKind::Release,
            KeyModifiers::NONE,
        );
        assert_eq!(map_key(&k), None);
        let k = key(KeyCode::Char(' '), KeyEventKind::Press, KeyModifiers::NONE);
        assert_eq!(map_key(&k), Some(Intent::Harvest));
    }

    #[test]
    fn case_insensitive_and_digits() {
        for c in ['m', 'M'] {
            let k = key(KeyCode::Char(c), KeyEventKind::Press, KeyModifiers::SHIFT);
            assert_eq!(map_key(&k), Some(Intent::CycleMode));
        }
        let k = key(KeyCode::Char('3'), KeyEventKind::Press, KeyModifiers::NONE);
        assert_eq!(map_key(&k), Some(Intent::QuickBuy(2)));
    }

    #[test]
    fn hold_detection_with_release_events() {
        use KeyEventKind::*;
        let mut d = HoldDetector::default();
        d.saw_release();
        assert_eq!(d.on_space(Press, 0.0), Some(Intent::Harvest));
        assert_eq!(
            d.on_space(Press, 0.5),
            Some(Intent::HarvestHeld),
            "pas de Release : maintenue"
        );
        assert_eq!(d.on_space(Press, 0.53), Some(Intent::HarvestHeld));
        assert_eq!(d.on_space(Release, 0.6), None);
        assert_eq!(
            d.on_space(Press, 0.62),
            Some(Intent::Harvest),
            "relâchée puis rappuyée"
        );
        // Release perdu : après un long silence, c'est un nouvel appui.
        assert_eq!(d.on_space(Press, 5.0), Some(Intent::Harvest));
    }

    #[test]
    fn hold_detection_by_timing_without_release_events() {
        use KeyEventKind::*;
        let mut d = HoldDetector::default();
        assert_eq!(d.on_space(Press, 0.0), Some(Intent::Harvest));
        assert_eq!(
            d.on_space(Press, 0.12),
            Some(Intent::Harvest),
            "tapotage rapide"
        );
        assert_eq!(
            d.on_space(Press, 0.15),
            Some(Intent::HarvestHeld),
            "répétition auto"
        );
        assert_eq!(d.on_space(Press, 0.183), Some(Intent::HarvestHeld));
        assert_eq!(d.on_space(Repeat, 0.2), Some(Intent::HarvestHeld));
    }

    #[test]
    fn ctrl_c_quits() {
        let k = key(
            KeyCode::Char('c'),
            KeyEventKind::Press,
            KeyModifiers::CONTROL,
        );
        assert_eq!(map_key(&k), Some(Intent::Quit));
        let k = key(KeyCode::Char('c'), KeyEventKind::Press, KeyModifiers::NONE);
        assert_eq!(map_key(&k), Some(Intent::CatchComet));
    }
}
