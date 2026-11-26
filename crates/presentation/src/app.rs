//! Boucle principale : entrée → `Command` → événements → effets/journal → rendu.

use std::io;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, BorderType, Borders, Widget};
use ratatui::Frame;

use application::{Command, GameEvent, GameService, GameSnapshot};
use domain::catalog::balance::{ACHIEVEMENT_BONUS, FRAGMENT_BONUS};
use domain::{CometReward, DomainEvent, Effect, GeneratorId, PurchaseTarget, Requirement};

use crate::draw;
use crate::fmt;
use crate::input::{map_event, HoldDetector, Intent};
use crate::layout::is_too_small;
use crate::screens::game::{self, GameView};
use crate::screens::supernova::{NovaSequence, WAVE_AT};
use crate::screens::title::TitleScreen;
use crate::screens::too_small;
use crate::terminal::{Term, TerminalGuard};
use crate::theme;
use crate::ui::{Panel, Popup, Ui};
use crate::visuals::{Visuals, NEVER};
use crate::widgets::journal::{Journal, Kind};
use crate::widgets::toast::Toasts;
use crate::widgets::{inner, Ctx};

/// ≈ 30 images par seconde.
const FRAME: Duration = Duration::from_millis(33);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Title,
    Game,
}

/// Compteurs de performance affichés par l'overlay F3.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameStats {
    pub fps: f64,
    pub render_ms: f64,
    pub render_max_ms: f64,
    pub frame_ms: f64,
}

impl FrameStats {
    fn record(&mut self, dt: f64, render: Duration, frame: Duration) {
        let r = render.as_secs_f64() * 1000.0;
        let f = frame.as_secs_f64() * 1000.0;
        let k = 0.1;
        if dt > 0.0 {
            self.fps += (1.0 / dt - self.fps) * k;
        }
        self.render_ms += (r - self.render_ms) * k;
        self.frame_ms += (f - self.frame_ms) * k;
        self.render_max_ms = (self.render_max_ms * 0.995).max(r);
    }
}

pub struct RunOutcome {
    /// Erreur de la sauvegarde finale, le cas échéant.
    pub save_error: Option<String>,
}

pub struct App {
    service: GameService,
    snap: GameSnapshot,
    screen: Screen,
    ui: Ui,
    vis: Visuals,
    title: TitleScreen,
    journal: Journal,
    toasts: Toasts,
    spark_scratch: Vec<u64>,
    start: Instant,
    now: f64,
    dt: f64,
    quit: bool,
    stats: FrameStats,
    hold: HoldDetector,
    /// Dernière fois qu'on a expliqué comment débloquer la récolte continue.
    hold_hint_at: f64,
}

fn seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(42)
}

impl App {
    /// `startup` : événements produits par le chargement (gains hors-ligne…).
    pub fn new(service: GameService, startup: Vec<GameEvent>) -> Self {
        let snap = service.snapshot();
        let s = seed();
        let mut app = App {
            vis: Visuals::new(service.catalog().upgrades.len(), s),
            service,
            snap,
            screen: Screen::Title,
            ui: Ui::default(),
            title: TitleScreen::new(0.0, s ^ 0x7177),
            journal: Journal::default(),
            toasts: Toasts::default(),
            spark_scratch: Vec::with_capacity(256),
            start: Instant::now(),
            now: 0.0,
            dt: 0.0,
            quit: false,
            stats: FrameStats::default(),
            hold: HoldDetector::default(),
            hold_hint_at: NEVER,
        };
        app.vis.set_reduced(app.snap.reduced_effects);
        app.journal.push(
            app.snap.play_time,
            0.0,
            Kind::Info,
            "✦ Bienvenue dans la Fonderie Stellaire !",
        );
        for ev in startup {
            app.react(ev);
        }
        app
    }

    pub fn screen(&self) -> Screen {
        self.screen
    }

