//! Mode mobile : rendu de chaque onglet en portrait et hit-test.
//!
//! `cargo test -p presentation --test mobile -- --nocapture` affiche les écrans.

use std::cell::RefCell;
use std::rc::Rc;

use application::{
    Clock, GameService, LoadOutcome, RandomSource, RepositoryError, SaveData, SaveRepository,
};
use presentation::mobile::{Confirm, Hit, Page, Tab};
use presentation::App;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

#[derive(Clone, Default)]
struct InMemoryRepository(Rc<RefCell<Option<SaveData>>>);

impl SaveRepository for InMemoryRepository {
    fn load(&self) -> Result<LoadOutcome, RepositoryError> {
        Ok(match self.0.borrow().clone() {
            Some(d) => LoadOutcome::Loaded(Box::new(d)),
            None => LoadOutcome::NotFound,
        })
    }
    fn save(&self, data: &SaveData) -> Result<(), RepositoryError> {
        *self.0.borrow_mut() = Some(data.clone());
        Ok(())
    }
    fn location(&self) -> String {
        "mémoire".into()
    }
}

struct FakeClock;
impl Clock for FakeClock {
    fn now_unix_secs(&self) -> u64 {
        1_700_000_000
    }
}

struct FakeRng(f64);
impl RandomSource for FakeRng {
    fn next_f64(&mut self) -> f64 {
        self.0 = (self.0 + 0.37) % 1.0;
        self.0
    }
}

fn advanced_save() -> SaveData {
    SaveData {
        version: application::SAVE_VERSION,
        stardust: 123_456_789.0,
        total_run: 4.2e8,
        total_all: 9.9e9,
        owned: vec![42, 30, 26, 12, 3, 0, 0, 0, 0],
        upgrades: vec!["gen0_t0".into()],
        achievements: vec!["first".into(), "m1".into()],
        fragments: 12.0,
        supernovas: 2,
        clicks: 321,
        play_time: 4000.0,
        last_saved: 1_700_000_000,
        ..SaveData::default()
    }
}

fn mobile_app(save: Option<SaveData>) -> App {
    let repo = InMemoryRepository::default();
    *repo.0.borrow_mut() = save;
    let mut service = GameService::new(Box::new(repo), Box::new(FakeClock), Box::new(FakeRng(0.1)));
    let startup = service.load().expect("chargement");
    let mut app = App::new(service, startup);
    app.set_mobile(true);
    app.set_time(100.0);
    app.enter_game_now();
    app.update(0.033);
    app
}

