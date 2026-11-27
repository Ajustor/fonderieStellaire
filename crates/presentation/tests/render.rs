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

/// Partie déjà avancée : plusieurs générateurs, améliorations disponibles.
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
    assert!(text.contains("Supernova : +6 fragments"));
    assert!(text.contains("◆ 12 (+7%)"));
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
    assert!(text.contains("PENDANT TON ABSENCE (2h 14m)"), "{text}");
    assert!(text.contains("Ta fonderie a produit"));
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
        text.contains("+1% de la prod/s par clic → +") && text.contains(" par clic"),
        "{text}"
    );
    assert!(text.contains("Chaque geste ramasse"));

    app.on_intent(Intent::Info);
    app.set_time(101.0);
    let text = render(&mut app, 100, 30);
    println!("{text}");
    let total = domain::Catalog::standard().achievements.len();
    assert!(text.contains(&format!("SUCCÈS  2/{total}")), "{text}");
    assert!(text.contains("★ Premier pas"));
    // Succès du plus facile au plus difficile.
    let first = text.find("★ Premier pas").expect("premier succès");
    let comet = text.find("○ Chasseur de comètes").expect("deuxième succès");
    assert!(first < comet);
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
        text.contains(&format!("v{}", env!("CARGO_PKG_VERSION"))),
        "version affichée sur l'écran titre : {text}"
    );
    app.react_update(UpdateEvent::Installed {
        version: "v0.2.0".into(),
    });
    let text = render(&mut app, 100, 30);
    assert!(text.contains("v0.2.0 installée, redémarrage…"), "{text}");
    app.enter_game_now();
    app.set_time(6.0); // le toast a fini de glisser
    app.update(0.033);
    let text = render(&mut app, 100, 30);
    assert!(text.contains("Mise à jour v0.2.0"), "toast en jeu : {text}");
}

#[test]
fn v0_2_features_render() {
    // --- Atelier : achat avec les fragments de la sauvegarde avancée (12).
    let mut app = game_app();
    app.on_intent(Intent::Shop);
    app.set_time(101.0);
    let text = render(&mut app, 100, 30);
    println!("{text}");
    assert!(text.contains("ATELIER STELLAIRE"));
    assert!(text.contains("Fragments libres : 12 sur 12"));
    assert!(text.contains("Héritage"));
    app.on_intent(Intent::Buy);
    app.update(0.033);
    assert!(app.snapshot().perks[0].owned);
    assert_eq!(app.snapshot().fragments_spendable, 11.0);
    let text = render(&mut app, 100, 30);
    assert!(text.contains("● acquise"));
    app.on_intent(Intent::Escape);

    // --- Section Progression de l'écran Infos.
    app.set_time(102.0);
    app.update(0.033);
    app.on_intent(Intent::Info);
    app.set_time(103.0);
    let text = render(&mut app, 100, 30);
    assert!(text.contains("PROGRESSION"));
    assert!(text.contains("Prochain fragment : encore"));
    assert!(text.contains("Atelier 1/8"));
    app.on_intent(Intent::Escape);

    // --- Journal : la sauvegarde auto ne s'y écrit plus, et on peut remonter.
    app.set_time(104.0);
    app.update(0.033);
    let before = render(&mut app, 100, 30);
    for _ in 0..(31 * 30) {
        app.update(1.0 / 30.0);
    }
    let after = render(&mut app, 100, 30);
    assert!(!after.contains("Sauvegarde automatique"));
    assert!(before.contains("Journal"));
    app.on_intent(Intent::Save);
    for i in 0..6 {
        app.on_intent(Intent::Save);
        app.set_time(110.0 + i as f64);
    }
    app.update(0.033);
    app.on_intent(Intent::JournalUp);
    let text = render(&mut app, 100, 30);
    assert!(text.contains("↑ ") && text.contains("[PgDn]"), "{text}");

    // --- Nouvelle partie : la liste vide annonce la prochaine amélioration.
    let mut fresh = app_with(None);
    fresh.enter_game_now();
    fresh.update(0.033);
    let _ = render(&mut fresh, 100, 30);
    let text = render(&mut fresh, 100, 30);
    assert!(
        text.contains("Prochaine : Alliage renforcé · Sondes"),
        "{text}"
    );
    assert!(text.contains("0 / 10 Sondes"));
}

/// Longueur de la barre (█ et ░ consécutifs) sur une ligne de générateur.
fn bar_len(line: &str) -> usize {
    let start = line.find(['█', '░']).unwrap_or(0);
    line[start..]
        .chars()
        .take_while(|c| *c == '█' || *c == '░')
        .count()
}

#[test]
fn click_value_and_constant_bar_width() {
    let mut app = game_app(); // ~123 millions : le mot d'échelle est affiché
    for mode_presses in 0..4 {
        for (w, h) in [(100u16, 30u16), (80, 24)] {
            let text = render(&mut app, w, h);
            assert!(text.contains("millions"));
            assert!(
                text.contains("clic : +"),
                "valeur d'un clic visible :\n{text}"
            );
            let left = w as usize * 55 / 100;
            let widths: Vec<usize> = text
                .lines()
                .map(|l| l.chars().take(left).collect::<String>())
                .filter(|l| l.contains('░') || l.contains("█░") || l.contains("█ "))
                .filter(|l| l.trim_start_matches('│').starts_with("    "))
                .map(|l| bar_len(&l))
                .filter(|&n| n > 0)
                .collect();
            // L'encart de détail complet laisse au moins 3 générateurs visibles.
            assert!(widths.len() >= 3, "{widths:?}\n{text}");
            assert!(
                widths.windows(2).all(|p| p[0] == p[1]),
                "barres de tailles différentes {widths:?} (mode {mode_presses}) :\n{text}"
            );
        }
        app.on_intent(Intent::CycleMode); // ×1 → ×10 → ×25 → MAX
        app.update(0.033);
    }
}

