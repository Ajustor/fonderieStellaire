//! Lisibilité : chaque écran et chaque fenêtre est rendu à plusieurs tailles de
//! terminal, en parcourant toutes les lignes sélectionnables, et aucun texte ne
//! doit être coupé (audit de `draw::text` / `draw::truncate`).
//!
//! `cargo test -p presentation --test readability -- --nocapture` liste les coupures.

use std::cell::RefCell;
use std::rc::Rc;

use application::{
    Clock, GameService, LoadOutcome, RandomSource, RepositoryError, SaveData, SaveRepository,
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

/// Partie de fin de contenu : tout est ouvert, rien n'est encore acheté.
fn late_save() -> SaveData {
    SaveData {
        version: application::SAVE_VERSION,
        stardust: 8.8e15,
        total_run: 9.9e16,
        total_all: 9.9e19,
        owned: vec![250, 220, 200, 180, 160, 140, 120, 100, 80, 60, 40, 20],
        achievements: vec!["first".into(), "m1".into()],
        fragments: 123_456.0,
        supernovas: 15,
        clicks: 987_654,
        play_time: 400_000.0,
        last_saved: 1_700_000_000,
        crunches: 12,
        perks: vec!["perk_start".into()],
        constellations: vec![("const_forge".into(), 4)],
        ..SaveData::default()
    }
}

const SIZES: [(u16, u16); 4] = [(80, 24), (100, 30), (120, 40), (160, 48)];

fn settle(app: &mut App, t: &mut f64, w: u16, h: u16) {
    for _ in 0..45 {
        *t += 1.0 / 30.0;
        app.set_time(*t);
        app.update(1.0 / 30.0);
        let _ = render(app, w, h);
    }
}

/// Ouvre une fenêtre, parcourt `steps` lignes et renvoie les textes coupés.
fn audit(name: &str, save: SaveData, open: &[Intent], steps: usize) -> Vec<String> {
    let report = audit_raw(save, open, steps);
    report
        .into_iter()
        .map(|(t, sizes)| format!("[{name} {}] {t}", sizes.join(" ")))
        .collect()
}

fn audit_raw(save: SaveData, open: &[Intent], steps: usize) -> Vec<(String, Vec<String>)> {
    let mut report = Vec::new();
    for (w, h) in SIZES {
        let mut app = app_with(Some(save.clone()));
        let mut t = 100.0;
        app.set_time(t);
        app.enter_game_now();
        settle(&mut app, &mut t, w, h);
        for i in open {
            app.on_intent(*i);
        }
        settle(&mut app, &mut t, w, h);
        for _ in 0..=steps {
            presentation::draw::audit::start();
            let text = render(&mut app, w, h);
            let clips = presentation::draw::audit::take();
            if !clips.is_empty() {
                if std::env::var("SHOW_SCREENS").is_ok() {
                    println!("{text}");
                }
                for c in clips {
                    let size = format!("{w}×{h}");
                    match report
                        .iter_mut()
                        .find(|(t, _): &&mut (String, Vec<String>)| *t == c)
                    {
                        Some((_, sizes)) => {
                            if !sizes.contains(&size) {
                                sizes.push(size);
                            }
                        }
                        None => report.push((c, vec![size])),
                    }
                }
            }
            app.on_intent(Intent::Down);
            t += 0.5;
            app.set_time(t);
            app.update(0.033);
        }
    }
    report
}

#[test]
fn no_text_is_cut_anywhere() {
    let late = late_save();
    let mut report = Vec::new();
    report.extend(audit("jeu", advanced_save(), &[], 12));
    report.extend(audit("jeu (fin)", late.clone(), &[], 12));
    report.extend(audit(
        "améliorations",
        late.clone(),
        &[Intent::NextPanel],
        25,
    ));
    report.extend(audit("atelier", late.clone(), &[Intent::Shop], 20));
    report.extend(audit("défis", late.clone(), &[Intent::Challenges], 6));
    report.extend(audit("labo", late.clone(), &[Intent::Lab], 13));
    let mut stats = late.clone();
    stats.history = (0..2000)
        .map(|i| (i as f64 * 900.0, 1.0 + i as f32 * 0.01))
        .collect();
    stats.best_production = 9.9e21;
    stats.milestones = vec![3600.0, 90_000.0, 900_000.0, 2_000_000.0, 3_000_000.0];
    report.extend(audit("infos", stats, &[Intent::Info], 60));
    report.extend(audit("supernova", late.clone(), &[Intent::Supernova], 0));
    report.extend(audit("big crunch", late.clone(), &[Intent::BigCrunch], 0));
    report.extend(audit("univers", late.clone(), &[Intent::Universes], 0));
    report.extend(audit("options", late.clone(), &[Intent::Options], 2));
    let mut act3 = late.clone();
    act3.big_bangs = 1;
    act3.attractor = 7;
    report.extend(audit(
        "carte stellaire",
        act3.clone(),
        &[Intent::StarMap],
        6,
    ));
    let mut relics = act3.clone();
    relics.relics = vec![
        ("r_hammer".into(), 3),
        ("r_compass".into(), 1),
        ("r_mirror".into(), 2),
        ("r_tear".into(), 0),
    ];
    relics.equipped = vec!["r_hammer".into()];
    relics.shards = 12;
    report.extend(audit("reliques", relics, &[Intent::Relics], 8));
    let mut mega = act3.clone();
    mega.map_seed = 42;
    mega.converged = true;
    mega.exotic = 12345.0;
    mega.grid = (0..45)
        .map(|i| if i % 3 == 0 { (i % 6) as i8 } else { -1 })
        .collect();
    report.extend(audit(
        "mégastructures",
        mega.clone(),
        &[Intent::Megastructures],
        10,
    ));
    report.extend(audit("défi du jour", mega.clone(), &[Intent::Daily], 1));
    let mut awake = mega;
    awake.grid = (0..45)
        .map(|i| if i == 22 { 5 } else { (i % 5) as i8 })
        .collect();
    awake.awakening_left = 50_000.0;
    report.extend(audit("éveil", awake, &[Intent::Megastructures], 25));
    for key in ["civ_signal", "plasma_storm", "merchant", "nearby_nova"] {
        let mut ev = act3.clone();
        ev.map_seed = 42;
        ev.pending_event = key.into();
        ev.pending_left = 3000.0;
        report.extend(audit(&format!("événement {key}"), ev, &[Intent::Event], 2));
    }
    let mut bang = late.clone();
    bang.crunches = 34;
    report.extend(audit("cosmogonie", bang, &[Intent::Cosmos], 11));
    let mut built = late.clone();
    built.big_bangs = 2;
    built.particles = 9;
    built.particles_spent = 4;
    built.forged = vec![12];
    built.attractor = 3;
    built.attractor_left = 5000.0;
    built.research = "lab_prod2".into();
    built.research_left = 7000.0;
    report.extend(audit("chantiers", built.clone(), &[], 0));
    report.extend(audit(
        "cosmogonie (chantier)",
        built.clone(),
        &[Intent::Cosmos],
        11,
    ));
    report.extend(audit("labo (recherche)", built, &[Intent::Lab], 13));
    for r in &report {
        println!("{r}");
    }
    assert!(report.is_empty(), "{} texte(s) coupé(s)", report.len());
}

/// `cargo test -p presentation --test readability dump -- --ignored --nocapture`
#[test]
#[ignore]
fn dump_screens() {
    let size: Vec<u16> = std::env::var("DUMP_SIZE")
        .unwrap_or_else(|_| "80x24".into())
        .split('x')
        .map(|n| n.parse().unwrap())
        .collect();
    let (w, h) = (size[0], size[1]);
    let scenarios: [(&str, &[Intent]); 7] = [
        ("jeu", &[]),
        ("améliorations", &[Intent::NextPanel]),
        ("atelier", &[Intent::Shop]),
        ("défis", &[Intent::Challenges]),
        ("labo", &[Intent::Lab]),
        ("infos", &[Intent::Info]),
        ("univers", &[Intent::Universes]),
    ];
    for (name, open) in scenarios {
        let mut app = app_with(Some(late_save()));
        let mut t = 100.0;
        app.set_time(t);
        app.enter_game_now();
        settle(&mut app, &mut t, w, h);
        for i in open {
            app.on_intent(*i);
        }
        settle(&mut app, &mut t, w, h);
        println!("===== {name} =====\n{}", render(&mut app, w, h));
    }
}

/// Profil d'une partie avancée : temps de calcul, de rendu, et volume envoyé
/// au terminal (cellules modifiées d'une image à l'autre).
/// `cargo test --release -p presentation --test readability profile -- --ignored --nocapture`
#[test]
#[ignore]
fn profile_frames() {
    use std::time::Instant;
    for (w, h) in [(120u16, 40u16), (200, 60)] {
        let mut app = app_with(Some(late_save()));
        let mut t = 100.0;
        app.set_time(t);
        app.enter_game_now();
        settle(&mut app, &mut t, w, h);
        let mut terminal = Terminal::new(TestBackend::new(w, h)).expect("terminal");
        let mut prev = Buffer::empty(ratatui::layout::Rect::new(0, 0, w, h));
        let (mut upd, mut ren, mut cells, mut worst) = (0.0f64, 0.0f64, 0usize, 0.0f64);
        let frames = 300;
        for _ in 0..frames {
            t += 1.0 / 30.0;
            app.set_time(t);
            let a = Instant::now();
            app.update(1.0 / 30.0);
            let b = Instant::now();
            terminal.draw(|f| app.render(f)).expect("draw");
            let c = Instant::now();
            let buf = terminal.backend().buffer().clone();
            cells += prev.diff(&buf).len();
            prev = buf;
            upd += (b - a).as_secs_f64();
            ren += (c - b).as_secs_f64();
            worst = worst.max((c - a).as_secs_f64());
        }
        let n = frames as f64;
        println!(
            "{w}×{h} : calcul {:.3} ms · rendu {:.3} ms · pire image {:.2} ms · {:.0} cellules changées/image ({:.0}% de l'écran)",
            upd / n * 1e3,
            ren / n * 1e3,
            worst * 1e3,
            cells as f64 / n,
            cells as f64 / n / (w as f64 * h as f64) * 100.0
        );
    }
}

/// Volume réellement écrit vers le terminal (séquences d'échappement comprises)
/// et nombre d'écritures système, avec la sortie standard telle quelle
/// (`LineWriter`) ou derrière un tampon.
#[test]
#[ignore]
fn profile_terminal_output() {
    use ratatui::backend::CrosstermBackend;
    use ratatui::layout::Rect;
    use ratatui::{TerminalOptions, Viewport};
    use std::io::{LineWriter, Write};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Clone)]
    struct Sink(Arc<AtomicUsize>, Arc<AtomicUsize>);
    impl Write for Sink {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.fetch_add(b.len(), Ordering::Relaxed);
            self.1.fetch_add(1, Ordering::Relaxed);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let (w, h) = (160u16, 48u16);
    for buffered in [false, true] {
        let bytes = Arc::new(AtomicUsize::new(0));
        let calls = Arc::new(AtomicUsize::new(0));
        let sink = Sink(bytes.clone(), calls.clone());
        let writer: Box<dyn Write> = if buffered {
            Box::new(std::io::BufWriter::with_capacity(1 << 16, sink))
        } else {
            Box::new(LineWriter::new(sink))
        };
        let mut terminal = Terminal::with_options(
            CrosstermBackend::new(writer),
            TerminalOptions {
                viewport: Viewport::Fixed(Rect::new(0, 0, w, h)),
            },
        )
        .expect("terminal");
        let mut app = app_with(Some(late_save()));
        let mut t = 100.0;
        app.set_time(t);
        app.enter_game_now();
        let frames = 300;
        for i in 0..frames + 30 {
            if i == 30 {
                bytes.store(0, Ordering::Relaxed);
                calls.store(0, Ordering::Relaxed);
            }
            t += 1.0 / 30.0;
            app.set_time(t);
            app.update(1.0 / 30.0);
            terminal.draw(|f| app.render(f)).expect("draw");
        }
        let n = frames as f64;
        println!(
            "{} : {:.1} Ko/image ({:.0} Ko/s), {:.1} écritures système/image",
            if buffered {
                "tamponné"
            } else {
                "LineWriter (actuel)"
            },
            bytes.load(Ordering::Relaxed) as f64 / n / 1024.0,
            bytes.load(Ordering::Relaxed) as f64 / n / 1024.0 * 30.0,
            calls.load(Ordering::Relaxed) as f64 / n
        );
    }
}

