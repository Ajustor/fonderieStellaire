//! Composition root Android : instancie les adapters d'infrastructure, les
//! injecte dans le `GameService` et expose à Kotlin (via UniFFI) un objet
//! `Game` qui avance le jeu et renvoie l'écran en grille de cellules.
//!
//! Kotlin n'appelle `Game` que depuis le thread principal ; le `Mutex` ne sert
//! qu'à satisfaire UniFFI, qui exige des objets partageables entre threads.

use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use application::GameService;
use infrastructure::{JsonSaveRepository, SystemClock, XorShift64};
use presentation::cells::{self, Buffer, Rect};
use presentation::input::Intent;
use presentation::mobile::{Request as MobileRequest, Tab as MobileTab};
use presentation::App;

uniffi::setup_scaffolding!();

/// Nom du fichier de sauvegarde dans le dossier privé de l'app.
const SAVE_FILE: &str = "save.json";
/// Taille de grille maximale acceptée (garde-fou contre une vue mal mesurée).
const MAX_CELLS: u32 = 400 * 400;

#[derive(Debug, uniffi::Error)]
#[uniffi(flat_error)]
pub enum GameError {
    Failed(String),
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameError::Failed(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for GameError {}

impl From<String> for GameError {
    fn from(msg: String) -> Self {
        GameError::Failed(msg)
    }
}

/// Actions venues de l'interface native (bouton retour, menus…).
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum Action {
    Back,
    Up,
    Down,
    NextPanel,
    PrevPanel,
    Buy,
    BuyAll,
    CycleMode,
    CatchComet,
    Supernova,
    Yes,
    No,
    ToggleDebug,
}

impl From<Action> for Intent {
    fn from(a: Action) -> Self {
        match a {
            Action::Back => Intent::Escape,
            Action::Up => Intent::Up,
            Action::Down => Intent::Down,
            Action::NextPanel => Intent::NextPanel,
            Action::PrevPanel => Intent::PrevPanel,
            Action::Buy => Intent::Buy,
            Action::BuyAll => Intent::BuyAll,
            Action::CycleMode => Intent::CycleMode,
            Action::CatchComet => Intent::CatchComet,
            Action::Supernova => Intent::Supernova,
            Action::Yes => Intent::Yes,
            Action::No => Intent::No,
            Action::ToggleDebug => Intent::ToggleDebug,
        }
    }
}

/// Onglets de la barre native.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum Tab {
    Foundry,
    Factories,
    Upgrade,
    Cosmos,
    More,
}

impl From<MobileTab> for Tab {
    fn from(t: MobileTab) -> Self {
        match t {
            MobileTab::Foundry => Tab::Foundry,
            MobileTab::Factories => Tab::Factories,
            MobileTab::Upgrade => Tab::Upgrade,
            MobileTab::Cosmos => Tab::Cosmos,
            MobileTab::More => Tab::More,
        }
    }
}

impl From<Tab> for MobileTab {
    fn from(t: Tab) -> Self {
        match t {
            Tab::Foundry => MobileTab::Foundry,
            Tab::Factories => MobileTab::Factories,
            Tab::Upgrade => MobileTab::Upgrade,
            Tab::Cosmos => MobileTab::Cosmos,
            Tab::More => MobileTab::More,
        }
    }
}

/// État d'un onglet : débloqué, actif, pastille.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Record)]
pub struct TabInfo {
    pub tab: Tab,
    pub unlocked: bool,
    pub active: bool,
    pub badge: bool,
}

/// Demande du jeu à la plateforme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum Request {
    ExportSave,
    ImportSave,
}

struct Inner {
    app: App,
    repo: JsonSaveRepository,
    buf: Buffer,
    prev: Option<Buffer>,
    out: Vec<u8>,
    last_frame: Option<Instant>,
}

/// SAFETY : `App` contient des ports `Box<dyn …>` sans borne `Send`. Kotlin
/// n'utilise `Game` que depuis le thread principal, et tout accès passe par
/// le `Mutex` : l'état n'est jamais touché par deux threads à la fois.
unsafe impl Send for Inner {}

#[derive(uniffi::Object)]
pub struct Game {
    inner: Mutex<Inner>,
}

