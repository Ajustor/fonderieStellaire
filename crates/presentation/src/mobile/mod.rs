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
}

/// Délai d'un appui long (fiche détail) puis cadence des achats en rafale.
pub const LONG_PRESS: f64 = 0.45;
pub const REPEAT_START: f64 = 0.35;
pub const REPEAT_EVERY: f64 = 0.09;