    pub fn snapshot(&self) -> &GameSnapshot {
        &self.snap
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    /// Passe directement à l'écran de jeu (sans cascade) — utile aux tests.
    pub fn enter_game_now(&mut self) {
        self.screen = Screen::Game;
        self.vis.cascade_start = self.now - 10.0;
    }

    /// Avance l'horloge interne manuellement (tests).
    pub fn set_time(&mut self, now: f64) {
        self.now = now;
    }

    fn command(&mut self, cmd: Command) {
        for ev in self.service.handle(cmd) {
            self.react(ev);
        }
    }

    // ------------------------------------------------------------------
    // Boucle
    // ------------------------------------------------------------------

    pub fn run_loop(&mut self, terminal: &mut Term) -> io::Result<()> {
        let mut last = Instant::now();
        while !self.quit {
            let frame_start = Instant::now();
            let dt = frame_start.duration_since(last).as_secs_f64();
            last = frame_start;
            self.now = self.start.elapsed().as_secs_f64();
            self.update(dt);

            let mut render_time = Duration::ZERO;
            terminal.draw(|f| {
                let t0 = Instant::now();
                self.render(f);
                render_time = t0.elapsed();
            })?;
            self.stats.record(dt, render_time, frame_start.elapsed());

            let timeout = FRAME.saturating_sub(frame_start.elapsed());
            if event::poll(timeout)? {
                loop {
                    let ev = event::read()?;
                    self.on_terminal_event(&ev);
                    if self.quit || !event::poll(Duration::ZERO)? {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn on_terminal_event(&mut self, ev: &Event) {
        if let Event::Resize(_, _) = ev {
            // Les particules sont en coordonnées écran : on repart proprement.
            self.vis.particles.clear();
            self.vis.floaters.clear();
            return;
        }
        if let Event::Key(k) = ev {
            if k.kind == KeyEventKind::Release {
                self.hold.saw_release();
            }
            if k.code == KeyCode::Char(' ') && !k.modifiers.contains(KeyModifiers::CONTROL) {
                let t = self.start.elapsed().as_secs_f64();
                if let Some(intent) = self.hold.on_space(k.kind, t) {
                    self.on_intent(intent);
                }
                return;
            }
        }
        if let Some(intent) = map_event(ev) {
            self.on_intent(intent);
        }
    }

    /// Avance la logique (temps réel) et les effets.
    pub fn update(&mut self, real_dt: f64) {
        let dt = real_dt.clamp(0.0, 0.25);
        self.dt = dt;
        let now = self.now;
        match self.screen {
            Screen::Title => self.title.update(dt, now),
            Screen::Game => {
                for ev in self.service.tick(real_dt) {
                    self.react(ev);
                }
                self.update_nova(now);
                if let Some(p) = self.ui.popup {
                    if p.is_closed(now) {
                        self.ui.popup = None;
                    }
                }
            }
        }
        self.service.fill_snapshot(&mut self.snap);
        let reduced = self.snap.reduced_effects;
        let sky = inner(self.ui.layout.sky);
        self.vis.update(dt, now, sky, reduced);
        self.toasts.update(now);
    }

    fn update_nova(&mut self, now: f64) {
        let Some(nova) = self.vis.nova.as_ref() else {
            self.vis.sun_scale = 1.0;
            return;
        };
        let t = nova.t(now);
        let fire_wave = !nova.wave_fired && t >= WAVE_AT;
        let commit = !nova.committed && t >= crate::screens::supernova::COMMIT_AT;
        let done = nova.is_done(now);
        self.vis.sun_scale = nova.sun_scale(now);
        if fire_wave {
            if let Some(n) = self.vis.nova.as_mut() {
                n.wave_fired = true;
            }
            let area = self
                .ui
                .layout
                .panels()
                .iter()
                .fold(Rect::default(), |a, r| a.union(*r));
            let cx = area.x as f64 + area.width as f64 / 2.0;
            let cy = area.y as f64 + area.height as f64 / 2.0;
            self.vis.explode(
                cx,
                cy,
                140,
                60.0,
                &[theme::WHITE, theme::GOLD, theme::ORANGE, theme::PINK],
            );
        }
        if commit {
            if let Some(n) = self.vis.nova.as_mut() {
                n.committed = true;
            }
            self.command(Command::Supernova);
        }
        if done {
            self.vis.nova = None;
            self.vis.sun_scale = 1.0;
            self.vis.cascade_start = now;
            self.vis.particles.clear();
        }
    }

    // ------------------------------------------------------------------
    // Entrées
    // ------------------------------------------------------------------

    pub fn on_intent(&mut self, intent: Intent) {
        let now = self.now;
        if intent == Intent::ToggleDebug {
            self.ui.debug = !self.ui.debug;
            return;
        }
        match self.screen {
            Screen::Title => match intent {
                Intent::Quit | Intent::Escape => self.quit = true,
                // Une touche maintenue ne doit pas lancer la partie toute seule.
                Intent::HarvestHeld => {}
                _ if !self.title.intro_done(now) => self.title.skip(),
                _ => {
                    self.screen = Screen::Game;
                    self.vis.cascade_start = now;
                }
            },
            Screen::Game => self.on_game_intent(intent),
        }
    }

    fn on_game_intent(&mut self, intent: Intent) {
        let now = self.now;
        if self.vis.nova.is_some() {
            if intent == Intent::Quit {
                self.quit = true;
            }
            return;
        }
        if let Some(popup) = self.ui.popup {
            if popup.closing_at.is_none() {
                match intent {
                    Intent::Yes => {
                        self.close_popup();
                        self.start_supernova();
                    }
                    Intent::No | Intent::Escape | Intent::Quit => self.close_popup(),
                    _ => {}
                }
            }
            return;
        }
        match intent {
            Intent::Harvest => self.command(Command::Harvest),
            Intent::HarvestHeld => self.command(Command::HarvestHeld),
            Intent::Up => match self.ui.focus {
                Panel::Generators => self.ui.gen_sel = self.ui.gen_sel.saturating_sub(1),
                Panel::Upgrades => self.ui.upg_sel = self.ui.upg_sel.saturating_sub(1),
            },
            Intent::Down => match self.ui.focus {
                Panel::Generators => {
                    let max = self.snap.generators.len().saturating_sub(1);
                    self.ui.gen_sel = (self.ui.gen_sel + 1).min(max);
                }
                Panel::Upgrades => {
                    let max = self.snap.upgrades.len().saturating_sub(1);
                    self.ui.upg_sel = (self.ui.upg_sel + 1).min(max);
                }
            },
            Intent::NextPanel | Intent::PrevPanel => {
                self.ui.focus = self.ui.focus.toggle();
                self.ui.focus_changed_at = now;
            }
            Intent::Buy => match self.ui.focus {
                Panel::Generators => {
                    if let Some(g) = self.snap.generators.get(self.ui.gen_sel) {
                        let id = g.id;
                        self.command(Command::BuyGenerator(id));
                    }
                }
                Panel::Upgrades => {
                    if let Some(u) = self.snap.upgrades.get(self.ui.upg_sel) {
                        let id = u.id;
                        self.command(Command::BuyUpgrade(id));
                    }
                }
            },
            Intent::QuickBuy(n) => {
                if n < self.service.catalog().generators.len() {
                    self.command(Command::BuyGenerator(GeneratorId(n)));
                }
            }
            Intent::CycleMode => self.command(Command::CycleBuyMode),
            Intent::CatchComet => {
                if self.snap.comet_progress.is_some() {
                    self.command(Command::CatchComet);
                }
            }
            Intent::Supernova => {
                if self.snap.fragments_available >= 1.0 {
                    self.ui.popup = Some(Popup {
                        opened_at: now,
                        closing_at: None,
                    });
                } else {
                    self.command(Command::Supernova);
                }
            }
            Intent::Save => self.command(Command::Save),
            Intent::ToggleEffects => self.command(Command::ToggleReducedEffects),
            Intent::Quit | Intent::Escape => self.quit = true,
            Intent::Yes | Intent::No | Intent::Other | Intent::ToggleDebug => {}
        }
    }

    fn close_popup(&mut self) {
        if let Some(p) = self.ui.popup.as_mut() {
            p.closing_at = Some(self.now);
        }
    }

    fn start_supernova(&mut self) {
        let gained = self.snap.fragments_available;
        let old_bonus = self.snap.fragment_bonus;
        let new_bonus = (self.snap.fragments + gained) * FRAGMENT_BONUS;
        self.vis.nova = Some(NovaSequence::new(self.now, gained, old_bonus, new_bonus));
    }

    // ------------------------------------------------------------------
    // Réactions aux événements : chaque événement a un effet visuel.
    // ------------------------------------------------------------------

    fn log(&mut self, kind: Kind, text: impl Into<String>) {
        self.journal.push(self.snap.play_time, self.now, kind, text);
    }

    pub fn react(&mut self, ev: GameEvent) {
        let now = self.now;
        match ev {
            GameEvent::Domain(d) => self.react_domain(d),
            GameEvent::BuyModeChanged(_) => self.vis.mode_flash = now,
            GameEvent::ReducedEffectsChanged(reduced) => {
                self.vis.set_reduced(reduced);
                let msg = if reduced {
                    "Effets réduits activés"
                } else {
                    "Effets complets activés"
                };
                self.log(Kind::Info, msg);
                self.toasts.push(now, theme::CYAN, "Affichage", msg);
            }
            GameEvent::Saved { automatic } => {
                let msg = if automatic {
                    "Sauvegarde automatique"
                } else {
                    "Partie sauvegardée"
                };
                self.log(Kind::Save, msg);
            }
            GameEvent::SaveFailed(msg) => {
                self.log(Kind::Error, format!("Échec de la sauvegarde : {msg}"));
                self.toasts
                    .push(now, theme::RED, "Sauvegarde impossible", msg);
            }
            GameEvent::SaveRecovered { backup } => {
                self.log(
                    Kind::Error,
                    format!("Sauvegarde corrompue, copiée dans {backup}. Nouvelle partie."),
                );
            }
        }
    }

    fn react_domain(&mut self, ev: DomainEvent) {
        let now = self.now;
        let sky = inner(self.ui.layout.sky);
        match ev {
            DomainEvent::Harvested { amount } => {
                let (x, y) = if sky.is_empty() {
                    (0.0, 0.0)
                } else {
                    self.vis.random_point(sky)
                };
                let n = 6 + self.vis.rng.below(5);
                self.vis.particles.burst(x, y, n, 14.0, &theme::SPARKS);
                self.vis
                    .add_floater(format!("+{}", fmt::short(amount)), x, y, now, theme::GOLD);
                self.vis.bump_counter(now);
            }
            DomainEvent::Purchased {
                generator, count, ..
            } => {
                let gi = generator.index().min(8);
                self.vis.gen_flash[gi] = now;
                let color = theme::generator_color(gi);
                if let Some(y) = self.ui.gen_rows[gi] {
                    let r = self.ui.layout.generators;
                    self.vis
                        .line_sparks(r.x as f64 + 3.0, r.right() as f64 - 3.0, y as f64, color);
                }
                self.vis.roll_counter(now);
                if count > 1 {
                    let name = self.generator_plural(generator);
                    self.log(Kind::Purchase, format!("Achat : {count} {name}"));
                }
            }
            DomainEvent::UpgradePurchased { upgrade, .. } => {
                if let Some(t) = self.vis.upg_flash.get_mut(upgrade.index()) {
                    *t = now;
                }
                if let Some(&(_, y)) = self.ui.upg_rows.iter().find(|(id, _)| *id == upgrade) {
                    let r = self.ui.layout.upgrades;
                    self.vis.line_sparks(
                        r.x as f64 + 3.0,
                        r.right() as f64 - 3.0,
                        y as f64,
                        theme::GREEN,
                    );
                }
                self.vis.roll_counter(now);
                let name = self
                    .service
                    .catalog()
                    .upgrade(upgrade)
                    .map(|u| u.name.clone())
                    .unwrap_or_default();
                self.log(Kind::Purchase, format!("Amélioration : {name}"));
                let is_hold = self
                    .service
                    .catalog()
                    .upgrade(upgrade)
                    .is_some_and(|u| u.effect == Effect::HoldHarvest);
                if is_hold {
                    self.toasts.push(
                        now,
                        theme::GREEN,
                        "✦ Récolte continue !",
                        "Maintiens [Espace] pour farmer en continu",
                    );
                }
            }
            DomainEvent::PurchaseFailed { target } => match target {
                PurchaseTarget::Generator(g) => self.vis.gen_fail[g.index().min(8)] = now,
                PurchaseTarget::Upgrade(u) => {
                    if let Some(t) = self.vis.upg_fail.get_mut(u.index()) {
                        *t = now;
                    }
                }
            },
            DomainEvent::MilestoneReached { generator, count } => {
                self.vis.fireworks(now);
                let name = self.generator_plural(generator);
                self.toasts.push(
                    now,
                    theme::CYAN,
                    "◆ Palier atteint !",
                    format!("{count} {name} : production ×2"),
                );
                self.log(Kind::Milestone, format!("◆ Palier : {count} {name} (×2)"));
            }
            DomainEvent::GeneratorRevealed { generator } => {
                self.vis.gen_reveal[generator.index().min(8)] = now;
                let name = self
                    .service
                    .catalog()
                    .generator(generator)
                    .map(|g| g.name)
                    .unwrap_or("?");
                self.log(Kind::Milestone, format!("▸ Nouveau générateur : {name}"));
            }
            DomainEvent::AchievementUnlocked { achievement } => {
                let name = self
                    .service
                    .catalog()
                    .achievement(achievement)
                    .map(|a| a.name)
                    .unwrap_or("?");
                let pct = fmt::percent(ACHIEVEMENT_BONUS);
                self.toasts.push(
                    now,
                    theme::GOLD,
                    "★ Succès débloqué",
                    format!("{name} (+{pct})"),
                );
                self.log(Kind::Achievement, format!("★ Succès : {name} (+{pct})"));
                if !sky.is_empty() {
                    let (x, y) = self.vis.random_point(sky);
                    self.vis
                        .explode(x, y, 18, 14.0, &[theme::GOLD, theme::WHITE]);
                }
            }
            DomainEvent::CometSpawned => {
                let h = sky.height.max(4) as f64;
                self.vis.comet.y0 = self.vis.rng.range(0.45, 0.8) * h;
                self.vis.comet.amp = -self.vis.rng.range(0.15, 0.35) * h;
                self.vis.comet.trail_acc = 0.0;
                self.log(Kind::Comet, "● Une comète approche ! [C] pour la capturer");
            }
            DomainEvent::CometMissed => {
                self.log(Kind::Save, "La comète s'éloigne dans le vide…");
            }
            DomainEvent::CometCaught { reward } => {
                let (x, y) = self.vis.comet.head.unwrap_or_else(|| {
                    (
                        sky.x as f64 + sky.width as f64 / 2.0,
                        sky.y as f64 + sky.height as f64 / 2.0,
                    )
                });
                self.vis.explode(
                    x,
                    y,
                    40,
                    22.0,
                    &[
                        theme::WHITE,
                        theme::CYAN,
                        theme::PINK,
                        theme::COMET_TRAIL[2],
                    ],
                );
                self.vis.sky_flash = now;
                self.vis.sky_flash_color = theme::PINK.glow(0.5);
                match reward {
                    CometReward::SolarStorm {
                        multiplier,
                        duration,
                    } => {
                        self.toasts.push(
                            now,
                            theme::ORANGE,
                            "● Tempête solaire !",
                            format!("Production ×{multiplier} pendant {duration} s"),
                        );
                        self.log(Kind::Comet, "● Comète capturée : Tempête solaire !");
                    }
                    CometReward::StarShower { amount } => {
                        let a = fmt::short(amount);
                        self.toasts.push(
                            now,
                            theme::PINK,
                            "● Pluie d'étoiles !",
                            format!("+{a} ✦"),
                        );
                        self.log(
                            Kind::Comet,
                            format!("● Comète capturée : Pluie d'étoiles, +{a} ✦"),
                        );
                        self.vis.bump_counter(now);
                    }
                }
            }
            DomainEvent::SolarStormEnded => {
                self.log(Kind::Info, "La tempête solaire s'apaise.");
            }
            DomainEvent::Supernova {
                fragments_gained,
                total_fragments,
            } => {
                self.ui.gen_sel = 0;
                self.ui.upg_sel = 0;
                self.ui.gen_scroll = 0;
                self.ui.upg_scroll = 0;
                self.vis.bar_fill = [0.0; 9];
                self.vis.roll_counter(now);
                self.log(
                    Kind::Supernova,
                    format!(
                        "✦ SUPERNOVA ! +{} fragments (total {})",
                        fmt::short(fragments_gained),
                        fmt::short(total_fragments)
                    ),
                );
            }
            DomainEvent::HoldHarvestLocked => {
                // Indice discret, pas plus d'une fois toutes les 15 s.
                if now - self.hold_hint_at > 15.0 {
                    self.hold_hint_at = now;
                    let catalog = self.service.catalog();
                    let hold = catalog
                        .upgrades
                        .iter()
                        .find(|u| u.effect == Effect::HoldHarvest);
                    let (name, req) = match hold {
                        Some(u) => (
                            u.name.clone(),
                            match u.requirement {
                                Requirement::TotalRun(r) => fmt::short(r),
                                _ => String::new(),
                            },
                        ),
                        None => ("Récolte continue".into(), String::new()),
                    };
                    let available = hold
                        .map(|u| self.snap.upgrades.iter().any(|v| v.id == u.id))
                        .unwrap_or(false);
                    let body = if available {
                        format!("Achète « {name} » dans les améliorations")
                    } else {
                        format!("Débloque « {name} » ({req} ✦ récoltés)")
                    };
                    self.toasts
                        .push(now, theme::CYAN, "Maintenir [Espace] ?", body.clone());
                    self.log(Kind::Info, format!("Maintenir Espace : {body}"));
                }
            }
            DomainEvent::SupernovaUnavailable => {
                self.log(
                    Kind::Save,
                    "Supernova indisponible : récolte 100 M ✦ dans ce cycle pour 1 fragment.",
                );
                self.toasts.push(
                    now,
                    theme::GREY,
                    "Pas encore…",
                    "Il faut 100 M ✦ récoltés pour une Supernova",
                );
            }
            DomainEvent::OfflineGains { seconds, amount } => {
                self.title.offline = Some((seconds, amount));
                self.log(
                    Kind::Info,
                    format!(
                        "Pendant ton absence ({}) : +{} ✦",
                        fmt::duration(seconds),
                        fmt::short(amount)
                    ),
                );
            }
        }
    }

    fn generator_plural(&self, id: GeneratorId) -> &'static str {
        self.service
            .catalog()
            .generator(id)
            .map(|g| g.plural)
            .unwrap_or("?")
    }

    // ------------------------------------------------------------------
    // Rendu
    // ------------------------------------------------------------------

    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        self.render_buffer(frame.buffer_mut(), area);
    }

    pub fn render_buffer(&mut self, buf: &mut Buffer, area: Rect) {
        let now = self.now;
        if is_too_small(area) {
            too_small::render(buf, area, now);
            return;
        }
        let App {
            service,
            snap,
            screen,
            ui,
            vis,
            title,
            journal,
            toasts,
            spark_scratch,
            dt,
            ..
        } = self;
        let ctx = Ctx {
            snap,
            catalog: service.catalog(),
            now,
            dt: *dt,
            reduced: snap.reduced_effects,
        };
        match screen {
            Screen::Title => title.render(buf, area, now, &vis.stars, snap, ctx.reduced),
            Screen::Game => {
                let reveal = vis.nova.as_ref().map(|n| n.in_reveal(now)).unwrap_or(false);
                if reveal {
                    if let Some(n) = vis.nova.as_ref() {
                        n.render_reveal(buf, area, now, &vis.stars);
                    }
                    vis.particles.draw(buf, area);
                } else {
                    game::render(
                        buf,
                        area,
                        &ctx,
                        vis,
                        ui,
                        GameView {
                            journal,
                            toasts,
                            spark_scratch,
                        },
                    );
                    if let Some(n) = vis.nova.as_ref() {
                        n.render_over_game(buf, area, now, ctx.reduced);
                    }
                }
            }
        }
        if self.ui.debug {
            self.render_debug(buf, area);
        }
    }

    fn render_debug(&self, buf: &mut Buffer, area: Rect) {
        let rect = Rect::new(area.x + 1, area.y, 46.min(area.width.saturating_sub(2)), 4)
            .intersection(area);
        draw::clear(buf, rect);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(theme::fg(theme::GREEN))
            .title(" debug ");
        let inner = block.inner(rect);
        block.render(rect, buf);
        let s = &self.stats;
        let line1 = format!(
            "FPS {:>5.1} · particules {}/{}",
            s.fps,
            self.vis.particles.alive_count(),
            self.vis.particles.capacity()
        );
        let line2 = format!(
            "rendu {:.2} ms (max {:.2}) · frame {:.1} ms",
            s.render_ms, s.render_max_ms, s.frame_ms
        );
        draw::text(
            buf,
            inner.x,
            inner.y,
            &line1,
            theme::fg(theme::GREEN),
            inner.right(),
        );
        draw::text(
            buf,
            inner.x,
            inner.y + 1,
            &line2,
            theme::fg(theme::GREEN),
            inner.right(),
        );
    }
}

/// Lance le jeu dans le terminal. Sauvegarde à la sortie.
pub fn run(service: GameService, startup: Vec<GameEvent>) -> io::Result<RunOutcome> {
    let mut guard = TerminalGuard::new()?;
    let mut app = App::new(service, startup);
    let result = app.run_loop(guard.terminal());
    let save_error = app.service.save().err().map(|e| e.to_string());
    drop(guard);
    result.map(|_| RunOutcome { save_error })
}