#[uniffi::export]
impl Game {
    /// Charge la partie depuis `save_dir` (le `filesDir` de l'app) et applique
    /// les gains hors-ligne.
    #[uniffi::constructor]
    pub fn new(save_dir: String) -> Result<Arc<Self>, GameError> {
        let path = PathBuf::from(save_dir).join(SAVE_FILE);
        let repo = JsonSaveRepository::new(&path);
        let mut service = GameService::new(
            Box::new(repo.clone()),
            Box::new(SystemClock),
            Box::new(XorShift64::from_time()),
        );
        let startup = service.load().map_err(|e| e.to_string())?;
        let mut app = App::new(service, startup);
        app.set_mobile(true);
        Ok(Arc::new(Game {
            inner: Mutex::new(Inner {
                app,
                repo,
                buf: Buffer::empty(Rect::default()),
                prev: None,
                out: Vec::with_capacity(64 * 1024),
                last_frame: None,
            }),
        }))
    }

    /// Avance le jeu puis rend l'écran en `cols × rows` cellules. Renvoie les
    /// lignes modifiées depuis l'image précédente (format : `presentation::cells`).
    pub fn frame(&self, cols: u16, rows: u16) -> Vec<u8> {
        let mut g = self.lock();
        let now = Instant::now();
        let dt = g
            .last_frame
            .map_or(0.0, |t| now.duration_since(t).as_secs_f64());
        g.last_frame = Some(now);
        g.app.step(dt);

        let (cols, rows) = clamp_size(cols, rows);
        let area = Rect::new(0, 0, cols, rows);
        let Inner {
            app,
            buf,
            prev,
            out,
            ..
        } = &mut *g;
        if buf.area != area {
            *buf = Buffer::empty(area);
            *prev = None;
        } else {
            buf.reset();
        }
        app.render_timed(buf, area, dt, now);
        cells::encode(buf, prev.as_ref(), out);
        match prev {
            Some(p) if p.area == area => p.content.clone_from(&buf.content),
            _ => *prev = Some(buf.clone()),
        }
        out.clone()
    }

    /// Vrai quand la grille veut des cellules plus petites (≈ 88 colonnes) :
    /// lancement, séquences plein écran, écrans hérités du terminal.
    pub fn wants_dense(&self) -> bool {
        self.lock().app.mobile_wants_dense()
    }

    /// La barre d'onglets est visible (masquée pendant les séquences).
    pub fn bar_visible(&self) -> bool {
        self.lock().app.mobile_bar_visible()
    }

    pub fn tabs(&self) -> Vec<TabInfo> {
        self.lock()
            .app
            .mobile_tabs()
            .into_iter()
            .map(|t| TabInfo {
                tab: t.tab.into(),
                unlocked: t.unlocked,
                active: t.active,
                badge: t.badge,
            })
            .collect()
    }

    pub fn select_tab(&self, tab: Tab) {
        self.lock().app.mobile_select_tab(tab.into());
    }

    /// Images par seconde choisies dans les options (10 à 60).
    pub fn frame_rate(&self) -> u32 {
        self.lock().app.snapshot().frame_rate.clamp(10, 60)
    }

    /// Doigt posé sur la cellule (`col`, `row`).
    pub fn touch_down(&self, col: u16, row: u16) {
        self.lock().app.touch_down(col, row);
    }

    /// Doigt toujours posé (à chaque image) : récolte continue, achats en
    /// rafale, appui long.
    pub fn touch_hold(&self) {
        self.lock().app.touch_hold();
    }

    /// Doigt levé sans avoir glissé.
    pub fn touch_up(&self, col: u16, row: u16) {
        self.lock().app.touch_up(col, row);
    }

    /// Le geste est devenu un glissement : le toucher est annulé.
    pub fn touch_cancel(&self) {
        self.lock().app.touch_cancel();
    }

    /// Défilement vertical en lignes (> 0 : vers le bas de la liste).
    pub fn scroll_by(&self, rows: i32) {
        self.lock().app.scroll_by(rows);
    }

    /// Balayage horizontal : `dir` = 1 vers la droite (onglet suivant), -1 sinon.
    pub fn swipe(&self, dir: i32) {
        self.lock().app.swipe(dir.signum());
    }

