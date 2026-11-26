//! Mode mobile (Android) : un écran plein par onglet, une barre d'onglets
//! native en bas, des cartes avec leur propre bouton au lieu de « choisir puis
//! valider ». Activé par `App::set_mobile(true)` ; la version terminal ne
//! passe jamais par ce module et garde ses écrans tels quels.
//!
//! Chaque zone touchable est enregistrée pendant le rendu (`HitMap`) : le
//! hit-test ne peut donc pas diverger de ce qui est affiché.

mod cosmos;
mod input;
mod screens;

use domain::{BuyMode, ConstellationId, GeneratorId, PerkId, UpgradeId};
use ratatui::layout::Rect;

use crate::input::Intent;

pub use screens::{render, render_controls, MobileView};

/// Onglets de la barre du bas.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Foundry,
    Factories,
    Upgrade,
    Cosmos,
    More,
}

impl Tab {
    pub const ALL: [Tab; 5] = [
        Tab::Foundry,
        Tab::Factories,
        Tab::Upgrade,
        Tab::Cosmos,
        Tab::More,
    ];

    pub fn index(self) -> usize {
        Tab::ALL.iter().position(|&t| t == self).unwrap_or(0)
    }
}

/// État d'un onglet pour la barre native.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabInfo {
    pub tab: Tab,
    pub unlocked: bool,
    pub active: bool,
    /// Une action attend dans l'onglet (pastille).
    pub badge: bool,
}

/// Demande adressée à la plateforme (sélecteur de fichiers…).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    ExportSave,
    ImportSave,
    /// Synchronisation entre appareils (QR code, scan, code à taper).
    Sync,
}

/// Ce que fait une zone touchée.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hit {
    /// Ciel : récolte (maintenir = récolte continue).
    Harvest,
    CatchComet,
    BuyGenerator(GeneratorId),
    BuyUpgrade(UpgradeId),
    BuyPerk(PerkId),
    BuyConstellation(ConstellationId),
    SetBuyMode(BuyMode),
    ToggleAutoBuy,
    BuyAllUpgrades,
    ToggleProduction,
    SubTab(usize),
    /// Aller à un onglet et un sous-onglet.
    Open(Tab, usize),
    /// Fiche détail (appui long sur une carte).
    Details(Detail),
    CloseDetail,
    CycleColors,
    CycleFrameRate,
    ToggleReducedEffects,
    Request(Request),
    /// Intention clavier : ouvre un écran existant ou le pilote (popups).
    Key(Intent),
    /// Zone qui absorbe le toucher (sous une fiche ou une confirmation).
    Block,
    OpenPage(Page),
    ClosePage,
    /// Ouvre la confirmation « glisser pour confirmer ».
    Ask(Confirm),
    /// Curseur à faire glisser de la confirmation.
    Slider,
    CancelConfirm,
    EventChoice(usize),
    DailyToggle,
    CloseAbsence,
    ToggleVibrations,
    /// Ferme la bulle d'aide en cours.
    DismissTip(Tip),
    /// Recherche du Labo, par position d'affichage.
    Lab(usize),
    /// Élément de la Cosmogonie, par index.
    Cosmogony(usize),
    ToggleRelic(usize),
    /// Carte stellaire : glisser déplace la vue, toucher choisit un système.
    Map,
    SendFleet,
    ToggleAutoFleets,
    Convergence,
    MegaCell(usize),
    Build(domain::StructureKind),
}

impl Hit {
    /// Se répète tant que le doigt reste posé (achats en rafale).
    pub fn repeats(self) -> bool {
        matches!(
            self,
            Hit::BuyGenerator(_) | Hit::BuyUpgrade(_) | Hit::BuyConstellation(_)
        )
    }
}

/// Contenu d'une fiche détail.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Detail {
    Generator(GeneratorId),
    Upgrade(UpgradeId),
    Perk(usize),
    Constellation(usize),
    Relic(usize),
    Achievement(usize),
}

/// Bulles de première partie : elles montrent l'onglet où agir, une fois.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tip {
    FirstGenerator,
    FirstUpgrade,
    Supernova,
    Atelier,
    Cosmos,
}

impl Tip {
    pub const ALL: [Tip; 5] = [
        Tip::FirstGenerator,
        Tip::FirstUpgrade,
        Tip::Supernova,
        Tip::Atelier,
        Tip::Cosmos,
    ];