#[test]
fn v0_4_constellations_and_new_generators() {
    // Sauvegarde avancée : 12 fragments, aucune Supernova encore.
    let mut app = game_app();
    let text = render(&mut app, 100, 30);
    // Le Trou blanc n'est pas encore là : il attend la première Supernova.
    assert!(!text.contains("Trou blanc"));

    // Atelier : la section Constellations est sous les améliorations uniques.
    app.on_intent(Intent::Shop);
    app.set_time(101.0);
    let count = 8 + 6;
    for _ in 0..8 {
        app.on_intent(Intent::Down); // → première Constellation (Le Forgeron)
    }
    let text = render(&mut app, 100, 30);
    println!("{text}");
    assert!(text.contains("CONSTELLATIONS"));
    assert!(text.contains("Le Forgeron"));
    assert!(text.contains("niv. 0 · 2 ◆"));
    assert!(text.contains("[Entrée] → niveau 1 : +20% de production"));
    app.on_intent(Intent::Buy);
    app.update(0.033);
    assert_eq!(app.snapshot().constellations[0].level, 1);
    assert_eq!(app.snapshot().fragments_spendable, 10.0);
    for _ in 0..count {
        app.on_intent(Intent::Down); // borné en bas de liste, sans paniquer
    }
    let _ = render(&mut app, 80, 24);
    app.on_intent(Intent::Escape);
    app.set_time(102.0);
    app.update(0.033);

    // Le ciel montre la constellation (une étoile allumée au moins).
    let text = render(&mut app, 100, 30);
    assert!(text.contains("Constellation") || text.contains('✦'));
}

#[test]
fn locked_generator_teaser_mentions_supernovas() {
    // Tous les générateurs de base possédés, aucune Supernova : le suivant est verrouillé.
    let mut save = advanced_save();
    save.owned = vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0];
    save.total_run = 1e15;
    let mut app = app_with(Some(save));
    app.enter_game_now();
    app.update(0.033);
    let mut text = String::new();
    for _ in 0..12 {
        app.on_intent(Intent::Down); // fait défiler jusqu'en bas de la liste
        text = render(&mut app, 100, 30);
    }
    assert!(text.contains("??? ─ apparaît après 1 Supernova"), "{text}");
}

#[test]
fn v0_5_challenges_and_rare_comets() {
    // 5 Supernovas faites : tous les Défis sont débloqués.
    let mut save = advanced_save();
    save.supernovas = 5;
    save.total_run = 5000.0; // pas de fragment à encaisser au lancement
    let mut app = app_with(Some(save));
    app.set_time(100.0);
    app.enter_game_now();
    app.update(0.033);
    let _ = render(&mut app, 100, 30);

    app.on_intent(Intent::Challenges);
    app.set_time(101.0);
    let text = render(&mut app, 100, 30);
    println!("{text}");
    assert!(text.contains("DÉFIS"));
    assert!(text.contains("Doigts liés"));
    assert!(text.contains("Course contre la montre"));
    assert!(text.contains("Récompense : Production ×1,25 pour toujours"));

    // Entrée une fois : armé (rien ne se passe), deux fois : lancé.
    app.on_intent(Intent::Buy);
    let text = render(&mut app, 100, 30);
    assert!(text.contains("Entrée de nouveau pour LANCER"), "{text}");
    assert!(app.snapshot().active_challenge.is_none());
    app.on_intent(Intent::Buy);
    app.set_time(103.0);
    app.update(0.033);
    let snap = app.snapshot();
    assert!(snap.active_challenge.is_some(), "Défi lancé");
    assert_eq!(snap.stardust, 0.0, "le cycle est relancé");

    // L'en-tête affiche le Défi en cours, les clics sont refusés.
    let text = render(&mut app, 100, 30);
    assert!(text.contains("◆ DÉFI Doigts liés"), "{text}");
    app.on_intent(Intent::Harvest);
    app.update(0.033);
    assert_eq!(
        app.snapshot().clicks,
        321,
        "aucun clic compté pendant « Doigts liés »"
    );

    // Comète dorée : bannière dorée (le tirage 0.1 de FakeRng ne la donne pas,
    // on vérifie donc le texte du widget directement).
    assert!(presentation::widgets::sky::GOLDEN_BANNER.contains("DORÉE"));
}