    /// Bouton retour Android.
    pub fn back(&self) {
        self.lock().app.mobile_back();
    }

    /// Prochaine demande à traiter (sélecteur de fichiers…).
    pub fn take_request(&self) -> Option<Request> {
        self.lock().app.take_request().map(|r| match r {
            MobileRequest::ExportSave => Request::ExportSave,
            MobileRequest::ImportSave => Request::ImportSave,
        })
    }

    /// Message à afficher en toast (résultat d'un export…).
    pub fn notify(&self, title: String, body: String) {
        self.lock().app.notify(&title, &body);
    }

    pub fn action(&self, action: Action) {
        self.lock().app.on_intent(action.into());
    }

    /// Le joueur a demandé à quitter (retour depuis l'écran principal).
    pub fn should_quit(&self) -> bool {
        self.lock().app.should_quit()
    }

    /// Mise en arrière-plan : sauvegarde immédiate (Android peut tuer l'app ensuite).
    pub fn pause(&self) -> Result<(), GameError> {
        let mut g = self.lock();
        g.last_frame = None;
        g.app.save_now().map_err(GameError::from)
    }

    /// Retour au premier plan : gains hors-ligne depuis la dernière sauvegarde.
    pub fn resume(&self) -> Result<(), GameError> {
        let mut g = self.lock();
        g.last_frame = None;
        g.prev = None;
        g.app.reload_save().map_err(GameError::from)
    }

    /// Contenu de la sauvegarde, à partager avec la version PC.
    pub fn export_save(&self) -> Result<Vec<u8>, GameError> {
        let mut g = self.lock();
        g.app.save_now()?;
        g.repo.export_bytes().map_err(|e| e.to_string().into())
    }

    /// Reprend une sauvegarde exportée (l'actuelle passe en `.bak`).
    pub fn import_save(&self, bytes: Vec<u8>) -> Result<(), GameError> {
        let mut g = self.lock();
        g.repo
            .import_bytes(&bytes)
            .map_err(|e| GameError::Failed(e.to_string()))?;
        g.prev = None;
        g.app.reload_save().map_err(GameError::from)
    }
}

impl Game {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn clamp_size(cols: u16, rows: u16) -> (u16, u16) {
    let cols = cols.max(1);
    let rows = rows.max(1);
    if u32::from(cols) * u32::from(rows) > MAX_CELLS {
        (cols.min(400), rows.min(400))
    } else {
        (cols, rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dossier neuf (le dépôt crée les dossiers manquants à la sauvegarde).
    fn temp_dir(tag: &str) -> String {
        let nanos = Instant::now().elapsed().as_nanos();
        std::env::temp_dir()
            .join(format!(
                "fonderie-android-{tag}-{}-{nanos}",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn renders_a_portrait_grid_and_then_only_changes() {
        let game = Game::new(temp_dir("frame")).unwrap();
        let first = game.frame(44, 43);
        assert_eq!(&first[0..4], &[44, 0, 43, 0]);
        assert_eq!(
            u16::from_le_bytes([first[4], first[5]]),
            43,
            "première image complète"
        );
        let second = game.frame(44, 43);
        let sent = u16::from_le_bytes([second[4], second[5]]);
        assert!(sent <= 43);
        assert_eq!(
            second.len(),
            cells::HEADER_BYTES + sent as usize * (2 + 44 * cells::CELL_BYTES)
        );
        // Changement de taille : tout est renvoyé.
        let resized = game.frame(88, 86);
        assert_eq!(u16::from_le_bytes([resized[4], resized[5]]), 86);
    }

    #[test]
    fn pause_saves_and_export_import_round_trip() {
        let dir = temp_dir("save");
        let game = Game::new(dir.clone()).unwrap();
        game.frame(44, 43);
        game.touch_down(10, 10); // passe l'intro
        game.touch_up(10, 10);
        assert!(game.export_save().is_ok(), "export sauvegarde d'abord");
        game.pause().unwrap();
        let bytes = game.export_save().unwrap();
        game.import_save(bytes).unwrap();
        game.resume().unwrap();
        game.frame(44, 43);
        assert!(!game.should_quit());
    }
}