/// Les grosses animations plein écran : temps de rendu et volume terminal.
#[test]
#[ignore]
fn profile_animations() {
    use ratatui::backend::CrosstermBackend;
    use ratatui::layout::Rect;
    use ratatui::{TerminalOptions, Viewport};
    use std::io::{LineWriter, Write};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Instant;

    #[derive(Clone)]
    struct Sink(Arc<AtomicUsize>, Arc<AtomicUsize>);
    impl Write for Sink {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.fetch_add(b.len(), Ordering::Relaxed);
            self.1.fetch_add(1, Ordering::Relaxed);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let (w, h) = (160u16, 48u16);
    let scenarios: [(&str, &[Intent]); 4] = [
        ("jeu", &[]),
        ("supernova", &[Intent::Supernova, Intent::Yes]),
        ("portails", &[Intent::Universes]),
        ("infos", &[Intent::Info]),
    ];
    for (name, open) in scenarios {
        let bytes = Arc::new(AtomicUsize::new(0));
        let calls = Arc::new(AtomicUsize::new(0));
        let sink = Sink(bytes.clone(), calls.clone());
        let mut terminal = Terminal::with_options(
            CrosstermBackend::new(LineWriter::new(sink)),
            TerminalOptions {
                viewport: Viewport::Fixed(Rect::new(0, 0, w, h)),
            },
        )
        .expect("terminal");
        let mut app = app_with(Some(late_save()));
        let mut t = 100.0;
        app.set_time(t);
        app.enter_game_now();
        for _ in 0..30 {
            t += 1.0 / 30.0;
            app.set_time(t);
            app.update(1.0 / 30.0);
            terminal.draw(|f| app.render(f)).expect("draw");
        }
        for i in open {
            app.on_intent(*i);
        }
        bytes.store(0, Ordering::Relaxed);
        calls.store(0, Ordering::Relaxed);
        let frames = 180;
        let (mut total, mut worst) = (0.0f64, 0.0f64);
        for _ in 0..frames {
            t += 1.0 / 30.0;
            app.set_time(t);
            let a = Instant::now();
            app.update(1.0 / 30.0);
            terminal.draw(|f| app.render(f)).expect("draw");
            let d = a.elapsed().as_secs_f64();
            total += d;
            worst = worst.max(d);
        }
        let n = frames as f64;
        println!(
            "{name:>10} : {:.2} ms/image (pire {:.2}) · {:.1} Ko/image · {:.1} écritures/image",
            total / n * 1e3,
            worst * 1e3,
            bytes.load(Ordering::Relaxed) as f64 / n / 1024.0,
            calls.load(Ordering::Relaxed) as f64 / n
        );
    }
}

/// Quelques images du Big Bang, pour juger l'animation à l'œil.
#[test]
#[ignore]
fn dump_bang() {
    let mut save = late_save();
    save.crunches = 34;
    let mut app = app_with(Some(save));
    let mut t = 100.0;
    app.set_time(t);
    app.enter_game_now();
    settle(&mut app, &mut t, 100, 30);
    app.on_intent(Intent::Cosmos);
    app.on_intent(Intent::Buy);
    app.on_intent(Intent::Buy);
    let start = t;
    let marks = [0.8, 1.4, 2.1, 2.6, 3.0, 3.6, 4.4, 5.6, 7.0];
    let mut next = 0;
    while next < marks.len() {
        t += 1.0 / 30.0;
        app.set_time(t);
        app.update(1.0 / 30.0);
        let text = render(&mut app, 100, 30);
        if t - start >= marks[next] {
            println!("===== t = {:.1} s =====\n{text}", marks[next]);
            next += 1;
        }
    }
}

/// Relais factice : l'appareil est lié, chaque envoi réussit.
struct LinkedRelay;

impl application::SyncBackend for LinkedRelay {
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

#[test]
fn sync_screen_shows_the_qr_code_without_cutting_text() {
    for (w, h) in SIZES {
        let mut app = app_with(Some(late_save()));
        app.set_sync(application::SyncService::spawn(Box::new(LinkedRelay)));
        let mut t = 100.0;
        app.set_time(t);
        app.enter_game_now();
        settle(&mut app, &mut t, w, h);
        app.on_intent(Intent::Options);
        for _ in 0..3 {
            app.on_intent(Intent::Down);
        }
        app.on_intent(Intent::Buy);
        settle(&mut app, &mut t, w, h);
        presentation::draw::audit::start();
        let text = render(&mut app, w, h);
        let clips = presentation::draw::audit::take();
        assert!(clips.is_empty(), "{w}×{h} : texte coupé {clips:?}\n{text}");
        assert!(text.contains("SYNCHRONISATION"), "{w}×{h}\n{text}");
        assert!(
            text.contains("ABCDE-FGHJK-MNPQR-STVWX-YZ012"),
            "{w}×{h}\n{text}"
        );
        assert!(
            text.matches('▀').count() > 300,
            "QR code absent à {w}×{h}\n{text}"
        );
        // Délier demande une confirmation.
        app.on_intent(Intent::Buy);
        settle(&mut app, &mut t, w, h);
        assert!(render(&mut app, w, h).contains("confirmer"));
        app.on_intent(Intent::Escape);
        app.on_intent(Intent::Escape);
        settle(&mut app, &mut t, w, h);
        assert!(render(&mut app, w, h).contains("Synchronisation"));
    }
}
