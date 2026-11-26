//! Rendu de l'écran de jeu dans un `TestBackend` : snapshot texte, vérification
//! de l'alignement et mesure du temps de rendu.
//!
//! `cargo test -p presentation --test render -- --nocapture` affiche les écrans.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use application::{
    Clock, Command, GameService, LoadOutcome, RandomSource, RepositoryError, SaveData,
    SaveRepository,
};
use presentation::input::Intent;
use presentation::App;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

#[derive(Clone, Default)]
struct InMemoryRepository(Rc<RefCell<Option<SaveData>>>);

impl SaveRepository for InMemoryRepository {
    fn load(&self) -> Result<LoadOutcome, RepositoryError> {
        Ok(match self.0.borrow().clone() {
            Some(d) => LoadOutcome::Loaded(d),
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

/// Partie déjà avancée : plusieurs générateurs, améliorations disponibles.
fn advanced_save() -> SaveData {
    SaveData {
        version: 1,
        stardust: 123_456_789.0,
        total_run: 4.2e8,
        total_all: 9.9e9,
        owned: vec![42, 30, 26, 12, 3, 0, 0, 0, 0],
        upgrades: vec!["gen0_t0".into()],
        achievements: vec!["first".into(), "m1".into()],
        fragments: 12.0,
        clicks: 321,
        play_time: 4000.0,
        last_saved: 1_700_000_000,
        ..SaveData::default()
    }
}

fn app_with(save: Option<SaveData>) -> App {
    let repo = InMemoryRepository::default();
    *repo.0.borrow_mut() = save;
    let mut service = GameService::new(Box::new(repo), Box::new(FakeClock), Box::new(FakeRng(0.1)));
    let startup = service.load().expect("chargement");
    App::new(service, startup)
}

fn buffer_text(buf: &Buffer) -> String {
    let area = buf.area;
    let mut out = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

fn render(app: &mut App, w: u16, h: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(w, h)).expect("terminal");
    terminal.draw(|f| app.render(f)).expect("draw");
    buffer_text(terminal.backend().buffer())
}

fn game_app() -> App {
    let mut app = app_with(Some(advanced_save()));
    app.set_time(100.0);
    app.enter_game_now();
    app.update(0.033);
    // Une frame pour calculer la mise en page, une seconde pour l'état stable.
    let _ = render(&mut app, 100, 30);
    app
}

#[test]
fn game_screen_100x30() {
    let mut app = game_app();
    let text = render(&mut app, 100, 30);
    println!("{text}");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 30);
    for l in &lines {
        assert_eq!(l.chars().count(), 100, "largeur constante : {l}");
    }
    assert!(lines[0].starts_with('╭') && lines[0].contains("FONDERIE STELLAIRE"));
    assert!(text.contains("milliards") || text.contains("millions"));
    assert!(text.contains("Générateurs"));
    assert!(text.contains("Sonde"));
    assert!(text.contains("Station lunaire"));
    assert!(text.contains("Ciel"));
    assert!(text.contains("Améliorations"));
    assert!(text.contains("Production"));
    assert!(text.contains("Journal"));
    assert!(text.contains("Supernova : +2 fragments"));
    assert!(text.contains("◆ 12 (+60%)"));
    assert!(text.contains("[Espace] Récolter"));
    assert!(text.contains('▶'));
    // Bordures droites alignées sur la dernière colonne.
    assert!(lines[1].ends_with('│'));
    assert!(lines[28].ends_with('╯'));
}

#[test]
fn game_screen_80x24_is_readable() {
    let mut app = game_app();
    let text = render(&mut app, 80, 24);
    println!("{text}");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 24);
    assert!(text.contains("Générateurs"));
    assert!(text.contains("Sonde"));
    assert!(text.contains("Améliorations"));
    assert!(text.contains("[Espace] Récolter"));
}

#[test]
fn too_small_terminal_shows_message() {
    let mut app = game_app();
    let text = render(&mut app, 60, 20);
    assert!(text.contains("Agrandis ta fenêtre (80×24 minimum)"));
}

#[test]
fn new_game_shows_teaser_and_empty_upgrades() {
    let mut app = app_with(None);
    app.enter_game_now();
    app.update(0.033);
    let _ = render(&mut app, 100, 30);
    let text = render(&mut app, 100, 30);
    println!("{text}");
    assert!(text.contains("??? ─ encore"));
    assert!(text.contains("Rien pour l'instant"));
}

#[test]
fn title_screen_renders_logo_and_offline_gains() {
    let mut save = advanced_save();
    save.last_saved -= 2 * 3600 + 14 * 60;
    let mut app = app_with(Some(save));
    app.set_time(1.0);
    app.on_intent(Intent::Other); // saute l'intro
    assert_eq!(app.screen(), presentation::Screen::Title);
    app.set_time(5.0);
    let text = render(&mut app, 100, 30);
    println!("{text}");
    assert!(text.contains("███████╗"));
    assert!(text.contains("Pendant ton absence (2h 14m)"));
    assert!(text.contains("Appuie sur une touche pour commencer"));
}

#[test]
fn harvest_and_purchase_have_visual_feedback() {
    let mut app = app_with(None);
    app.set_time(50.0);
    app.enter_game_now();
    app.update(0.033);
    let _ = render(&mut app, 100, 30);
    for _ in 0..15 {
        app.on_intent(Intent::Harvest);
    }
    app.on_intent(Intent::Buy);
    assert_eq!(
        app.snapshot().generators[0].owned,
        0,
        "snapshot rafraîchi à l'update"
    );
    app.update(0.033);
    assert_eq!(app.snapshot().generators[0].owned, 1);
    let text = render(&mut app, 100, 30);
    assert!(text.contains("Achat") || text.contains("Sonde"));
    let _ = Command::Harvest;
}

#[test]
fn frame_render_time_120x40() {
    let mut app = game_app();
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("terminal");
    for _ in 0..5 {
        app.on_intent(Intent::Harvest);
    }
    let mut total = 0.0;
    let frames = 60;
    for i in 0..frames {
        app.set_time(100.0 + i as f64 / 30.0);
        app.update(1.0 / 30.0);
        let t0 = Instant::now();
        terminal.draw(|f| app.render(f)).expect("draw");
        total += t0.elapsed().as_secs_f64();
    }
    let avg_ms = total / frames as f64 * 1000.0;
    println!("rendu moyen 120×40 : {avg_ms:.2} ms");
    // Large marge en debug ; l'objectif < 5 ms s'entend en release (overlay F3).
    assert!(avg_ms < 60.0, "{avg_ms} ms");
}

#[test]
fn full_session_smoke_test_never_panics() {
    let mut app = game_app();
    let sizes = [
        (80u16, 24u16),
        (100, 30),
        (120, 40),
        (200, 60),
        (60, 20),
        (81, 25),
    ];
    let mut t = 100.0;
    let mut frame = 0usize;
    let mut step = |app: &mut App, t: &mut f64, frames: usize| {
        for _ in 0..frames {
            *t += 1.0 / 30.0;
            app.set_time(*t);
            app.update(1.0 / 30.0);
            let (w, h) = sizes[frame % sizes.len()];
            frame += 1;
            let _ = render(app, w, h);
        }
    };
    // Laisse passer la première comète (45 s) puis la capture.
    step(&mut app, &mut t, 30 * 47);
    assert!(app.snapshot().comet_progress.is_some(), "comète en vol");
    app.on_intent(Intent::CatchComet);
    step(&mut app, &mut t, 30);
    for i in [
        Intent::Harvest,
        Intent::Down,
        Intent::NextPanel,
        Intent::Buy,
        Intent::CycleMode,
        Intent::QuickBuy(2),
        Intent::ToggleEffects,
        Intent::Save,
        Intent::ToggleDebug,
    ] {
        app.on_intent(i);
        step(&mut app, &mut t, 3);
    }
    app.on_intent(Intent::ToggleEffects);
    // Popup puis séquence Supernova complète.
    app.on_intent(Intent::Supernova);
    step(&mut app, &mut t, 10);
    app.on_intent(Intent::Yes);
    step(&mut app, &mut t, 30 * 6);
    let snap = app.snapshot();
    assert!(snap.fragments > 12.0, "fragments gagnés");
    assert_eq!(snap.supernovas, 1);
    assert!(!app.should_quit());
    app.on_intent(Intent::Quit);
    assert!(app.should_quit());
}

fn space(kind: crossterm::event::KeyEventKind) -> crossterm::event::Event {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventState, KeyModifiers};
    Event::Key(KeyEvent {
        code: KeyCode::Char(' '),
        modifiers: KeyModifiers::NONE,
        kind,
        state: KeyEventState::NONE,
    })
}

#[test]
fn holding_space_requires_the_upgrade() {
    use crossterm::event::KeyEventKind::{Press, Release};
    // Sans l'amélioration : appuis distincts OK, touche maintenue refusée.
    let mut app = app_with(None);
    app.enter_game_now();
    app.on_terminal_event(&space(Press));
    app.on_terminal_event(&space(Release));
    app.on_terminal_event(&space(Press));
    app.update(0.033);
    assert_eq!(app.snapshot().clicks, 2, "deux appuis distincts");
    app.on_terminal_event(&space(Press)); // pas de Release : touche maintenue
    app.on_terminal_event(&space(Press));
    app.update(0.033);
    assert_eq!(
        app.snapshot().clicks,
        2,
        "maintien refusé sans l'amélioration"
    );
    assert!(!app.snapshot().hold_harvest);

    // Avec « Récolte continue » : le maintien récolte, à cadence plafonnée.
    let mut save = advanced_save();
    save.upgrades.push("hold_0".into());
    save.clicks = 0;
    let mut app = app_with(Some(save));
    app.enter_game_now();
    app.update(0.033);
    assert!(app.snapshot().hold_harvest);
    app.on_terminal_event(&space(Press));
    for _ in 0..10 {
        app.on_terminal_event(&space(Press)); // rafale de répétitions dans la même frame
    }
    app.update(0.033);
    assert_eq!(app.snapshot().clicks, 2, "1 appui + 1 répétition acceptée");
    for _ in 0..30 {
        app.on_terminal_event(&space(Press));
        app.update(0.05);
    }
    let clicks = app.snapshot().clicks;
    assert!((15..=18).contains(&clicks), "≈ 10/s sur 1.5 s : {clicks}");
}

#[test]
fn production_panel_has_axes_and_trend() {
    let mut app = game_app();
    let mut t = 100.0;
    for i in 0..90 {
        t += 1.0;
        app.set_time(t);
        if i % 15 == 0 {
            app.on_intent(Intent::QuickBuy(3)); // la prod grimpe par paliers
        }
        app.update(1.0);
    }
    for (w, h) in [(100u16, 30u16), (80, 24)] {
        let text = render(&mut app, w, h);
        let rows: Vec<&str> = text.lines().collect();
        let start = rows.iter().position(|l| l.contains("Production")).unwrap();
        let panel: Vec<String> = rows[start..h as usize - 1]
            .iter()
            .map(|l| l.chars().take(w as usize * 55 / 100).collect())
            .collect();
        println!("{}\n", panel.join("\n"));
        let joined = panel.join("\n");
        assert!(joined.contains("maintenant"));
        assert!(joined.contains("−1m 30s"));
        assert!(joined.contains('┤'));
        assert!(joined.contains("/s ▲") || joined.contains("/s = stable"));
    }
    // Les colonnes portent les couleurs des systèmes qui produisent.
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("terminal");
    terminal.draw(|f| app.render(f)).expect("draw");
    let buf = terminal.backend().buffer();
    let station = ratatui::style::Color::Rgb(220, 220, 140); // ○ Station lunaire
    let dyson = ratatui::style::Color::Rgb(255, 190, 90); // ● Sphère de Dyson
    let chart_colors: Vec<_> = (25..29)
        .flat_map(|y| (8..54).map(move |x| (x, y)))
        .filter(|&(x, y)| buf[(x, y)].symbol() == "█")
        .map(|(x, y)| buf[(x, y)].fg)
        .collect();
    assert!(chart_colors.contains(&station), "couleur des stations");
    assert!(chart_colors.contains(&dyson), "couleur des sphères");
}

#[test]
fn descriptions_and_info_screen() {
    let mut app = game_app();
    let text = render(&mut app, 100, 30);
    println!("{text}");
    // Encart du générateur sélectionné (la Sonde) : ce qu'elle fait + chiffres.
    assert!(text.contains("Petite sonde autonome"));
    assert!(text.contains("/s par unité"));
    assert!(text.contains("×2 à 50 (encore 8)"));
    // Encart de l'amélioration sélectionnée : effet + ambiance.
    assert!(text.contains("Maintenir [Espace] : 10 récoltes/s"));
    assert!(text.contains("Un gant motorisé"));
    // Amélioration suivante : effet chiffré d'après la production actuelle.
    app.on_intent(Intent::NextPanel);
    app.on_intent(Intent::Down);
    let text = render(&mut app, 100, 30);
    assert!(
        text.contains("+1% de la prod/s par clic → +30 par clic"),
        "{text}"
    );
    assert!(text.contains("Chaque geste ramasse"));

    app.on_intent(Intent::Info);
    app.set_time(101.0);
    let text = render(&mut app, 100, 30);
    println!("{text}");
    assert!(text.contains("SUCCÈS  2/12"));
    assert!(text.contains("★ Premier pas"));
    assert!(text.contains("○ Centurion"));
    app.on_intent(Intent::Down);
    app.on_intent(Intent::Escape);
    app.set_time(102.0);
    app.update(0.033);
    let text = render(&mut app, 80, 24);
    assert!(!text.contains("SUCCÈS"), "Infos refermé");
}

#[test]
fn update_status_is_shown() {
    use application::UpdateEvent;
    let mut app = app_with(None);
    app.set_time(5.0);
    let text = render(&mut app, 100, 30);
    assert!(
        text.contains("v0.1.0"),
        "version affichée sur l'écran titre"
    );
    app.react_update(UpdateEvent::Installed {
        version: "v0.2.0".into(),
    });
    let text = render(&mut app, 100, 30);
    assert!(
        text.contains("v0.2.0 installée, relance le jeu !"),
        "{text}"
    );
    app.enter_game_now();
    app.set_time(6.0); // le toast a fini de glisser
    app.update(0.033);
    let text = render(&mut app, 100, 30);
    assert!(text.contains("Mise à jour v0.2.0"), "toast en jeu : {text}");
}