fn text(buf: &Buffer) -> String {
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

fn render(app: &mut App, w: u16, h: u16) -> String {
    let area = Rect::new(0, 0, w, h);
    let mut buf = Buffer::empty(area);
    app.render_buffer(&mut buf, area);
    text(&buf)
}

/// Avance le temps et rend deux images (mise en page puis état stable).
fn settle(app: &mut App, w: u16, h: u16) -> String {
    let t = app_time(app) + 1.0;
    app.set_time(t);
    app.update(0.033);
    render(app, w, h);
    render(app, w, h)
}

fn app_time(app: &App) -> f64 {
    app.time()
}

fn find(app: &App, pred: impl Fn(&Hit) -> bool) -> Option<Rect> {
    app.mobile_hits()
        .into_iter()
        .find(|(_, h)| pred(h))
        .map(|(r, _)| r)
}

fn tap(app: &mut App, r: Rect) {
    let (x, y) = (r.x + r.width / 2, r.y + r.height / 2);
    app.touch_down(x, y);
    app.touch_up(x, y);
}

#[test]
fn every_tab_renders_in_portrait() {
    for (w, h) in [(44, 43), (40, 70), (48, 90)] {
        let mut app = mobile_app(Some(advanced_save()));
        for tab in Tab::ALL {
            app.mobile_select_tab(tab);
            let screen = settle(&mut app, w, h);
            if w == 44 {
                println!("=== {tab:?} {w}×{h} ===\n{screen}");
            }
            assert!(screen.contains("FONDERIE STELLAIRE"), "{tab:?}");
            assert!(
                !app.mobile_hits().is_empty(),
                "{tab:?} {w}×{h} : aucune zone touchable"
            );
            // Toutes les zones tiennent dans l'écran.
            for (r, hit) in app.mobile_hits() {
                assert!(r.right() <= w && r.bottom() <= h, "{hit:?} {r:?}");
            }
        }
    }
}

#[test]
fn sub_tabs_render() {
    let mut app = mobile_app(Some(advanced_save()));
    app.mobile_select_tab(Tab::Upgrade);
    settle(&mut app, 44, 43);
    let subs: Vec<Rect> = app
        .mobile_hits()
        .into_iter()
        .filter(|(_, h)| matches!(h, Hit::SubTab(_)))
        .map(|(r, _)| r)
        .collect();
    assert!(subs.len() >= 2, "Améliorations et Atelier");
    tap(&mut app, subs[1]);
    let screen = settle(&mut app, 44, 43);
    println!("{screen}");
    assert!(screen.contains("libres"), "bandeau des fragments");

    app.mobile_select_tab(Tab::More);
    for i in 0..5 {
        let r = find(&app, |h| *h == Hit::SubTab(i)).expect("sous-onglet");
        tap(&mut app, r);
        let screen = settle(&mut app, 44, 43);
        println!("{screen}");
        match i {
            1 => assert!(screen.contains("PROGRESSION")),
            2 => assert!(screen.contains("★ 2/55") || screen.contains("2/55")),
            4 => assert!(screen.contains("Général") && screen.contains("Exporter")),
            _ => {}
        }
    }
}

#[test]
fn touching_the_sky_harvests() {
    let mut app = mobile_app(Some(advanced_save()));
    settle(&mut app, 44, 43);
    let before = app.snapshot().clicks;
    let sky = find(&app, |h| *h == Hit::Harvest).expect("ciel");
    tap(&mut app, sky);
    app.update(0.033);
    assert_eq!(app.snapshot().clicks, before + 1);
}

#[test]
fn buy_buttons_buy_and_cards_open_details() {
    let mut app = mobile_app(Some(advanced_save()));
    app.mobile_select_tab(Tab::Factories);
    settle(&mut app, 44, 43);
    let owned = app.snapshot().owned[0];
    let buy = find(
        &app,
        |h| matches!(h, Hit::BuyGenerator(g) if g.index() == 0),
    )
    .expect("bouton");
    tap(&mut app, buy);
    app.update(0.033);
    assert!(app.snapshot().owned[0] > owned, "achat au toucher");

    let card = find(&app, |h| matches!(h, Hit::Details(_))).expect("carte");
    tap(&mut app, card);
    let screen = settle(&mut app, 44, 43);
    assert!(screen.contains("toucher pour fermer"), "{screen}");
    assert!(find(&app, |h| *h == Hit::CloseDetail).is_some());
    app.mobile_back();
    settle(&mut app, 44, 43);
    assert!(find(&app, |h| *h == Hit::CloseDetail).is_none());
}

#[test]
fn scrolling_moves_the_list() {
    let mut app = mobile_app(Some(advanced_save()));
    app.mobile_select_tab(Tab::Factories);
    let a = settle(&mut app, 44, 24);
    app.scroll_by(4);
    let b = settle(&mut app, 44, 24);
    assert_ne!(a, b, "la liste a défilé");
    app.scroll_by(-100);
    let c = settle(&mut app, 44, 24);
    assert_eq!(a.lines().nth(20), c.lines().nth(20));
}

#[test]
fn back_goes_home_then_quits() {
    let mut app = mobile_app(Some(advanced_save()));
    app.mobile_select_tab(Tab::Factories);
    app.mobile_back();
    assert!(app
        .mobile_tabs()
        .iter()
        .any(|t| t.tab == Tab::Foundry && t.active));
    assert!(!app.should_quit());
    app.mobile_back();
    assert!(app.should_quit());
}

#[test]
fn supernova_is_confirmed_by_sliding() {
    let mut save = advanced_save();
    save.total_run = 1e15;
    let mut app = mobile_app(Some(save));
    settle(&mut app, 44, 43);
    let nova = find(&app, |h| *h == Hit::Ask(Confirm::Supernova)).expect("bouton Supernova");
    tap(&mut app, nova);
    // La fiche monte du bas, puis le curseur apparaît.
    let screen = settle(&mut app, 44, 43);
    println!("{screen}");
    assert!(screen.contains("SUPERNOVA"));
    let slider = find(&app, |h| *h == Hit::Slider).expect("curseur");
    assert!(find(&app, |h| *h == Hit::CancelConfirm).is_some());

    // Un glissement incomplet ne fait rien.
    let y = slider.y + 1;
    app.touch_down(slider.x + 2, y);
    assert!(app.touch_move(slider.x + slider.width / 2, y));
    app.touch_up(slider.x + slider.width / 2, y);
    assert!(!app.in_sequence());

    // Jusqu'au bout : la Supernova part.
    app.touch_down(slider.x + 2, y);
    assert!(app.touch_move(slider.right() - 1, y));
    app.touch_up(slider.right() - 1, y);
    assert!(app.in_sequence(), "séquence Supernova lancée");
    assert!(app.mobile_wants_dense());
}

#[test]
fn every_cosmos_page_renders() {
    for (w, h) in [(44, 43), (40, 70), (48, 90)] {
        let mut app = mobile_app(Some(advanced_save()));
        for page in [
            Page::Event,
            Page::Daily,
            Page::Challenges,
            Page::Lab,
            Page::Cosmogony,
            Page::StarMap,
            Page::Relics,
            Page::Mega,
        ] {
            app.mobile_open(page);
            let screen = settle(&mut app, w, h);
            if w == 44 {
                println!("=== {page:?} ===\n{screen}");
            }
            assert!(screen.contains("‹ Cosmos"), "{page:?}");
            for (r, hit) in app.mobile_hits() {
                assert!(r.right() <= w && r.bottom() <= h, "{page:?} {hit:?} {r:?}");
            }
            app.mobile_back();
            settle(&mut app, w, h);
            assert!(
                find(&app, |h| *h == Hit::ClosePage).is_none(),
                "retour ferme la page {page:?}"
            );
        }
    }
}

#[test]
fn challenges_launch_after_confirmation() {
    let mut app = mobile_app(Some(advanced_save()));
    app.mobile_open(Page::Challenges);
    settle(&mut app, 44, 43);
    let Some(ask) = find(&app, |h| matches!(h, Hit::Ask(Confirm::Challenge(_)))) else {
        panic!("aucun Défi lançable");
    };
    tap(&mut app, ask);
    settle(&mut app, 44, 43);
    let slider = find(&app, |h| *h == Hit::Slider).expect("curseur");
    let y = slider.y + 1;
    app.touch_down(slider.x + 2, y);
    app.touch_move(slider.right() - 1, y);
    app.touch_up(slider.right() - 1, y);
    app.update(0.033);
    assert!(app.snapshot().active_challenge.is_some(), "Défi lancé");
}

#[test]
fn mega_and_map_pages_take_touches() {
    let mut app = mobile_app(Some(advanced_save()));
    app.mobile_open(Page::Mega);
    settle(&mut app, 44, 43);
    let cells: Vec<Rect> = app
        .mobile_hits()
        .into_iter()
        .filter(|(_, h)| matches!(h, Hit::MegaCell(_)))
        .map(|(r, _)| r)
        .collect();
    assert!(cells.len() >= 5 * 5, "grille pivotée : 5 colonnes");
    assert!(
        find(&app, |h| matches!(h, Hit::Build(_))).is_some(),
        "palette"
    );

    app.mobile_open(Page::StarMap);
    settle(&mut app, 44, 43);
    let map = find(&app, |h| *h == Hit::Map).expect("vue de la carte");
    let before = settle(&mut app, 44, 43);
    app.touch_down(map.x + 10, map.y + 5);
    assert!(
        app.touch_move(map.x + 2, map.y + 5),
        "la carte suit le doigt"
    );
    app.touch_up(map.x + 2, map.y + 5);
    let after = settle(&mut app, 44, 43);
    assert_ne!(before, after, "la vue a glissé");
}

#[test]
fn cosmos_tab_unlocks_with_the_first_supernova() {
    let mut fresh = SaveData {
        version: application::SAVE_VERSION,
        last_saved: 1_700_000_000,
        ..SaveData::default()
    };
    fresh.stardust = 10.0;
    let app = mobile_app(Some(fresh));
    let cosmos = app.mobile_tabs().into_iter().find(|t| t.tab == Tab::Cosmos);
    assert!(!cosmos.unwrap().unlocked);
    let app = mobile_app(Some(advanced_save()));
    let cosmos = app.mobile_tabs().into_iter().find(|t| t.tab == Tab::Cosmos);
    assert!(cosmos.unwrap().unlocked);
}

#[test]
fn absence_report_card_after_resume() {
    let mut save = advanced_save();
    save.last_saved = 1_700_000_000 - 3 * 3600;
    let mut app = mobile_app(Some(save));
    app.reload_save().expect("reprise");
    let screen = settle(&mut app, 44, 43);
    println!("{screen}");
    assert!(screen.contains("Pendant ton absence"), "{screen}");
    let ok = find(&app, |h| *h == Hit::CloseAbsence).expect("Récupérer");
    tap(&mut app, ok);
    settle(&mut app, 44, 43);
    assert!(
        find(&app, |h| *h == Hit::CloseAbsence).is_none(),
        "carte fermée"
    );
}

#[test]
fn first_game_tips_point_to_the_right_tab_once() {
    let fresh = SaveData {
        version: application::SAVE_VERSION,
        stardust: 20.0,
        last_saved: 1_700_000_000,
        ..SaveData::default()
    };
    let mut app = mobile_app(Some(fresh));
    let screen = settle(&mut app, 44, 43);
    assert!(screen.contains("première Sonde"), "{screen}");
    assert!(screen.contains('▼'));
    // Aller dans Usines referme la bulle pour toujours.
    app.mobile_select_tab(Tab::Factories);
    let screen = settle(&mut app, 44, 43);
    assert!(
        !screen.contains("première Sonde"),
        "pas de bulle hors de Fonderie"
    );
    app.mobile_select_tab(Tab::Foundry);
    let screen = settle(&mut app, 44, 43);
    assert!(!screen.contains("première Sonde"));
    assert_ne!(app.tips_seen(), 0);
}

#[test]
fn harvest_and_purchase_ask_for_a_vibration() {
    let mut app = mobile_app(Some(advanced_save()));
    settle(&mut app, 44, 43);
    assert_eq!(app.take_haptic(), 0);
    let sky = find(&app, |h| *h == Hit::Harvest).expect("ciel");
    tap(&mut app, sky);
    assert_eq!(app.take_haptic(), 1, "récolte : vibration légère");
    assert_eq!(app.take_haptic(), 0, "une seule fois");
    app.set_vibrations(false);
    tap(&mut app, sky);
    assert_eq!(app.take_haptic(), 0, "désactivées dans les options");
}

/// Relais factice : appareil lié, classement de 3 joueurs (le joueur 2e).
struct Linked;

impl application::SyncBackend for Linked {
    fn link_code(&self) -> Option<String> {
        Some("ABCDE-FGHJK-MNPQR-STVWX-YZ012".into())
    }
    fn create_link(&mut self) -> Result<String, application::SyncError> {
        Ok("ABCDE-FGHJK-MNPQR-STVWX-YZ012".into())
    }
    fn join_link(&mut self, code: &str) -> Result<String, application::SyncError> {
        Ok(code.into())
    }
    fn unlink(&mut self) -> Result<(), application::SyncError> {
        Ok(())
    }
    fn synced_rev(&self) -> u64 {
        0
    }
    fn set_synced_rev(&mut self, _: u64) -> Result<(), application::SyncError> {
        Ok(())
    }
    fn pull(&self) -> Result<Option<application::RemoteSave>, application::SyncError> {
        Ok(None)
    }
    fn push(
        &self,
        _: &SaveData,
        _: u64,
    ) -> Result<application::PushOutcome, application::SyncError> {
        Ok(application::PushOutcome::Stored { rev: 1 })
    }
}

struct Board3;

impl application::LeaderboardBackend for Board3 {
    fn linked(&self) -> bool {
        true
    }
    fn pseudo(&self) -> Option<String> {
        Some("Nova".into())
    }
    fn refresh_pseudo(&mut self) -> Result<Option<String>, application::LbError> {
        Ok(Some("Nova".into()))
    }
    fn claim(&mut self, p: &str) -> Result<String, application::LbError> {
        Ok(p.into())
    }
    fn withdraw(&mut self) -> Result<(), application::LbError> {
        Ok(())
    }
    fn submit(&self, _: &application::ScoreCard) -> Result<(), application::LbError> {
        Ok(())
    }
    fn standings(
        &self,
        board: application::Board,
        day: u64,
    ) -> Result<application::Standings, application::LbError> {
        let e = |rank: u32, pseudo: &str| application::Entry {
            rank,
            pseudo: pseudo.into(),
            me: pseudo == "Nova",
            act: 4 - rank as u8,
            progress: 0.5,
            score: Some(1e9 / rank as f64),
            ..application::Entry::default()
        };
        Ok(application::Standings {
            board,
            day,
            total: 3,
            entries: vec![e(1, "Orion"), e(2, "Nova"), e(3, "Véga")],
            me: None,
        })
    }
    fn site_url(&self) -> String {
        "https://relais/".into()
    }
}

#[test]
fn leaderboard_tab_shows_the_board_and_asks_for_a_pseudo() {
    let mut app = mobile_app(Some(advanced_save()));
    app.set_sync(application::SyncService::spawn(Box::new(Linked)));
    app.set_leaderboard(application::LeaderboardService::spawn(Box::new(Board3)));
    app.mobile_select_tab(Tab::More);
    settle(&mut app, 44, 43);
    let r = find(&app, |h| *h == Hit::SubTab(4)).expect("sous-onglet Sauver");
    tap(&mut app, r);
    // L'onglet s'affiche, le classement est demandé à l'image suivante, la
    // réponse arrive du thread du classement.
    settle(&mut app, 44, 43);
    settle(&mut app, 44, 43);
    std::thread::sleep(std::time::Duration::from_millis(100));
    let screen = settle(&mut app, 44, 43);
    println!("{screen}");
    assert!(
        screen.contains("Orion") && screen.contains("Véga"),
        "{screen}"
    );
    assert!(screen.contains("Acte III"), "{screen}");
    assert!(screen.contains("Changer de pseudo"), "{screen}");

    let daily =
        find(&app, |h| *h == Hit::Leaderboard(application::Board::Daily)).expect("onglet du jour");
    tap(&mut app, daily);
    let screen = settle(&mut app, 44, 43);
    assert!(screen.contains("✦"), "scores du jour\n{screen}");

    let pseudo = find(&app, |h| {
        *h == Hit::Request(presentation::mobile::Request::Pseudo)
    })
    .expect("bouton pseudo");
    tap(&mut app, pseudo);
    assert_eq!(
        app.take_request(),
        Some(presentation::mobile::Request::Pseudo)
    );
    let site = find(&app, |h| {
        *h == Hit::Request(presentation::mobile::Request::OpenSite)
    })
    .expect("bouton site");
    tap(&mut app, site);
    assert_eq!(
        app.take_request(),
        Some(presentation::mobile::Request::OpenSite)
    );
    for (r, hit) in app.mobile_hits() {
        assert!(r.right() <= 44 && r.bottom() <= 43, "{hit:?} {r:?}");
    }
}

/// Relais partagé en mémoire entre deux appareils (révisions comme le vrai).
#[derive(Default)]
struct Slot {
    rev: u64,
    save: Option<SaveData>,
}

struct SharedRelay {
    slot: std::sync::Arc<std::sync::Mutex<Slot>>,
    rev: u64,
}

impl application::SyncBackend for SharedRelay {
    fn link_code(&self) -> Option<String> {
        Some("ABCDE-FGHJK-MNPQR-STVWX-YZ012".into())
    }
    fn create_link(&mut self) -> Result<String, application::SyncError> {
        Ok("ABCDE-FGHJK-MNPQR-STVWX-YZ012".into())
    }
    fn join_link(&mut self, code: &str) -> Result<String, application::SyncError> {
        Ok(code.into())
    }
    fn unlink(&mut self) -> Result<(), application::SyncError> {
        Ok(())
    }
    fn synced_rev(&self) -> u64 {
        self.rev
    }
    fn set_synced_rev(&mut self, rev: u64) -> Result<(), application::SyncError> {
        self.rev = rev;
        Ok(())
    }
    fn pull(&self) -> Result<Option<application::RemoteSave>, application::SyncError> {
        let s = self.slot.lock().unwrap();
        Ok(s.save
            .clone()
            .map(|data| application::RemoteSave { rev: s.rev, data }))
    }
    fn push(
        &self,
        data: &SaveData,
        base: u64,
    ) -> Result<application::PushOutcome, application::SyncError> {
        let mut s = self.slot.lock().unwrap();
        if base != s.rev {
            return Ok(application::PushOutcome::Conflict);
        }
        s.rev += 1;
        s.save = Some(data.clone());
        Ok(application::PushOutcome::Stored { rev: s.rev })
    }
}

/// Laisse le thread de synchro répondre, puis avance d'une image.
fn sync_settle(app: &mut App) {
    for _ in 0..3 {
        std::thread::sleep(std::time::Duration::from_millis(60));
        settle(app, 48, 90);
    }
}

#[test]
fn a_new_game_on_one_device_restarts_every_linked_device() {
    let slot = std::sync::Arc::new(std::sync::Mutex::new(Slot::default()));
    let relay = |slot: &std::sync::Arc<std::sync::Mutex<Slot>>| SharedRelay {
        slot: slot.clone(),
        rev: 0,
    };
    // Le téléphone est le plus joué ; le PC reprend sa partie.
    let mut phone_save = advanced_save();
    phone_save.play_time = 9_000.0;
    let mut phone = mobile_app(Some(phone_save));
    phone.set_sync(application::SyncService::spawn(Box::new(relay(&slot))));
    sync_settle(&mut phone);
    let mut pc = mobile_app(Some(advanced_save()));
    pc.set_sync(application::SyncService::spawn(Box::new(relay(&slot))));
    sync_settle(&mut pc);
    assert!(
        pc.snapshot().play_time >= 9_000.0,
        "le PC a repris la partie"
    );

    // Nouvelle partie sur le PC : Plus › Options › Recommencer, glisser.
    pc.mobile_select_tab(Tab::More);
    settle(&mut pc, 48, 90);
    let options = find(&pc, |h| *h == Hit::SubTab(3)).expect("sous-onglet Options");
    tap(&mut pc, options);
    settle(&mut pc, 48, 90);
    let ask = find(&pc, |h| *h == Hit::Ask(Confirm::NewGame)).expect("Recommencer");
    tap(&mut pc, ask);
    let screen = settle(&mut pc, 48, 90);
    assert!(screen.contains("appareils synchronisés"), "{screen}");
    let slider = find(&pc, |h| *h == Hit::Slider).expect("curseur");
    let y = slider.y + 1;
    pc.touch_down(slider.x + 2, y);
    pc.touch_move(slider.right() - 1, y);
    pc.touch_up(slider.right() - 1, y);
    for _ in 0..4 {
        settle(&mut pc, 48, 90);
    }
    sync_settle(&mut pc);
    assert_eq!(pc.snapshot().clicks, 0, "nouvelle partie sur le PC");
    assert!(pc.snapshot().started_at > 0);
    {
        let s = slot.lock().unwrap();
        let cloud = s.save.as_ref().expect("partie sur le relais");
        assert_eq!(
            cloud.clicks, 0,
            "l'ancienne partie est remplacée sur le relais"
        );
        assert_eq!(cloud.started_at, pc.snapshot().started_at);
    }

    // Le téléphone, bien plus joué, reprend la nouvelle partie à sa synchro.
    phone.sync_now();
    sync_settle(&mut phone);
    assert_eq!(phone.snapshot().clicks, 0, "le téléphone repart de zéro");
    assert_eq!(phone.snapshot().started_at, pc.snapshot().started_at);
    assert_eq!(phone.screen(), presentation::Screen::Title);
}

#[test]
fn title_screen_speaks_touch() {
    let repo = InMemoryRepository::default();
    *repo.0.borrow_mut() = Some(advanced_save());
    let mut service = GameService::new(Box::new(repo), Box::new(FakeClock), Box::new(FakeRng(0.1)));
    let startup = service.load().expect("chargement");
    let mut app = App::new(service, startup);
    app.set_mobile(true);
    app.set_time(100.0);
    app.update(0.033);
    let screen = render(&mut app, 44, 43);
    println!("{screen}");
    assert_eq!(app.screen(), presentation::Screen::Title);
    assert!(screen.contains("Touche l'écran"), "{screen}");
    assert!(
        !screen.contains("[Q]") && !screen.contains("une touche"),
        "{screen}"
    );
}

/// Version publiée plus récente, à installer à la main (APK).
struct NewerApk;

impl application::UpdateProvider for NewerApk {
    fn latest(&self) -> Result<application::ReleaseInfo, application::UpdateError> {
        Ok(application::ReleaseInfo {
            version: "v99.0.0".into(),
            page_url: "https://github.com/o/r/releases/tag/v99.0.0".into(),
            binary_url: Some("https://dl/apk".into()),
            checksum_url: None,
        })
    }
    fn install_blocker(&self, _: &application::ReleaseInfo) -> Option<String> {
        Some("APK".into())
    }
    fn install(&self, _: &application::ReleaseInfo) -> Result<(), application::UpdateError> {
        Err(application::UpdateError("manuel".into()))
    }
}

#[test]
fn a_new_apk_is_announced_and_opens_its_page() {
    let mut app = mobile_app(Some(advanced_save()));
    app.set_updates(application::UpdateService::spawn(
        Box::new(NewerApk),
        "0.18.0",
    ));
    std::thread::sleep(std::time::Duration::from_millis(100));
    settle(&mut app, 44, 43);
    assert_eq!(
        app.update_url().as_deref(),
        Some("https://github.com/o/r/releases/tag/v99.0.0")
    );
    app.mobile_select_tab(Tab::More);
    settle(&mut app, 44, 43);
    let save = find(&app, |h| *h == Hit::SubTab(4)).expect("sous-onglet Sauver");
    tap(&mut app, save);
    let screen = settle(&mut app, 44, 43);
    println!("{screen}");
    assert!(screen.contains("v99.0.0 disponible"), "{screen}");
    let link = find(&app, |h| {
        *h == Hit::Request(presentation::mobile::Request::OpenUpdate)
    })
    .expect("lien de la nouvelle version");
    tap(&mut app, link);
    assert_eq!(
        app.take_request(),
        Some(presentation::mobile::Request::OpenUpdate)
    );
}

#[test]
fn progress_tab_details_every_production_multiplier() {
    let mut save = advanced_save();
    save.crunches = 2;
    let mut app = mobile_app(Some(save));
    app.mobile_select_tab(Tab::More);
    settle(&mut app, 48, 90);
    let progress = find(&app, |h| *h == Hit::SubTab(1)).expect("sous-onglet Progrès");
    tap(&mut app, progress);
    let screen = settle(&mut app, 48, 90);
    println!("{screen}");
    assert!(screen.contains("PRODUCTION"), "{screen}");
    assert!(
        screen.contains("×3") && screen.contains("Matière noire (Big Crunch)"),
        "{screen}"
    );
}