    pub fn bit(self) -> u32 {
        1 << Tip::ALL.iter().position(|&t| t == self).unwrap_or(0)
    }

    /// Onglet vers lequel pointe la bulle.
    pub fn tab(self) -> Tab {
        match self {
            Tip::FirstGenerator => Tab::Factories,
            Tip::FirstUpgrade | Tip::Atelier => Tab::Upgrade,
            Tip::Supernova => Tab::Foundry,
            Tip::Cosmos => Tab::Cosmos,
        }
    }

    pub fn text(self) -> &'static str {
        match self {
            Tip::FirstGenerator => "Ta première Sonde est à portée : achète-la dans Usines.",
            Tip::FirstUpgrade => "Une amélioration est abordable : jette un œil à Améliorer.",
            Tip::Supernova => "Supernova possible ! Le bouton est en bas de Fonderie.",
            Tip::Atelier => "Tes fragments se dépensent dans Améliorer › Atelier.",
            Tip::Cosmos => "Le Cosmos s'ouvre : Défis et bien plus t'attendent.",
        }
    }

    /// La situation qui appelle la bulle.
    pub fn applies(self, s: &application::GameSnapshot) -> bool {
        match self {
            Tip::FirstGenerator => {
                s.generators.iter().any(|g| g.affordable) && s.owned.iter().all(|&n| n == 0)
            }
            Tip::FirstUpgrade => {
                s.upgrades.iter().any(|u| u.affordable) && s.progress.upgrades_bought == 0
            }
            Tip::Supernova => s.fragments_available >= 1.0 && s.supernovas == 0,
            Tip::Atelier => s.fragments_spendable >= 1.0 && s.perks.iter().all(|p| !p.owned),
            Tip::Cosmos => input::cosmos_unlocked(s),
        }
    }
}

/// Écrans ouverts depuis le menu Cosmos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Event,
    Daily,
    Challenges,
    Lab,
    Cosmogony,
    StarMap,
    Relics,
    Mega,
}

/// Actions lourdes, confirmées en faisant glisser un curseur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confirm {
    Supernova,
    Crunch,
    /// Lancer ou abandonner le Défi n°.
    Challenge(usize),
    BigBang,
    NewGame,
    Demolish(usize),
    Awakening,
}

/// Confirmation ouverte et position du curseur (0 → 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Confirming {
    pub what: Confirm,
    pub slide: f64,
    pub opened_at: f64,
}

/// Zones touchables de la dernière image ; la dernière enregistrée est au-dessus.
#[derive(Debug, Default)]
pub struct HitMap {
    zones: Vec<(Rect, Hit)>,
}

impl HitMap {
    pub fn clear(&mut self) {
        self.zones.clear();
    }

    pub fn add(&mut self, rect: Rect, hit: Hit) {
        if !rect.is_empty() {
            self.zones.push((rect, hit));
        }
    }

    pub fn at(&self, col: u16, row: u16) -> Option<(Rect, Hit)> {
        self.zones
            .iter()
            .rev()
            .find(|(r, _)| col >= r.x && col < r.right() && row >= r.y && row < r.bottom())
            .copied()
    }

    pub fn zones(&self) -> &[(Rect, Hit)] {
        &self.zones
    }
}

/// Doigt posé : zone visée et suivi du maintien.
#[derive(Clone, Copy, Debug)]
pub struct Press {
    pub rect: Rect,
    pub hit: Hit,
    pub at: f64,
    /// Dernière répétition (achats en rafale).
    pub last_repeat: f64,
    /// Le maintien a déjà agi : le lever du doigt ne refait rien.
    pub consumed: bool,
    /// Dernière cellule sous le doigt, et le doigt a glissé.
    pub last: (u16, u16),
    pub moved: bool,
}

/// Nombre d'écrans défilants (un défilement mémorisé par écran).
pub const SCREENS: usize = 20;