#[test]
fn v0_6_big_crunch_end_to_end() {
    // Trop tôt : un simple message.
    let mut app = game_app();
    app.on_intent(Intent::BigCrunch);
    app.set_time(103.0); // le message a fini de glisser et de s'écrire
    let text = render(&mut app, 100, 30);
    assert!(text.contains("Il faut 15 Supernovas"), "{text}");
    assert_eq!(app.snapshot().crunches, 0);

    // Au bout du contenu : 15 Supernovas.
    let mut save = advanced_save();
    save.supernovas = 15;
    save.perks = vec!["perk_start".into()];
    save.constellations = vec![("const_forge".into(), 4)];
    let mut app = app_with(Some(save));
    let mut t = 100.0;
    app.set_time(t);
    app.enter_game_now();
    app.update(0.033);
    let _ = render(&mut app, 100, 30);

    app.on_intent(Intent::BigCrunch);
    t += 1.0;
    app.set_time(t);
    let text = render(&mut app, 100, 30);
    println!("{text}");
    assert!(text.contains("BIG CRUNCH"));
    assert!(text.contains("Matière noire : 0 → 1"));
    assert!(text.contains("Production ×1 → ×2"));

    // Confirmation, puis toute la séquence (≈ 5 s) en rendant chaque image.
    app.on_intent(Intent::Yes);
    for i in 0..(6 * 30) {
        t += 1.0 / 30.0;
        app.set_time(t);
        app.update(1.0 / 30.0);
        let (w, h) = if i % 2 == 0 { (100, 30) } else { (80, 24) };
        let _ = render(&mut app, w, h);
    }
    let snap = app.snapshot();
    assert_eq!(snap.crunches, 1);
    assert_eq!(snap.supernovas, 0);
    assert_eq!(snap.fragments, 0.0);
    assert!(snap.perks.iter().all(|p| !p.owned));
    assert!(snap.constellations.iter().all(|c| c.level == 0));
    assert_eq!(snap.crunch_production, 2.0);
    let text = render(&mut app, 100, 30);
    assert!(
        text.contains("✶ 1 (×2)"),
        "matière noire dans l'en-tête :\n{text}"
    );
}

#[test]
fn v0_7_laboratory() {
    // Avant 10 Big Crunch : fermé.
    let mut app = game_app();
    app.on_intent(Intent::Lab);
    app.set_time(103.0);
    let text = render(&mut app, 100, 30);
    assert!(text.contains("S'ouvre au 10e Big Crunch (0/10)"), "{text}");

    // 10 Big Crunch : 10 matières noires à dépenser.
    let mut save = advanced_save();
    save.crunches = 10;
    let mut app = app_with(Some(save));
    let mut t = 100.0;
    app.set_time(t);
    app.enter_game_now();
    app.update(0.033);
    let _ = render(&mut app, 100, 30);
    app.on_intent(Intent::Lab);
    t += 1.0;
    app.set_time(t);
    let text = render(&mut app, 100, 30);
    println!("{text}");
    assert!(text.contains("LABORATOIRE DE MATIÈRE NOIRE"));
    assert!(text.contains("Matière noire libre : 10 sur 10"));
    assert!(text.contains("AUTOMATISMES"));
    assert!(text.contains("Filet à comètes"));
    assert!(text.contains("1 ✶ · 1h 00m"));
    assert!(
        text.contains("Densité II"),
        "toutes les recherches sont visibles"
    );

    // Lancer la première recherche (Filet à comètes, 1 h).
    app.on_intent(Intent::Buy);
    app.update(0.033);
    let snap = app.snapshot();
    assert!(snap.research.is_some());
    assert_eq!(snap.dark_matter_spendable, 9);
    let text = render(&mut app, 100, 30);
    assert!(text.contains("En cours : Filet à comètes"), "{text}");
    // Une seule à la fois.
    app.on_intent(Intent::Down);
    app.on_intent(Intent::Buy);
    app.update(0.033);
    assert_eq!(app.snapshot().dark_matter_spendable, 9);
    // L'en-tête rappelle la recherche en cours.
    app.on_intent(Intent::Escape);
    t += 1.0;
    app.set_time(t);
    app.update(0.033);
    let text = render(&mut app, 100, 30);
    assert!(text.contains("✶ Filet à comètes"), "{text}");
    // Une heure plus tard, c'est fait.
    app.update(3601.0);
    assert!(app.snapshot().lab[0].done || app.snapshot().lab.iter().any(|l| l.done));
}

