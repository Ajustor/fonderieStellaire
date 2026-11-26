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
    for i in 0..4 {
        let r = find(&app, |h| *h == Hit::SubTab(i)).expect("sous-onglet");
        tap(&mut app, r);
        let screen = settle(&mut app, 44, 43);
        if i == 3 {
            assert!(screen.contains("Exporter"));
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