/// État visuel du mode mobile.
#[derive(Debug, Default)]
pub struct MobileUi {
    pub tab: Tab,
    /// Sous-onglet choisi dans chaque onglet.
    pub sub: [usize; 5],
    /// Défilement (en lignes) de chaque écran.
    pub scroll: [u16; SCREENS],
    /// Défilement maximal de chaque écran, calculé au rendu.
    pub scroll_max: [u16; SCREENS],
    /// Écran affiché à la dernière image (cible du défilement).
    pub screen: usize,
    pub detail: Option<Detail>,
    pub hits: HitMap,
    pub press: Option<Press>,
    /// Courbe de production dépliée (onglet Usines).
    pub production_open: bool,
    pub requests: Vec<Request>,
    /// Instant du dernier changement d'onglet (fondu d'entrée).
    pub tab_changed_at: f64,
    /// Écran ouvert depuis le menu Cosmos.
    pub page: Option<Page>,
    pub confirm: Option<Confirming>,
    /// Carte stellaire : décalage de la vue (colonnes, lignes) et système choisi.
    pub map_pan: (i32, i32),
    pub map_centered: bool,
    pub map_sel: usize,
    /// Mégastructures : case choisie.
    pub mega_sel: usize,
    /// Rapport d'absence à montrer au retour dans l'app.
    pub absence: Vec<(char, String, crate::fx::Rgb)>,
    /// Vibration demandée depuis la dernière image (0, 1 léger, 2 fort).
    pub haptic: u8,
    /// Vibrations désactivées dans les options.
    pub no_vibrations: bool,
    /// Bulles de première partie déjà vues (un bit par `Tip`).
    pub tips_seen: u32,
}

impl MobileUi {
    /// Retient la vibration qu'appelle un événement du jeu.
    pub fn note_haptic(&mut self, ev: &domain::DomainEvent) {
        use domain::DomainEvent as E;
        let level = match ev {
            E::Harvested { .. }
            | E::Purchased { .. }
            | E::UpgradePurchased { .. }
            | E::PerkPurchased { .. } => 1,
            E::CometCaught { .. } | E::AchievementUnlocked { .. } | E::Supernova { .. } => 2,
            _ => 0,
        };
        self.haptic = self.haptic.max(level);
    }
}

/// Version tactile d'un message qui cite des touches : « [Z] pour décider »
/// devient « (Cosmos) pour décider », « [C] vite ! » devient « touche-la vite ! ».
pub fn touch_text(s: &str) -> String {
    if !s.contains('[') {
        return s.to_string();
    }
    let s = screens::touchify(s)
        .replace("Capture-la [C]", "Touche-la")
        .replace("[C]", "touche-la");
    let mut out = String::with_capacity(s.len());
    let mut rest = s.as_str();
    while let Some(start) = rest.find('[') {
        let Some(len) = rest[start..].find(']') else {
            break;
        };
        let key = &rest[start + 1..start + len];
        let label = match key {
            "E" => "(Améliorer)",
            "D" | "U" | "L" | "V" | "G" | "X" | "H" | "Z" | "W" | "Y" => "(Cosmos)",
            "P" => "(Fonderie)",
            "A" | "T" | "M" => "(Usines)",
            "I" | "R" => "(Plus)",
            k if k.chars().count() <= 8 && !k.contains(' ') => "",
            _ => {
                out.push_str(&rest[..start + len + 1]);
                rest = &rest[start + len + 1..];
                continue;
            }
        };
        out.push_str(&rest[..start]);
        out.push_str(label);
        rest = &rest[start + len + 1..];
    }
    out.push_str(rest);
    let mut tidy = out.split_whitespace().collect::<Vec<_>>().join(" ");
    for (a, b) in [(" )", ")"), ("( ", "("), (" ,", ","), (" .", ".")] {
        tidy = tidy.replace(a, b);
    }
    tidy.trim_end_matches([' ', ':']).to_string()
}

/// Délai d'un appui long (fiche détail) puis cadence des achats en rafale.
pub const LONG_PRESS: f64 = 0.45;
pub const REPEAT_START: f64 = 0.35;
pub const REPEAT_EVERY: f64 = 0.09;

#[cfg(test)]
mod tests {
    use super::touch_text;

    #[test]
    fn keys_become_touch_hints() {
        assert_eq!(
            touch_text("● Une comète approche ! [C] pour la capturer"),
            "● Une comète approche ! touche-la pour la capturer"
        );
        assert_eq!(
            touch_text("Orage : [Z] pour décider"),
            "Orage : (Cosmos) pour décider"
        );
        assert_eq!(
            touch_text("Achète-le à l'Atelier stellaire [E] avec des fragments"),
            "Achète-le à l'Atelier stellaire (Améliorer) avec des fragments"
        );
        assert_eq!(
            touch_text("La Carte stellaire s'ouvre [X]"),
            "La Carte stellaire s'ouvre (Cosmos)"
        );
        assert_eq!(touch_text("rien à changer"), "rien à changer");
    }
}