#[test]
fn v0_8_portals_universes_and_key_ceremony() {
    // 10 Big Crunch et 15 Supernovas : le Big Crunch ouvre les portails.
    let mut save = advanced_save();
    save.crunches = 10;
    save.supernovas = 15;
    let mut app = app_with(Some(save));
    let mut t = 100.0;
    app.set_time(t);
    app.enter_game_now();
    app.update(0.033);
    let _ = render(&mut app, 120, 36);

    app.on_intent(Intent::BigCrunch);
    // Les cartes arrivent en vol : on laisse l'animation se poser.
    for _ in 0..30 {
        t += 1.0 / 30.0;
        app.set_time(t);
        let _ = render(&mut app, 120, 36);
    }
    app.on_intent(Intent::NextPanel); // → Gravité forte
    for _ in 0..15 {
        t += 1.0 / 30.0;
        app.set_time(t);
        let _ = render(&mut app, 120, 36);
    }
    let text = render(&mut app, 120, 36);
    println!("{text}");
    assert!(text.contains("CHOISIS TON UNIVERS"));
    assert!(text.contains("Gravité forte"));
    assert!(text.contains("Clé : 25 Supernovas"));
    assert!(
        text.contains('▓') || text.contains('▒'),
        "les vortex sont dessinés"
    );

    // Plongée, puis toute la séquence du Big Crunch, image par image.
    app.on_intent(Intent::Buy);
    for i in 0..(7 * 30) {
        t += 1.0 / 30.0;
        app.set_time(t);
        app.update(1.0 / 30.0);
        let (w, h) = if i % 3 == 0 { (80, 24) } else { (120, 36) };
        let _ = render(&mut app, w, h);
    }
    let snap = app.snapshot();
    assert_eq!(snap.crunches, 11);
    assert_eq!(snap.universe.index(), 1, "univers : Gravité forte");
    let text = render(&mut app, 120, 36);
    assert!(
        text.contains("● Gravité forte"),
        "badge de l'univers :\n{text}"
    );

    // La consultation (V) ne lance rien.
    app.on_intent(Intent::Universes);
    t += 1.0;
    app.set_time(t);
    let text = render(&mut app, 120, 36);
    assert!(text.contains("UNIVERS PARALLÈLES"));
    app.on_intent(Intent::Buy);
    app.update(0.033);
    assert_eq!(app.snapshot().crunches, 11);
    app.on_intent(Intent::Escape);
}

#[test]
fn installed_update_restarts_the_game_by_itself() {
    use application::UpdateEvent;
    let mut app = game_app();
    let mut t = 200.0;
    app.set_time(t);
    app.react_update(UpdateEvent::Installed {
        version: "v9.9.9".into(),
    });
    app.update(0.033);
    let text = render(&mut app, 100, 30);
    assert!(
        text.contains("Mise à jour v9.9.9 installée · redémarrage dans 5 s"),
        "{text}"
    );
    assert!(!app.should_quit());
    for _ in 0..(4 * 30) {
        t += 1.0 / 30.0;
        app.set_time(t);
        app.update(1.0 / 30.0);
    }
    assert!(!app.should_quit());
    for _ in 0..(2 * 30) {
        t += 1.0 / 30.0;
        app.set_time(t);
        app.update(1.0 / 30.0);
        let _ = render(&mut app, 100, 30);
    }
    assert!(app.should_quit() && app.should_restart());
}

#[test]
fn outdated_save_is_wiped_and_the_player_is_told() {
    let mut old = advanced_save();
    old.version = application::SAVE_RESET_BELOW - 1;
    let mut app = app_with(Some(old));
    app.set_time(8.0); // intro terminée, texte entièrement écrit
    app.update(0.033);
    for (w, h) in [(100, 30), (80, 24)] {
        let text = render(&mut app, w, h);
        println!("{text}");
        assert!(text.contains("NOUVEL ÉQUILIBRAGE"), "{text}");
        assert!(text.contains("ancienne partie"), "{text}");
        assert!(text.contains("Bonne nouvelle forge !"), "{text}");
    }
    assert_eq!(app.snapshot().stardust, 0.0, "nouvelle partie");
}

#[test]
fn v0_9_big_bang_forge_attractor_and_ending() {
    // 34 Big Crunch : la Cosmogonie propose un Big Bang à +3 particules.
    let mut save = advanced_save();
    save.crunches = 34;
    save.lab = vec!["lab_comet_net".into()];
    let mut app = app_with(Some(save));
    let mut t = 100.0;
    app.set_time(t);
    app.enter_game_now();
    app.update(0.033);
    let _ = render(&mut app, 120, 36);
    app.on_intent(Intent::Cosmos);
    t += 1.0;
    app.set_time(t);
    let text = render(&mut app, 120, 36);
    println!("{text}");
    assert!(text.contains("COSMOGONIE"), "{text}");
    assert!(text.contains("+3 ✷"));
    assert!(text.contains("Forge à neutrons"));
    assert!(text.contains("Éveil du Grand Attracteur"));

    // Entrée deux fois : le Big Bang, rendu image par image.
    app.on_intent(Intent::Buy);
    let text = render(&mut app, 120, 36);
    assert!(
        text.contains("Entrée de nouveau pour déclencher le BIG BANG"),
        "{text}"
    );
    app.on_intent(Intent::Buy);
    let mut seen_title = false;
    for i in 0..(9 * 30) {
        t += 1.0 / 30.0;
        app.set_time(t);
        app.update(1.0 / 30.0);
        let (w, h) = if i % 3 == 0 { (80, 24) } else { (120, 36) };
        let text = render(&mut app, w, h);
        seen_title |= text.contains("particules primordiales ✷");
    }
    assert!(seen_title, "l'annonce des particules est affichée");
    let snap = app.snapshot();
    assert_eq!((snap.big_bangs, snap.particles, snap.crunches), (1, 3, 0));
    assert_eq!(snap.particle_production, 4.0);
    let text = render(&mut app, 120, 36);
    assert!(
        text.contains("✷ 3 (×4)"),
        "particules dans l'en-tête :\n{text}"
    );

    // Forger la Forge à neutrons (1 particule), lancer la 1re étape (2 particules).
    app.on_intent(Intent::Cosmos);
    app.on_intent(Intent::Down);
    app.on_intent(Intent::Buy);
    app.update(0.033);
    assert!(app.snapshot().forges[0].forged);
    for _ in 0..3 {
        app.on_intent(Intent::Down);
    }
    app.on_intent(Intent::Buy);
    app.update(0.033);
    assert!(app.snapshot().attractor_build.is_some());
    assert_eq!(app.snapshot().particles_spendable, 0);
    app.on_intent(Intent::Escape);
    t += 1.0;
    app.set_time(t);
    app.update(0.033);
    let text = render(&mut app, 120, 36);
    assert!(
        text.contains("◇ Ancrage gravitationnel"),
        "chantier dans l'en-tête :\n{text}"
    );
    app.update(3.0 * 3600.0 + 1.0);
    assert_eq!(app.snapshot().attractor_stages, 1);

    // Dernière étape : l'éveil, puis le générique de fin.
    let mut save = advanced_save();
    save.big_bangs = 3;
    save.particles = 30;
    save.attractor = 6;
    let mut app = app_with(Some(save));
    let mut t = 100.0;
    app.set_time(t);
    app.enter_game_now();
    app.update(0.033);
    let _ = render(&mut app, 120, 36);
    app.on_intent(Intent::Cosmos);
    for _ in 0..10 {
        app.on_intent(Intent::Down);
    }
    app.on_intent(Intent::Buy);
    app.update(0.033);
    app.on_intent(Intent::Escape);
    app.update(36.0 * 3600.0 + 1.0);
    assert!(app.snapshot().attractor_awakened);
    let mut text = String::new();
    for _ in 0..(30 * 30) {
        t += 1.0 / 30.0;
        app.set_time(t);
        app.update(1.0 / 30.0);
        text = render(&mut app, 100, 30);
    }
    println!("{text}");
    assert!(text.contains("FIN DE L'ACTE II"), "{text}");
    assert!(text.contains("Conçu et forgé par Ajustor"));
    assert!(text.contains("[Entrée] continuer à jouer"));
    app.on_intent(Intent::Buy);
    t += 1.0;
    app.set_time(t);
    app.update(0.033);
    let text = render(&mut app, 100, 30);
    assert!(!text.contains("FIN DE L'ACTE II"), "retour au jeu");
}

#[test]
fn options_and_new_game_with_double_confirmation() {
    let mut app = game_app();
    let mut t = 200.0;
    app.set_time(t);
    app.on_intent(Intent::Options);
    t += 1.0;
    app.set_time(t);
    let text = render(&mut app, 100, 30);
    println!("{text}");
    assert!(text.contains("OPTIONS"), "{text}");
    assert!(text.contains("Images par seconde"));
    assert!(text.contains("Recommencer une nouvelle partie"));
    assert!(text.contains("mises à jour automatiques"));
    // Couleurs : normales → daltonien.
    assert!(text.contains("Couleurs"));
    app.on_intent(Intent::Buy);
    app.update(0.033);
    assert_eq!(app.snapshot().color_mode, 1);
    // Images par seconde : 30 → 60 → 20.
    app.on_intent(Intent::Down);
    app.on_intent(Intent::Buy);
    app.update(0.033);
    assert_eq!(app.snapshot().frame_rate, 60);
    app.on_intent(Intent::Buy);
    app.update(0.033);
    assert_eq!(app.snapshot().frame_rate, 20);
    // Synchronisation : indisponible sans relais, rien ne s'ouvre.
    assert!(text.contains("Synchronisation"));
    app.on_intent(Intent::Down);
    app.on_intent(Intent::Down);
    app.on_intent(Intent::Buy);
    assert!(!render(&mut app, 100, 30).contains("SYNCHRONISATION"));
    // Classement : la fenêtre s'ouvre, même sans relais, et explique pourquoi.
    app.on_intent(Intent::Down);
    app.on_intent(Intent::Buy);
    for _ in 0..15 {
        t += 1.0 / 30.0;
        app.set_time(t);
        app.update(1.0 / 30.0);
    }
    let text = render(&mut app, 100, 30);
    assert!(text.contains("CLASSEMENT"), "{text}");
    assert!(text.contains("indisponible"), "{text}");
    app.on_intent(Intent::NextPanel);
    assert!(render(&mut app, 100, 30).contains("Défi du jour"));
    app.on_intent(Intent::Escape);
    for _ in 0..15 {
        t += 1.0 / 30.0;
        app.set_time(t);
        app.update(1.0 / 30.0);
    }
    assert!(!render(&mut app, 100, 30).contains("CLASSEMENT"));
    // Recommencer : une première Entrée arme, changer de ligne désarme.
    app.on_intent(Intent::Down);
    app.on_intent(Intent::Buy);
    let text = render(&mut app, 100, 30);
    assert!(text.contains("TOUT effacer"), "{text}");
    app.on_intent(Intent::Up);
    app.on_intent(Intent::Down);
    app.on_intent(Intent::Buy);
    app.on_intent(Intent::Buy);
    // L'effacement, image par image, puis l'écran titre d'une partie neuve.
    for i in 0..(3 * 30) {
        t += 1.0 / 30.0;
        app.set_time(t);
        app.update(1.0 / 30.0);
        let (w, h) = if i % 2 == 0 { (100, 30) } else { (80, 24) };
        let _ = render(&mut app, w, h);
    }
    assert_eq!(app.screen(), presentation::app::Screen::Title);
    let snap = app.snapshot();
    assert_eq!(snap.stardust, 0.0);
    assert_eq!(snap.fragments, 0.0);
    assert_eq!(snap.frame_rate, 20, "les réglages sont gardés");
}

#[test]
fn v0_10_star_map_and_expeditions() {
    // Grand Attracteur éveillé : l'Acte III s'ouvre au premier instant de jeu.
    let mut save = advanced_save();
    save.big_bangs = 1;
    save.attractor = 7;
    let mut app = app_with(Some(save));
    let mut t = 100.0;
    app.set_time(t);
    app.enter_game_now();
    app.update(0.033);
    assert!(app.snapshot().starmap_unlocked);
    let _ = render(&mut app, 120, 36);
    app.on_intent(Intent::StarMap);
    for _ in 0..20 {
        t += 1.0 / 30.0;
        app.set_time(t);
        let _ = render(&mut app, 120, 36);
    }
    let text = render(&mut app, 120, 36);
    println!("{text}");
    assert!(text.contains("CARTE STELLAIRE"), "{text}");
    assert!(text.contains("Explorés : 0/24"));
    assert!(text.contains("[Entrée] envoyer une flotte"));
    assert!(text.contains("chance de succès"));

    // Envoi d'une flotte, puis son retour quelques heures plus tard.
    app.on_intent(Intent::Buy);
    app.update(0.033);
    let snap = app.snapshot();
    assert_eq!(snap.fleets.len(), 1);
    let target = snap.fleets[0].target;
    let secs = snap.fleets[0].total;
    let text = render(&mut app, 120, 36);
    assert!(text.contains('▸'), "la flotte est dessinée :\n{text}");
    app.update(secs + 1.0);
    let snap = app.snapshot();
    assert!(snap.fleets.is_empty());
    let explored = snap.systems[target].explored;
    t += 1.0;
    app.set_time(t);
    let text = render(&mut app, 120, 36);
    println!("{text}");
    if explored {
        assert!(text.contains("Explorés : 1/24"), "{text}");
    }
    // 80×24 : la carte reste lisible.
    let text = render(&mut app, 80, 24);
    assert!(text.contains("CARTE STELLAIRE"));
    app.on_intent(Intent::Escape);
    let text = render(&mut app, 120, 36);
    assert!(!text.contains("CARTE STELLAIRE"));
}

#[test]
fn v0_11_relics_found_equipped_and_card() {
    let mut save = advanced_save();
    save.big_bangs = 1;
    save.attractor = 7;
    save.relics = vec![("r_hammer".into(), 3), ("r_crucible".into(), 0)];
    let mut app = app_with(Some(save));
    let mut t = 100.0;
    app.set_time(t);
    app.enter_game_now();
    app.update(0.033);
    app.on_intent(Intent::Relics);
    t += 1.0;
    app.set_time(t);
    let text = render(&mut app, 120, 36);
    println!("{text}");
    assert!(text.contains("RELIQUES"), "{text}");
    assert!(text.contains("Marteau stellaire"));
    assert!(text.contains("légendaire"));
    assert!(text.contains("+80% de production"));
    assert!(text.contains("[Entrée] équiper"));
    app.on_intent(Intent::Buy);
    app.update(0.033);
    assert!(app.snapshot().relics[0].equipped);
    assert_eq!(app.snapshot().relics_equipped, 1);
    app.on_intent(Intent::Escape);

    // Une prospection rapporte une relique : la carte se retourne.
    let first = app.snapshot().systems[0].links[0];
    app.on_intent(Intent::StarMap);
    let _ = first;
    app.on_intent(Intent::Buy);
    app.update(0.033);
    let total = app
        .snapshot()
        .fleets
        .first()
        .map(|f| f.total)
        .unwrap_or(0.0);
    app.update(total + 1.0);
    app.on_intent(Intent::Escape);
    let mut saw_card = false;
    for _ in 0..(4 * 30) {
        t += 1.0 / 30.0;
        app.set_time(t);
        app.update(1.0 / 30.0);
        let text = render(&mut app, 100, 30);
        saw_card |= text.contains("RELIQUE TROUVÉE");
    }
    let found = app
        .snapshot()
        .relics
        .iter()
        .filter(|r| r.rarity.is_some())
        .count()
        > 2
        || app.snapshot().shards > 0;
    if found {
        assert!(saw_card, "la carte de la relique s'affiche");
    }
}

#[test]
fn v0_12_cosmic_event_and_convergence_ending() {
    // Un événement en attente : la fenêtre descend, on choisit, l'effet s'applique.
    let mut save = advanced_save();
    save.big_bangs = 1;
    save.attractor = 7;
    save.map_seed = 42;
    save.pending_event = "plasma_storm".into();
    save.pending_left = 1800.0;
    let mut app = app_with(Some(save));
    let mut t = 100.0;
    app.set_time(t);
    app.enter_game_now();
    app.update(0.033);
    app.on_intent(Intent::Event);
    for _ in 0..45 {
        t += 1.0 / 30.0;
        app.set_time(t);
        let _ = render(&mut app, 100, 30);
    }
    let text = render(&mut app, 100, 30);
    println!("{text}");
    assert!(text.contains("ÉVÉNEMENT COSMIQUE"), "{text}");
    assert!(text.contains("TEMPÊTE DE PLASMA"));
    assert!(text.contains("Récolter l'orage"));
    assert!(text.contains("de chances, sinon"));
    app.on_intent(Intent::Buy);
    app.update(0.033);
    assert!(app.snapshot().pending_event.is_none());
    assert_eq!(app.snapshot().events_resolved, 1);

    // Carte complète et 16 reliques : la Convergence, puis la fin de l'Acte III.
    let mut save = advanced_save();
    save.big_bangs = 1;
    save.attractor = 7;
    save.map_seed = 42;
    save.explored = (0..25).collect();
    let catalog = domain::Catalog::standard();
    save.relics = catalog
        .relics
        .iter()
        .map(|r| (r.key.to_string(), 0))
        .collect();
    let mut app = app_with(Some(save));
    let mut t = 100.0;
    app.set_time(t);
    app.enter_game_now();
    app.update(0.033);
    assert!(app.snapshot().convergence_ready);
    app.on_intent(Intent::StarMap);
    // Revenir sur la fonderie.
    for _ in 0..12 {
        app.on_intent(Intent::PrevPanel);
    }
    t += 1.0;
    app.set_time(t);
    let text = render(&mut app, 120, 36);
    assert!(text.contains("lancer la Convergence"), "{text}");
    app.on_intent(Intent::Buy);
    app.update(0.033);
    assert!(app.snapshot().convergence.is_some());
    app.update(48.0 * 3600.0 + 1.0);
    assert!(app.snapshot().converged);
    let mut text = String::new();
    for _ in 0..(30 * 30) {
        t += 1.0 / 30.0;
        app.set_time(t);
        app.update(1.0 / 30.0);
        text = render(&mut app, 100, 30);
    }
    println!("{text}");
    assert!(text.contains("FIN DE L'ACTE III"), "{text}");
    assert!(text.contains("Conçu et forgé par Ajustor"));
}

#[test]
fn help_bar_only_shows_what_is_possible_now() {
    // Nouvelle partie : l'essentiel seulement.
    let mut fresh = app_with(None);
    fresh.enter_game_now();
    fresh.update(0.033);
    let text = render(&mut fresh, 120, 30);
    let bar = text.lines().last().unwrap().to_string();
    println!("{bar}");
    assert!(bar.contains("[Espace] Récolter") && bar.contains("[R] Options"));
    for absent in [
        "[P]", "[U]", "[E]", "[L]", "[X]", "[H]", "[V]", "[G]", "[S]", "[F]",
    ] {
        assert!(
            !bar.contains(absent),
            "{absent} n'a rien à faire là : {bar}"
        );
    }
    // Partie avancée : la Supernova rapporte, l'Atelier est ouvert.
    let mut app = game_app();
    let text = render(&mut app, 120, 30);
    let bar = text.lines().last().unwrap().to_string();
    println!("{bar}");
    assert!(
        bar.contains("[P] Nova") && bar.contains("[E] Atelier"),
        "{bar}"
    );
    assert!(!bar.contains("[U]") && !bar.contains("[X]"));
}

#[test]
fn v0_13_absence_report_stats_and_colors() {
    // Dix heures d'absence : recherche, chantier et flottes ont avancé.
    let mut save = advanced_save();
    save.last_saved -= 10 * 3600;
    save.big_bangs = 1;
    save.attractor = 7;
    save.map_seed = 42;
    save.crunches = 10;
    save.research = "lab_comet_net".into();
    save.research_left = 600.0;
    save.explored = (0..12).collect();
    save.auto_fleets = true;
    save.history = (0..300)
        .map(|i| (i as f64 * 900.0, 2.0 + i as f32 * 0.04))
        .collect();
    save.best_production = 1e14;
    save.milestones = vec![3600.0, 20000.0, 0.0, 0.0, 0.0];
    let mut app = app_with(Some(save));
    app.set_time(1.0);
    app.on_intent(Intent::Other);
    app.set_time(9.0);
    let text = render(&mut app, 100, 30);
    println!("{text}");
    assert!(text.contains("PENDANT TON ABSENCE (10h"), "{text}");
    assert!(
        text.contains("Recherche terminée : Filet à comètes"),
        "{text}"
    );
    assert!(text.contains("expédition(s) rentrée(s)"), "{text}");

    // Infos : la courbe de production et les étapes.
    app.on_intent(Intent::Other);
    app.set_time(10.0);
    app.on_intent(Intent::Info);
    app.set_time(11.0);
    let text = render(&mut app, 100, 30);
    println!("{text}");
    assert!(text.contains("STATISTIQUES"));
    assert!(text.contains('█'), "courbe dessinée");
    assert!(text.contains("Record de production"));
    assert!(text.contains("1re Supernova : 1h 00m"));
}

#[test]
fn v0_14_megastructures_grid() {
    let mut save = advanced_save();
    save.big_bangs = 1;
    save.attractor = 7;
    save.map_seed = 42;
    save.converged = true;
    save.exotic = 200.0;
    let mut app = app_with(Some(save));
    let mut t = 100.0;
    app.set_time(t);
    app.enter_game_now();
    app.update(0.033);
    app.on_intent(Intent::Megastructures);
    t += 1.0;
    app.set_time(t);
    // Un Collecteur, un Relais à droite, une Forge-monde dessous, le Cœur.
    app.on_intent(Intent::Down);
    app.on_intent(Intent::NextPanel);
    app.on_intent(Intent::QuickBuy(0));
    app.on_intent(Intent::NextPanel);
    app.on_intent(Intent::QuickBuy(1));
    app.on_intent(Intent::Down);
    app.on_intent(Intent::QuickBuy(2));
    app.on_intent(Intent::PrevPanel);
    app.on_intent(Intent::QuickBuy(5));
    app.update(0.033);
    let snap = app.snapshot();
    assert_eq!(snap.grid.iter().filter(|c| c.is_some()).count(), 4);
    assert!(snap.grid_effects.production > 1.0);
    t += 1.5;
    app.set_time(t);
    let text = render(&mut app, 120, 40);
    println!("{text}");
    assert!(text.contains("MÉGASTRUCTURES"), "{text}");
    assert!(text.contains("Cœur stellaire"));
    assert!(text.contains("[1] ◇ Collecteur"));
    let text = render(&mut app, 80, 24);
    assert!(text.contains("MÉGASTRUCTURES"), "{text}");
}

#[test]
fn v0_15_daily_challenge() {
    let mut save = advanced_save();
    save.big_bangs = 1;
    save.attractor = 7;
    save.map_seed = 42;
    save.converged = true;
    save.fragments = 77.0;
    let mut app = app_with(Some(save));
    let mut t = 100.0;
    app.set_time(t);
    app.enter_game_now();
    app.update(0.033);
    app.on_intent(Intent::Daily);
    t += 1.0;
    app.set_time(t);
    let text = render(&mut app, 100, 30);
    println!("{text}");
    assert!(text.contains("DÉFI DU JOUR"), "{text}");
    assert!(text.contains("Règles du jour"));
    assert!(text.contains("univers de poche"));
    app.on_intent(Intent::Buy);
    app.update(0.033);
    assert!(app.snapshot().daily.is_some());
    assert_eq!(app.snapshot().fragments, 0.0, "partie vierge");
    for _ in 0..50 {
        app.on_intent(Intent::Harvest);
    }
    for i in 0..(30 * 3) {
        t += 1.0 / 30.0;
        app.set_time(t);
        app.update(1.0 / 30.0);
        let (w, h) = if i % 2 == 0 { (100, 30) } else { (80, 24) };
        let _ = render(&mut app, w, h);
    }
    let text = render(&mut app, 100, 30);
    assert!(
        text.contains("DÉFI"),
        "compte à rebours dans l'en-tête :\n{text}"
    );
    // Dix minutes plus tard : retour à la partie, carte du résultat.
    app.update(600.0);
    let snap = app.snapshot();
    assert!(snap.daily.is_none());
    assert_eq!(snap.fragments, 77.0);
    assert!(snap.daily_best > 0.0);
    let mut seen = false;
    for _ in 0..(30 * 3) {
        t += 1.0 / 30.0;
        app.set_time(t);
        app.update(1.0 / 30.0);
        let text = render(&mut app, 100, 30);
        seen |= text.contains("DÉFI DU JOUR TERMINÉ") && text.contains("Code à partager : FS");
    }
    assert!(seen, "la carte du résultat s'affiche");
}

#[test]
fn v0_16_cosmos_awakening_finale() {
    let mut save = advanced_save();
    save.big_bangs = 1;
    save.attractor = 7;
    save.map_seed = 42;
    save.converged = true;
    // 30 structures des six types, le Cœur en case 22 (centre de la grille).
    save.grid = (0..45)
        .map(|i| match i {
            22 => 5,
            i if i < 31 => (i % 5) as i8,
            _ => -1,
        })
        .collect();
    let mut app = app_with(Some(save));
    let mut t = 100.0;
    app.set_time(t);
    app.enter_game_now();
    app.update(0.033);
    assert!(app.snapshot().awakening_ready);
    let text = render(&mut app, 120, 36);
    assert!(
        text.contains('✶'),
        "la constellation des structures est dans le ciel"
    );
    app.on_intent(Intent::Megastructures);
    // Le Cœur : case 22 = ligne 2, colonne 4.
    for _ in 0..2 {
        app.on_intent(Intent::Down);
    }
    for _ in 0..4 {
        app.on_intent(Intent::NextPanel);
    }
    t += 1.0;
    app.set_time(t);
    let text = render(&mut app, 120, 40);
    assert!(text.contains("lancer l'Éveil du Cosmos"), "{text}");
    app.on_intent(Intent::Buy);
    app.update(0.033);
    assert!(app.snapshot().awakening.is_some());
    app.on_intent(Intent::Escape);
    app.update(72.0 * 3600.0 + 1.0);
    assert!(app.snapshot().awakened);
    let mut text = String::new();
    for _ in 0..(30 * 30) {
        t += 1.0 / 30.0;
        app.set_time(t);
        app.update(1.0 / 30.0);
        text = render(&mut app, 100, 30);
    }
    println!("{text}");
    assert!(text.contains("FIN DE L'ACTE IV"), "{text}");
    assert!(text.contains("Conçu et forgé par Ajustor"));
}
