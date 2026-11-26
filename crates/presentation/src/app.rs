//! Boucle principale : entrée → `Command` → événements → effets/journal → rendu.

use std::io;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, BorderType, Borders, Widget};
use ratatui::Frame;

use application::updates::display_version;
use application::{Command, GameEvent, GameService, GameSnapshot, UpdateEvent, UpdateService};
use domain::catalog::balance::ACHIEVEMENT_BONUS;
use domain::prestige::fragment_multiplier;
use domain::{
    CometKind, CometReward, DomainEvent, Effect, GeneratorId, PurchaseTarget, Requirement,
    Restriction,
};

use crate::draw;
use crate::fmt;
use crate::fx::effects;
use crate::fx::Rgb;
use crate::input::{map_event, HoldDetector, Intent};
use crate::layout::is_too_small;
use crate::screens::bigbang::BangSequence;
use crate::screens::ending::{CreditLine, EndingSequence};
use crate::screens::game::{self, GameView};
use crate::screens::supernova::{NovaSequence, WAVE_AT};
use crate::screens::title::TitleScreen;
use crate::screens::too_small;
use crate::terminal::{Term, TerminalGuard};
use crate::theme;
use crate::theme::GEN_SLOTS;
use crate::ui::{Panel, Popup, Ui};
use crate::visuals::{Visuals, NEVER};
use crate::widgets::cosmos::{self, CosmosItem, PARTICLE};
use crate::widgets::journal::{Journal, Kind};
use crate::widgets::portals::PortalState;
use crate::widgets::shop::{self, ShopItem};
use crate::widgets::toast::Toasts;
use crate::widgets::{inner, Ctx};
use crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate};

/// ≈ 30 images par seconde.
/// Durée d'une image selon le réglage (30 images/s par défaut).
fn frame_duration(fps: u32) -> Duration {
    Duration::from_millis(1000 / u64::from(fps.clamp(10, 60)))
}

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
    /// Une mise à jour vient d'être installée : relancer le jeu.
    pub restart: bool,
}

/// Délai entre l'installation d'une mise à jour et le redémarrage automatique.
pub const RESTART_DELAY: f64 = 5.0;
/// Fondu au noir juste avant de redémarrer.
const RESTART_FADE: f64 = 0.6;

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
    /// Dernier message « interdit pendant le Défi » (évite d'en inonder l'écran).
    blocked_hint_at: f64,
    updates: UpdateService,
    /// Dernier état connu de la mise à jour automatique.
    update_status: Option<UpdateEvent>,
    /// Redémarrage prévu (mise à jour installée) : instant et version.
    restart_at: Option<(f64, String)>,
    /// Le jeu se ferme pour redémarrer sur la nouvelle version.
    restart: bool,
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
            blocked_hint_at: NEVER,
            updates: UpdateService::disabled(env!("CARGO_PKG_VERSION")),
            update_status: None,
            restart_at: None,
            restart: false,
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

    /// Branche le service de mise à jour automatique.
    pub fn set_updates(&mut self, updates: UpdateService) {
        self.updates = updates;
    }

    pub fn update_status(&self) -> Option<&UpdateEvent> {
        self.update_status.as_ref()
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

    /// Vrai si le jeu s'est fermé pour redémarrer sur une nouvelle version.
    pub fn should_restart(&self) -> bool {
        self.restart
    }

    /// Redémarrage automatique : attend la fin des grandes séquences
    /// (Supernova, portails, cérémonie) pour ne rien couper.
    fn update_restart(&mut self, now: f64) {
        let Some((at, _)) = &self.restart_at else {
            return;
        };
        let busy = self.vis.nova.is_some()
            || self.ui.portal.is_some()
            || self.vis.key_ceremony.is_some()
            || self.vis.bang.is_some()
            || self.vis.ending.is_some();
        if busy && now > *at - RESTART_FADE {
            // On repousse : le compte à rebours reprendra à la fin de la séquence.
            self.restart_at = self.restart_at.take().map(|(_, v)| (now + 2.0, v));
            return;
        }
        if now >= *at {
            self.restart = true;
            self.quit = true;
        }
    }

    /// Bandeau du redémarrage, puis fondu au noir.
    fn render_restart(&self, buf: &mut Buffer, area: Rect) {
        let Some((at, version)) = &self.restart_at else {
            return;
        };
        let left = (at - self.now).max(0.0);
        if left < RESTART_FADE {
            effects::fade(buf, area, 1.0 - left / RESTART_FADE);
        }
        let text = format!(
            " ✦ Mise à jour {version} installée · redémarrage dans {} s ✦ ",
            left.ceil() as u64
        );
        let short = format!(" ✦ {version} : redémarrage dans {} s ", left.ceil() as u64);
        let text = if text.chars().count() + 4 <= area.width as usize {
            text
        } else {
            short
        };
        let w = text.chars().count() as u16;
        let x = area.x + area.width.saturating_sub(w) / 2;
        let pulse = 0.5 + 0.5 * (self.now * 4.0).sin();
        let color = theme::GREEN
            .glow(0.3 * pulse)
            .dim(1.0 - (left / RESTART_FADE).min(1.0));
        draw::text(buf, x, area.y, &text, theme::bold(color), area.right());
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
            // Mise à jour synchronisée : le terminal affiche l'image d'un bloc
            // (ignoré par les terminaux qui ne la gèrent pas).
            crossterm::queue!(terminal.backend_mut(), BeginSynchronizedUpdate)?;
            terminal.draw(|f| {
                let t0 = Instant::now();
                self.render(f);
                render_time = t0.elapsed();
            })?;
            crossterm::execute!(terminal.backend_mut(), EndSynchronizedUpdate)?;
            self.stats.record(dt, render_time, frame_start.elapsed());

            let timeout =
                frame_duration(self.snap.frame_rate).saturating_sub(frame_start.elapsed());
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
            Screen::Title => {
                self.title.update(dt, now);
                // La fonderie continue de produire pendant l'écran titre.
                for ev in self.service.tick(real_dt) {
                    self.react(ev);
                }
            }
            Screen::Game => {
                for ev in self.service.tick(real_dt) {
                    self.react(ev);
                }
                if let Some(s) = self.ui.shop {
                    if s.is_closed(now) {
                        self.ui.shop = None;
                    }
                }
                if let Some(c) = self.ui.challenges {
                    if c.is_closed(now) {
                        self.ui.challenges = None;
                    }
                }
                if let Some(l) = self.ui.lab {
                    if l.is_closed(now) {
                        self.ui.lab = None;
                    }
                }
                if let Some(c) = self.ui.cosmos {
                    if c.is_closed(now) {
                        self.ui.cosmos = None;
                        self.ui.cosmos_armed = false;
                    }
                }
                if let Some(o) = self.ui.options {
                    if o.is_closed(now) {
                        self.ui.options = None;
                        self.ui.options_armed = false;
                    }
                }
                // Fin de l'effacement : nouvelle partie, retour à l'écran titre.
                if let Some(at) = self.ui.wipe_at {
                    if now - at >= crate::widgets::options::WIPE_DURATION {
                        self.ui.wipe_at = None;
                        self.command(Command::NewGame);
                    }
                }
                self.update_bang(now);
                // Fin de la plongée dans un portail : le Big Crunch commence.
                if let Some(p) = self.ui.portal {
                    if p.dive_progress(now) == Some(1.0) {
                        self.ui.portal = None;
                        self.start_crunch_into(domain::UniverseId(p.sel));
                    }
                }
                if let Some((start, _)) = self.vis.key_ceremony {
                    if now - start > crate::widgets::ceremony::DURATION {
                        self.vis.key_ceremony = None;
                    }
                }
                self.update_nova(now);
                if let Some(p) = self.ui.popup {
                    if p.is_closed(now) {
                        self.ui.popup = None;
                    }
                }
                if let Some(c) = self.ui.codex {
                    if c.is_closed(now) {
                        self.ui.codex = None;
                    }
                }
            }
        }
        self.service.fill_snapshot(&mut self.snap);
        let reduced = self.snap.reduced_effects;
        let sky = inner(self.ui.layout.sky);
        self.vis.update(dt, now, sky, reduced);
        self.toasts.update(now);
        self.vis.shooting_fast = self
            .service
            .catalog()
            .universe(self.snap.universe)
            .is_some_and(|u| u.key == "swarm");
        while let Some(ev) = self.updates.poll() {
            self.react_update(ev);
        }
        self.update_restart(now);
    }

    /// Big Bang : validé à l'éclair, puis retour au jeu en cascade.
    fn update_bang(&mut self, now: f64) {
        let Some(bang) = self.vis.bang.as_ref() else {
            return;
        };
        let t = bang.t(now);
        if !bang.committed && t >= crate::screens::bigbang::COMMIT_AT {
            if let Some(b) = self.vis.bang.as_mut() {
                b.committed = true;
            }
            self.command(Command::BigBang);
        }
        if self.vis.bang.as_ref().is_some_and(|b| b.is_done(now)) {
            self.vis.bang = None;
            self.vis.cascade_start = now;
            self.vis.particles.clear();
            self.vis.fireworks(now + 0.3);
        }
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
            let target = self
                .vis
                .nova
                .as_ref()
                .filter(|n| n.crunch)
                .map(|n| n.universe);
            self.command(match target {
                Some(universe) => Command::BigCrunch(universe),
                None => Command::Supernova,
            });
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
        if let Some(e) = self.vis.ending.as_ref() {
            if intent == Intent::Quit {
                self.quit = true;
            } else if e.can_close(now)
                && matches!(intent, Intent::Buy | Intent::Escape | Intent::Yes)
            {
                self.vis.ending = None;
                self.vis.cascade_start = now;
            }
            return;
        }
        if self.vis.bang.is_some() || self.ui.wipe_at.is_some() {
            if intent == Intent::Quit {
                self.quit = true;
            }
            return;
        }
        if let Some(popup) = self.ui.options {
            if popup.closing_at.is_none() {
                match intent {
                    Intent::Up => {
                        self.ui.options_sel = self.ui.options_sel.saturating_sub(1);
                        self.ui.options_armed = false;
                    }
                    Intent::Down => {
                        self.ui.options_sel =
                            (self.ui.options_sel + 1).min(crate::widgets::options::ITEMS - 1);
                        self.ui.options_armed = false;
                    }
                    Intent::Buy | Intent::Yes => match self.ui.options_sel {
                        0 => self.command(Command::CycleFrameRate),
                        1 => self.command(Command::ToggleReducedEffects),
                        _ if self.ui.options_armed => {
                            self.ui.options = None;
                            self.ui.options_armed = false;
                            self.ui.wipe_at = Some(now);
                        }
                        _ => self.ui.options_armed = true,
                    },
                    Intent::Options | Intent::Escape | Intent::No | Intent::Quit => {
                        if let Some(o) = self.ui.options.as_mut() {
                            o.closing_at = Some(now);
                        }
                    }
                    _ => {}
                }
            }
            return;
        }
        if self.vis.nova.is_some() {
            if intent == Intent::Quit {
                self.quit = true;
            }
            return;
        }
        if let Some(shop) = self.ui.shop {
            if shop.closing_at.is_none() {
                let count = shop::item_count(&self.snap);
                match intent {
                    Intent::Up => self.ui.shop_sel = self.ui.shop_sel.saturating_sub(1),
                    Intent::Down => {
                        self.ui.shop_sel = (self.ui.shop_sel + 1).min(count.saturating_sub(1))
                    }
                    Intent::Buy | Intent::Yes => {
                        match shop::item_at(&self.snap, self.ui.shop_sel) {
                            Some(ShopItem::Perk(i)) => {
                                if let Some(id) = self.snap.perks.get(i).map(|p| p.id) {
                                    self.command(Command::BuyPerk(id));
                                }
                            }
                            Some(ShopItem::Constellation(i)) => {
                                if let Some(id) = self.snap.constellations.get(i).map(|c| c.id) {
                                    self.command(Command::BuyConstellation(id));
                                }
                            }
                            None => {}
                        }
                    }
                    Intent::ToggleAuto => self.command(Command::ToggleAutoBuy),
                    Intent::Shop | Intent::Escape | Intent::No | Intent::Quit => {
                        if let Some(s) = self.ui.shop.as_mut() {
                            s.closing_at = Some(now);
                        }
                    }
                    _ => {}
                }
            }
            return;
        }
        if let Some(mut portal) = self.ui.portal {
            if portal.chosen_at.is_none() {
                let n = self.service.catalog().universes.len();
                match intent {
                    Intent::PrevPanel | Intent::Up => {
                        portal.select(now, portal.sel.saturating_sub(1));
                    }
                    Intent::NextPanel | Intent::Down => {
                        portal.select(now, (portal.sel + 1).min(n.saturating_sub(1)));
                    }
                    Intent::Buy | Intent::Yes if portal.crunch => portal.chosen_at = Some(now),
                    Intent::Escape
                    | Intent::No
                    | Intent::Quit
                    | Intent::Universes
                    | Intent::BigCrunch => {
                        self.ui.portal = None;
                        return;
                    }
                    _ => {}
                }
                self.ui.portal = Some(portal);
            }
            return;
        }
        if let Some(popup) = self.ui.cosmos {
            if popup.closing_at.is_none() {
                let count = self.cosmos_count();
                match intent {
                    Intent::Up => {
                        self.ui.cosmos_sel = self.ui.cosmos_sel.saturating_sub(1);
                        self.ui.cosmos_armed = false;
                    }
                    Intent::Down => {
                        self.ui.cosmos_sel = (self.ui.cosmos_sel + 1).min(count.saturating_sub(1));
                        self.ui.cosmos_armed = false;
                    }
                    Intent::Buy | Intent::Yes => self.cosmos_action(),
                    Intent::Cosmos | Intent::Escape | Intent::No | Intent::Quit => {
                        if let Some(c) = self.ui.cosmos.as_mut() {
                            c.closing_at = Some(now);
                        }
                    }
                    _ => {}
                }
            }
            return;
        }
        if let Some(popup) = self.ui.lab {
            if popup.closing_at.is_none() {
                let count = self.snap.lab.len();
                match intent {
                    Intent::Up => self.ui.lab_sel = self.ui.lab_sel.saturating_sub(1),
                    Intent::Down => {
                        self.ui.lab_sel = (self.ui.lab_sel + 1).min(count.saturating_sub(1))
                    }
                    Intent::Buy | Intent::Yes => self.lab_action(),
                    Intent::Lab | Intent::Escape | Intent::No | Intent::Quit => {
                        if let Some(l) = self.ui.lab.as_mut() {
                            l.closing_at = Some(now);
                        }
                    }
                    _ => {}
                }
            }
            return;
        }
        if let Some(popup) = self.ui.challenges {
            if popup.closing_at.is_none() {
                let count = self.snap.challenges.len();
                match intent {
                    Intent::Up => {
                        self.ui.challenges_sel = self.ui.challenges_sel.saturating_sub(1);
                        self.ui.challenge_armed = false;
                    }
                    Intent::Down => {
                        self.ui.challenges_sel =
                            (self.ui.challenges_sel + 1).min(count.saturating_sub(1));
                        self.ui.challenge_armed = false;
                    }
                    Intent::Buy | Intent::Yes => self.challenge_action(),
                    Intent::Challenges | Intent::Escape | Intent::No | Intent::Quit => {
                        self.ui.challenge_armed = false;
                        if let Some(c) = self.ui.challenges.as_mut() {
                            c.closing_at = Some(now);
                        }
                    }
                    _ => {}
                }
            }
            return;
        }
        if let Some(codex) = self.ui.codex {
            if codex.closing_at.is_none() {
                match intent {
                    Intent::Up => self.ui.codex_scroll = self.ui.codex_scroll.saturating_sub(1),
                    Intent::Down => self.ui.codex_scroll += 1,
                    Intent::Info | Intent::Escape | Intent::No | Intent::Quit => {
                        if let Some(c) = self.ui.codex.as_mut() {
                            c.closing_at = Some(now);
                        }
                    }
                    _ => {}
                }
            }
            return;
        }
        if let Some(popup) = self.ui.popup {
            if popup.closing_at.is_none() {
                match intent {
                    Intent::Yes => {
                        self.close_popup();
                        if self.ui.popup_crunch {
                            self.start_crunch();
                        } else {
                            self.start_supernova();
                        }
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
                    self.ui.popup_crunch = false;
                    self.ui.popup = Some(Popup {
                        opened_at: now,
                        closing_at: None,
                    });
                } else {
                    self.command(Command::Supernova);
                }
            }
            Intent::Save => self.command(Command::Save),
            Intent::Options => {
                self.ui.options = Some(Popup {
                    opened_at: now,
                    closing_at: None,
                });
                self.ui.options_sel = 0;
                self.ui.options_armed = false;
            }
            Intent::Cosmos => {
                if self.snap.lab_unlocked {
                    self.ui.cosmos = Some(Popup {
                        opened_at: now,
                        closing_at: None,
                    });
                    self.ui.cosmos_armed = false;
                } else {
                    let need = self.snap.bigbang_needed;
                    self.toasts.push(
                        now,
                        theme::GREY,
                        "Cosmogonie",
                        format!(
                            "S'ouvre au 10e Big Crunch ({}/10) · Big Bang au {need}e",
                            self.snap.crunches
                        ),
                    );
                }
            }
            Intent::Lab => {
                if self.snap.lab_unlocked {
                    self.ui.lab = Some(Popup {
                        opened_at: now,
                        closing_at: None,
                    });
                } else {
                    let (have, need) = (self.snap.crunches, self.snap.lab_needed);
                    self.toasts.push(
                        now,
                        theme::GREY,
                        "Laboratoire",
                        format!("S'ouvre au {need}e Big Crunch ({have}/{need})"),
                    );
                }
            }
            Intent::Universes => {
                if self.snap.universes_unlocked {
                    let current = self.snap.universe.index();
                    self.ui.portal = Some(PortalState::new(now, current, false));
                } else {
                    self.toasts.push(
                        now,
                        theme::GREY,
                        "Univers parallèles",
                        format!("S'ouvrent au 10e Big Crunch ({}/10)", self.snap.crunches),
                    );
                }
            }
            Intent::BigCrunch if self.snap.crunch_available && self.snap.universes_unlocked => {
                // Le choix de l'univers tient lieu de confirmation.
                let current = self.snap.universe.index();
                self.ui.portal = Some(PortalState::new(now, current, true));
            }
            Intent::BigCrunch => {
                if self.snap.crunch_available {
                    self.ui.popup_crunch = true;
                    self.ui.popup = Some(Popup {
                        opened_at: now,
                        closing_at: None,
                    });
                } else {
                    self.command(Command::BigCrunch(domain::UniverseId::ORIGIN));
                }
            }
            Intent::Challenges => {
                self.ui.challenges = Some(Popup {
                    opened_at: now,
                    closing_at: None,
                });
                self.ui.challenge_armed = false;
                self.ui.challenges_sel = self
                    .snap
                    .challenges
                    .iter()
                    .position(|c| c.active || (c.unlocked && !c.done))
                    .unwrap_or(0);
            }
            Intent::Info => {
                self.ui.codex = Some(Popup {
                    opened_at: now,
                    closing_at: None,
                });
                self.ui.codex_scroll = 0;
            }
            Intent::ToggleEffects => self.command(Command::ToggleReducedEffects),
            Intent::BuyAll => self.command(Command::BuyAllUpgrades),
            Intent::ToggleAuto => self.command(Command::ToggleAutoBuy),
            Intent::Shop => {
                self.ui.shop = Some(Popup {
                    opened_at: now,
                    closing_at: None,
                });
                self.ui.shop_sel = self.snap.perks.iter().position(|p| !p.owned).unwrap_or(0);
            }
            Intent::JournalUp => {
                let max = self.journal.len().saturating_sub(1);
                self.ui.journal_scroll = (self.ui.journal_scroll + 2).min(max);
            }
            Intent::JournalDown => {
                self.ui.journal_scroll = self.ui.journal_scroll.saturating_sub(2);
            }
            Intent::Quit | Intent::Escape => self.quit = true,
            Intent::Yes | Intent::No | Intent::Other | Intent::ToggleDebug => {}
        }
    }

    fn close_popup(&mut self) {
        if let Some(p) = self.ui.popup.as_mut() {
            p.closing_at = Some(self.now);
        }
    }

    /// Entrée dans la fenêtre des Défis : 1er appui = arme, 2e = lance ou abandonne.
    fn challenge_action(&mut self) {
        let Some(v) = self.snap.challenges.get(self.ui.challenges_sel).copied() else {
            return;
        };
        let possible = v.active || (v.unlocked && !v.done && self.snap.active_challenge.is_none());
        if !possible {
            self.ui.challenge_armed = false;
            return;
        }
        if !self.ui.challenge_armed {
            self.ui.challenge_armed = true;
            return;
        }
        self.ui.challenge_armed = false;
        if v.active {
            self.command(Command::AbandonChallenge);
        } else {
            self.command(Command::StartChallenge(v.id));
            if let Some(c) = self.ui.challenges.as_mut() {
                c.closing_at = Some(self.now);
            }
            self.vis.cascade_start = self.now;
        }
    }

    fn challenge_name(&self, id: domain::ChallengeId) -> &'static str {
        self.service
            .catalog()
            .challenge(id)
            .map(|c| c.name)
            .unwrap_or("?")
    }

    /// Entrée dans le Laboratoire : lance la recherche, ou met en pause / relance.
    fn lab_action(&mut self) {
        let item = {
            let ctx = Ctx {
                snap: &self.snap,
                catalog: self.service.catalog(),
                now: self.now,
                dt: self.dt,
                reduced: self.snap.reduced_effects,
            };
            crate::widgets::lab::item_at(&ctx, self.ui.lab_sel)
        };
        let Some(v) = item else { return };
        let toggleable = self
            .service
            .catalog()
            .lab_node(v.id)
            .is_some_and(|d| d.effect.is_toggleable());
        if v.done && toggleable {
            self.command(Command::ToggleLab(v.id));
        } else if !v.done {
            self.command(Command::StartResearch(v.id));
        }
    }

    fn cosmos_ctx_item(&self) -> (usize, Option<CosmosItem>) {
        let ctx = Ctx {
            snap: &self.snap,
            catalog: self.service.catalog(),
            now: self.now,
            dt: self.dt,
            reduced: self.snap.reduced_effects,
        };
        (
            cosmos::item_count(&ctx),
            cosmos::item_at(&ctx, self.ui.cosmos_sel),
        )
    }

    fn cosmos_count(&self) -> usize {
        self.cosmos_ctx_item().0
    }

    /// Entrée dans la Cosmogonie : Big Bang (confirmé), forge, construction.
    fn cosmos_action(&mut self) {
        let now = self.now;
        match self.cosmos_ctx_item().1 {
            Some(CosmosItem::BigBang) => {
                if !self.snap.bigbang_available {
                    self.command(Command::BigBang);
                } else if self.ui.cosmos_armed {
                    self.ui.cosmos_armed = false;
                    self.ui.cosmos = None;
                    self.ui.shop = None;
                    let gained = self.snap.particles_available;
                    let total = self.snap.particles + gained;
                    let prod = 1.0 + domain::catalog::balance::PARTICLE_PRODUCTION * total as f64;
                    self.vis.bang = Some(BangSequence::new(now, gained, total, prod));
                } else {
                    self.ui.cosmos_armed = true;
                }
            }
            Some(CosmosItem::Forge(i)) => {
                if let Some(f) = self.snap.forges.get(i).copied() {
                    self.command(Command::Forge(f.id));
                }
            }
            Some(CosmosItem::Stage(i)) => {
                if i == self.snap.attractor_stages {
                    self.command(Command::BuildAttractor);
                } else {
                    self.command_failed_stage(i);
                }
            }
            None => {}
        }
    }

    fn command_failed_stage(&mut self, i: usize) {
        let body = if i < self.snap.attractor_stages {
            "Étape déjà achevée"
        } else {
            "Construis d'abord les étapes précédentes"
        };
        self.toasts
            .push(self.now, theme::GREY, "Grand Attracteur", body);
    }

    /// Générique de fin : quelques chiffres de la partie.
    fn credits(&self) -> Vec<CreditLine> {
        let s = &self.snap;
        let keys = s.universes.iter().filter(|u| u.key).count();
        let gold = theme::GOLD;
        let text = theme::TEXT;
        let muted = theme::MUTED;
        let line = |t: &str, c, b| (t.to_string(), c, b);
        vec![
            line("✶  LE GRAND ATTRACTEUR S'ÉVEILLE  ✶", gold, true),
            line("", text, false),
            line(
                "Toute la poussière du cosmos converge vers ta fonderie.",
                text,
                false,
            ),
            line("", text, false),
            line("FIN DE L'ACTE II", theme::PINK, true),
            line("", text, false),
            (
                format!("Temps de jeu : {}", fmt::duration(s.play_time)),
                muted,
                false,
            ),
            (
                format!(
                    "Big Bang : {} · particules {PARTICLE} : {}",
                    s.big_bangs, s.particles
                ),
                muted,
                false,
            ),
            (
                format!(
                    "Clés d'univers : {keys}/5 · succès : {}/{}",
                    s.achievements_unlocked, s.achievements_total
                ),
                muted,
                false,
            ),
            line("", text, false),
            line("FONDERIE STELLAIRE", theme::CYAN, true),
            line("Conçu et forgé par Ajustor", text, false),
            line("", text, false),
            line("Merci d'avoir joué !", gold, true),
            line("", text, false),
            line(
                "Le jeu continue : ta fonderie produit désormais ×10.",
                muted,
                false,
            ),
        ]
    }

    fn lab_name(&self, id: domain::LabId) -> &'static str {
        self.service
            .catalog()
            .lab_node(id)
            .map(|l| l.name)
            .unwrap_or("?")
    }

    fn start_crunch(&mut self) {
        self.start_crunch_into(domain::UniverseId::ORIGIN);
    }

    /// Séquence du Big Crunch vers un univers ; validée au milieu de l'animation.
    fn start_crunch_into(&mut self, universe: domain::UniverseId) {
        let crunches = self.snap.crunches + 1;
        let before = self.snap.crunch_production;
        let after = 1.0 + domain::catalog::balance::CRUNCH_PRODUCTION * crunches as f64;
        let name = if self.snap.universes_unlocked {
            self.service
                .catalog()
                .universe(universe)
                .map(|u| u.name)
                .unwrap_or("")
        } else {
            ""
        };
        self.vis.nova = Some(NovaSequence::crunch_into(
            self.now, crunches, before, after, universe, name,
        ));
    }

    fn start_supernova(&mut self) {
        let gained = self.snap.fragments_available;
        let old_bonus = self.snap.fragment_bonus;
        let new_bonus = fragment_multiplier(self.snap.fragments + gained) - 1.0;
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
                // La sauvegarde automatique (toutes les 30 s) reste silencieuse :
                // elle noierait le journal. Seule la sauvegarde demandée est annoncée.
                if !automatic {
                    self.log(Kind::Save, "Partie sauvegardée");
                }
            }
            GameEvent::SaveFailed(msg) => {
                self.log(Kind::Error, format!("Échec de la sauvegarde : {msg}"));
                self.toasts
                    .push(now, theme::RED, "Sauvegarde impossible", msg);
            }
            GameEvent::NewGame => {
                // Tout repart de zéro : écran titre, journal et effets neufs.
                let now = self.now;
                self.journal = Journal::default();
                self.journal.push(
                    0.0,
                    now,
                    Kind::Info,
                    "✦ Nouvelle partie : bienvenue dans la Fonderie Stellaire !",
                );
                self.vis.particles.clear();
                self.vis.floaters.clear();
                self.ui = crate::ui::Ui::default();
                self.title = TitleScreen::new(now, seed() ^ 0x7177);
                self.screen = Screen::Title;
            }
            GameEvent::FrameRateChanged(fps) => {
                self.toasts.push(
                    self.now,
                    theme::CYAN,
                    "Options",
                    format!("{fps} images par seconde"),
                );
            }
            GameEvent::SaveWiped => {
                self.title.wiped = true;
                self.log(
                    Kind::Error,
                    "Nouvel équilibrage : l'ancienne partie a été effacée, tout le monde repart de zéro.",
                );
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
                    // Ciel replié : les étincelles partent du compteur.
                    let h = self.ui.layout.header;
                    (h.x as f64 + h.width as f64 / 3.0, h.y as f64 + 2.0)
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
                let gi = generator.index().min(GEN_SLOTS - 1);
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
                PurchaseTarget::Generator(g) => {
                    self.vis.gen_fail[g.index().min(GEN_SLOTS - 1)] = now
                }
                PurchaseTarget::Forge(g) => {
                    let name = self
                        .service
                        .catalog()
                        .generator(g)
                        .map(|d| d.name)
                        .unwrap_or("?");
                    self.toasts.push(
                        now,
                        theme::GREY,
                        "Forge primordiale",
                        format!("« {name} » : pas assez de particules {PARTICLE}"),
                    );
                }
                PurchaseTarget::Attractor(_) => {
                    let body = if self.snap.big_bangs == 0 {
                        "Le chantier s'ouvre après ton premier Big Bang".to_string()
                    } else if self.snap.attractor_build.is_some() {
                        "Une étape à la fois".to_string()
                    } else {
                        format!("Pas assez de particules {PARTICLE}")
                    };
                    self.toasts.push(now, theme::GREY, "Grand Attracteur", body);
                }
                PurchaseTarget::Lab(l) => {
                    let name = self.lab_name(l);
                    let body = if self.snap.research.is_some() {
                        "Une recherche à la fois".to_string()
                    } else {
                        format!("« {name} » n'est pas disponible")
                    };
                    self.toasts.push(now, theme::GREY, "Laboratoire", body);
                }
                PurchaseTarget::Constellation(c) => {
                    let name = self
                        .service
                        .catalog()
                        .constellation(c)
                        .map(|d| d.name)
                        .unwrap_or("?");
                    let view = self.snap.constellations.get(c.index()).copied();
                    let body = match view {
                        Some(v) if v.maxed => format!("« {name} » est au niveau maximal"),
                        Some(v) => format!(
                            "Niveau suivant de « {name} » : {} fragment(s)",
                            fmt::short(v.cost)
                        ),
                        None => String::new(),
                    };
                    self.toasts.push(now, theme::GREY, "Constellations", body);
                }
                PurchaseTarget::Perk(p) => {
                    let (name, cost) = self
                        .service
                        .catalog()
                        .perk(p)
                        .map(|d| (d.name, d.cost))
                        .unwrap_or(("?", 0.0));
                    let owned = self.snap.perks.get(p.index()).is_some_and(|v| v.owned);
                    let body = if owned {
                        format!("« {name} » est déjà acquise")
                    } else {
                        format!("« {name} » coûte {} fragment(s)", fmt::short(cost))
                    };
                    self.toasts.push(now, theme::GREY, "Atelier", body);
                }
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
                self.vis.gen_reveal[generator.index().min(GEN_SLOTS - 1)] = now;
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
            DomainEvent::CometSpawned { kind } => {
                let h = sky.height.max(4) as f64;
                self.vis.comet.y0 = self.vis.rng.range(0.45, 0.8) * h;
                self.vis.comet.amp = -self.vis.rng.range(0.15, 0.35) * h;
                self.vis.comet.trail_acc = 0.0;
                match kind {
                    CometKind::Golden => {
                        self.log(Kind::Achievement, "● Une COMÈTE DORÉE passe ! [C] vite !");
                        self.toasts.push(
                            now,
                            theme::GOLD,
                            "● Comète dorée !",
                            "Capture-la [C] : production ×77",
                        );
                    }
                    CometKind::Black => {
                        self.log(
                            Kind::Error,
                            "● Une COMÈTE NOIRE approche… [C] ou elle te vole !",
                        );
                        self.toasts.push(
                            now,
                            theme::RED,
                            "● Comète noire !",
                            "Capture-la [C], sinon elle vole 10 % de ta poussière",
                        );
                    }
                    CometKind::Normal => {
                        self.log(Kind::Comet, "● Une comète approche ! [C] pour la capturer");
                    }
                }
            }
            DomainEvent::CometStole { amount } => {
                let a = fmt::short(amount);
                self.log(Kind::Error, format!("● La comète noire a volé {a} ✦"));
                self.toasts.push(
                    now,
                    theme::RED,
                    "● Volé !",
                    format!("La comète noire emporte {a} ✦"),
                );
                self.vis.roll_counter(now);
                self.vis.sky_flash = now;
                self.vis.sky_flash_color = theme::RED.dim(0.3);
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
                    CometReward::GoldenFrenzy {
                        multiplier,
                        duration,
                    } => {
                        self.toasts.push(
                            now,
                            theme::GOLD,
                            "● Frénésie dorée !",
                            format!("Production ×{multiplier} pendant {duration} s"),
                        );
                        self.log(Kind::Achievement, "● Comète dorée capturée : frénésie !");
                        self.vis.fireworks(now);
                        self.vis.bump_counter(now);
                    }
                    CometReward::DarkShard { fragments } => {
                        let f = fmt::short(fragments);
                        self.toasts.push(
                            now,
                            theme::PINK,
                            "● Éclat d'ombre",
                            format!("+{f} fragment permanent"),
                        );
                        self.log(
                            Kind::Achievement,
                            format!("● Comète noire capturée : +{f} fragment"),
                        );
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
                self.vis.bar_fill = [0.0; GEN_SLOTS];
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
            DomainEvent::BulkUpgrades { count } => {
                if count == 0 {
                    self.toasts.push(
                        now,
                        theme::GREY,
                        "Rien à acheter",
                        "Aucune amélioration abordable pour l'instant",
                    );
                } else {
                    let s = if count > 1 { "s" } else { "" };
                    self.log(
                        Kind::Purchase,
                        format!("Tout acheter : {count} amélioration{s}"),
                    );
                    self.vis.fireworks(now);
                }
            }
            DomainEvent::PerkPurchased { perk } => {
                let (name, description) = self
                    .service
                    .catalog()
                    .perk(perk)
                    .map(|d| (d.name, d.description))
                    .unwrap_or(("?", ""));
                self.toasts
                    .push(now, theme::GOLD, format!("◆ Atelier : {name}"), description);
                self.log(Kind::Achievement, format!("◆ Atelier : {name} acquise"));
                self.vis.fireworks(now);
            }
            DomainEvent::AutoBuyChanged { enabled } => {
                let msg = if enabled {
                    "Automate activé : il achète chaque seconde le meilleur générateur"
                } else {
                    "Automate arrêté"
                };
                self.log(Kind::Info, msg);
            }
            DomainEvent::ConstellationUpgraded {
                constellation,
                level,
            } => {
                let def = self.service.catalog().constellation(constellation);
                let name = def.map(|d| d.name).unwrap_or("?");
                let total = def
                    .map(|d| shop::constellation_total(d.effect, level))
                    .unwrap_or_default();
                self.toasts.push(
                    now,
                    theme::GOLD,
                    format!("★ {name} · niveau {level}"),
                    total.clone(),
                );
                self.log(
                    Kind::Achievement,
                    format!("★ Constellation {name} niveau {level} : {total}"),
                );
                self.vis.constellation_pop = now;
            }
            DomainEvent::BigCrunch { crunches } => {
                let prod =
                    fmt::short(1.0 + domain::catalog::balance::CRUNCH_PRODUCTION * crunches as f64);
                self.log(
                    Kind::Supernova,
                    format!("✶ BIG CRUNCH n°{crunches} : production ×{prod} pour toujours"),
                );
                self.toasts.push(
                    now,
                    theme::PINK,
                    format!("✶ Big Crunch n°{crunches}"),
                    format!("Matière noire : production ×{prod}"),
                );
                self.ui.gen_sel = 0;
                self.ui.upg_sel = 0;
                self.ui.gen_scroll = 0;
                self.ui.upg_scroll = 0;
                self.vis.bar_fill = [0.0; GEN_SLOTS];
                self.vis.roll_counter(now);
            }
            DomainEvent::LabResearchStarted { node } => {
                let name = self.lab_name(node);
                let secs = self
                    .service
                    .catalog()
                    .lab_node(node)
                    .map(|l| l.research_secs)
                    .unwrap_or(0.0);
                self.log(
                    Kind::Info,
                    format!("✶ Recherche lancée : {name} ({})", fmt::duration(secs)),
                );
                self.toasts.push(
                    now,
                    Rgb(170, 90, 255),
                    format!("✶ Recherche : {name}"),
                    format!("Terminée dans {} (même jeu fermé)", fmt::duration(secs)),
                );
            }
            DomainEvent::LabResearchDone { node } => {
                let name = self.lab_name(node);
                self.log(Kind::Achievement, format!("✶ Recherche terminée : {name}"));
                self.toasts.push(
                    now,
                    theme::GOLD,
                    format!("✶ {name} : terminé !"),
                    "Nouvelle recherche possible au Laboratoire [L]",
                );
                self.vis.fireworks(now);
            }
            DomainEvent::LabToggled { node, enabled } => {
                let name = self.lab_name(node);
                let state = if enabled { "relancé" } else { "en pause" };
                self.log(Kind::Info, format!("{name} : {state}"));
            }
            DomainEvent::UniverseEntered { universe } => {
                if self.snap.universes_unlocked || universe != domain::UniverseId::ORIGIN {
                    if let Some(def) = self.service.catalog().universe(universe) {
                        let color = theme::universe_color(universe.index());
                        self.toasts.push(
                            now,
                            color,
                            format!("{} Univers : {}", def.symbol, def.name),
                            def.rules_text,
                        );
                        self.log(
                            Kind::Supernova,
                            format!("{} Univers : {} ({})", def.symbol, def.name, def.rules_text),
                        );
                    }
                }
            }
            DomainEvent::UniverseKeyEarned { universe } => {
                let (name, reward) = self
                    .service
                    .catalog()
                    .universe(universe)
                    .map(|u| (u.key_name, u.reward_text))
                    .unwrap_or(("?", ""));
                self.log(Kind::Achievement, format!("✶ {name} obtenue ! {reward}"));
                self.vis.key_ceremony = Some((now, universe));
                // Gerbe aux couleurs de l'univers au centre de l'écran.
                let area = self
                    .ui
                    .layout
                    .panels()
                    .iter()
                    .fold(Rect::default(), |a, r| a.union(*r));
                let palette = theme::universe_palette(universe.index());
                self.vis.explode(
                    area.x as f64 + area.width as f64 / 2.0,
                    area.y as f64 + area.height as f64 / 2.0 - 2.0,
                    120,
                    45.0,
                    &[palette[0], palette[1], theme::WHITE, theme::GOLD],
                );
                self.vis.fireworks(now + 0.4);
                self.vis.fireworks(now + 1.2);
            }
            DomainEvent::BigBang {
                particles_gained,
                total_particles,
            } => {
                self.log(
                    Kind::Supernova,
                    format!(
                        "{PARTICLE} BIG BANG : +{particles_gained} particule(s) primordiale(s), {total_particles} au total"
                    ),
                );
            }
            DomainEvent::BigBangUnavailable { crunches, needed } => {
                self.toasts.push(
                    now,
                    theme::GREY,
                    "Big Bang",
                    format!(
                        "Il faut {needed} Big Crunch ({crunches}/{needed}), sans Défi en cours"
                    ),
                );
            }
            DomainEvent::GeneratorForged { generator } => {
                let (name, symbol) = self
                    .service
                    .catalog()
                    .generator(generator)
                    .map(|d| (d.name, d.symbol))
                    .unwrap_or(("?", '?'));
                let color = theme::generator_color(generator.index());
                self.log(
                    Kind::Achievement,
                    format!("{symbol} {name} forgé : il rejoint ta fonderie pour toujours"),
                );
                self.toasts.push(
                    now,
                    color,
                    format!("{symbol} {name} forgé !"),
                    "Présent dans tous les cycles, dès qu'il est abordable",
                );
                // Gerbe d'étincelles « de forge » au-dessus des générateurs.
                let g = self.ui.layout.generators;
                self.vis.explode(
                    g.x as f64 + g.width as f64 / 2.0,
                    g.y as f64 + g.height as f64 / 2.0,
                    110,
                    40.0,
                    &[color, theme::WHITE, theme::GOLD, theme::ORANGE],
                );
                self.vis.sky_flash = now;
                self.vis.sky_flash_color = color;
            }
            DomainEvent::AttractorStageStarted { stage } => {
                let (name, secs) = self
                    .service
                    .catalog()
                    .attractor_stage(stage)
                    .map(|s| (s.name, s.build_secs))
                    .unwrap_or(("?", 0.0));
                self.log(
                    Kind::Purchase,
                    format!("◇ Grand Attracteur : chantier « {name} » lancé"),
                );
                self.toasts.push(
                    now,
                    crate::fx::Rgb(190, 110, 255),
                    format!("◇ Chantier : {name}"),
                    format!("Achevé dans {} (même jeu fermé)", fmt::duration(secs)),
                );
            }
            DomainEvent::AttractorStageDone { stage } => {
                let (name, desc) = self
                    .service
                    .catalog()
                    .attractor_stage(stage)
                    .map(|s| (s.name, s.description))
                    .unwrap_or(("?", ""));
                self.log(
                    Kind::Achievement,
                    format!("◆ Grand Attracteur : « {name} » achevé ({desc})"),
                );
                self.toasts.push(
                    now,
                    theme::GOLD,
                    format!("◆ {name} : achevé !"),
                    desc.to_string(),
                );
                self.vis.attractor_pop = Some((now, stage));
                self.vis.fireworks(now);
                self.vis.fireworks(now + 0.6);
            }
            DomainEvent::AttractorAwakened => {
                self.log(
                    Kind::Achievement,
                    "✶ Le Grand Attracteur s'éveille : fin de l'Acte II !",
                );
                self.ui.cosmos = None;
                // Chiffres à jour (succès « Fin de l'Acte II » compris).
                self.service.fill_snapshot(&mut self.snap);
                let credits = self.credits();
                self.vis.ending = Some(EndingSequence::new(now + 1.5, credits));
            }
            DomainEvent::BigCrunchUnavailable { supernovas, needed } => {
                self.toasts.push(
                    now,
                    theme::GREY,
                    "Big Crunch",
                    format!(
                        "Il faut {needed} Supernovas ({supernovas}/{needed}), sans Défi en cours"
                    ),
                );
            }
            DomainEvent::ChallengeStarted { challenge } => {
                let name = self.challenge_name(challenge);
                let rule = self
                    .service
                    .catalog()
                    .challenge(challenge)
                    .map(|c| c.rule)
                    .unwrap_or("");
                self.toasts
                    .push(now, theme::PINK, format!("◆ Défi : {name}"), rule);
                self.log(Kind::Supernova, format!("◆ Défi lancé : {name} ({rule})"));
                self.ui.gen_sel = 0;
                self.ui.upg_sel = 0;
                self.vis.bar_fill = [0.0; GEN_SLOTS];
            }
            DomainEvent::ChallengeCompleted { challenge } => {
                let name = self.challenge_name(challenge);
                let reward = self
                    .service
                    .catalog()
                    .challenge(challenge)
                    .map(|c| c.reward_text)
                    .unwrap_or("");
                self.toasts
                    .push(now, theme::GOLD, format!("★ Défi réussi : {name}"), reward);
                self.log(
                    Kind::Achievement,
                    format!("★ Défi réussi : {name} · {reward}"),
                );
                self.vis.fireworks(now);
                self.vis.fireworks(now + 0.6);
            }
            DomainEvent::ChallengeFailed { challenge } => {
                let name = self.challenge_name(challenge);
                self.toasts.push(
                    now,
                    theme::RED,
                    "Défi échoué",
                    format!("{name} : temps écoulé"),
                );
                self.log(Kind::Error, format!("Défi échoué : {name} (temps écoulé)"));
            }
            DomainEvent::ChallengeAbandoned { challenge } => {
                let name = self.challenge_name(challenge);
                self.log(Kind::Info, format!("Défi abandonné : {name}"));
            }
            DomainEvent::ChallengeBlocked { restriction } => {
                if now - self.blocked_hint_at > 3.0 {
                    self.blocked_hint_at = now;
                    let why = match restriction {
                        Restriction::NoClick => "Pas de récolte manuelle pendant ce Défi",
                        Restriction::NoUpgrades => "Pas d'amélioration pendant ce Défi",
                        _ => "Interdit pendant ce Défi",
                    };
                    self.toasts.push(now, theme::GREY, "◆ Défi en cours", why);
                }
            }
            DomainEvent::ChallengeUnavailable { challenge } => {
                let name = self.challenge_name(challenge);
                self.toasts.push(
                    now,
                    theme::GREY,
                    "Défis",
                    format!("« {name} » n'est pas disponible"),
                );
            }
            DomainEvent::AutoBuyLocked => {
                self.toasts.push(
                    now,
                    theme::GREY,
                    "Automate",
                    "Achète-le à l'Atelier stellaire [E] avec des fragments",
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
        let save_path = self.service.save_location();
        let version = display_version(self.updates.current_version());
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
                let bang_full = vis.bang.as_ref().is_some_and(|b| b.is_full(now));
                if bang_full {
                    if let Some(b) = vis.bang.as_ref() {
                        b.render_full(buf, area, now, &vis.stars);
                    }
                } else if reveal {
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
                            save_path: &save_path,
                            version: &version,
                            journal,
                            toasts,
                            spark_scratch,
                        },
                    );
                    if let Some(n) = vis.nova.as_ref() {
                        n.render_over_game(buf, area, now, ctx.reduced);
                    }
                    if let Some(b) = vis.bang.as_ref() {
                        b.render_collapse(buf, area, now, ctx.reduced);
                    }
                }
            }
        }
        if self.screen == Screen::Game {
            if let Some(e) = self.vis.ending.as_ref() {
                if e.t(now) >= 0.0 {
                    e.render(buf, area, now);
                }
            }
        }
        if self.screen == Screen::Title {
            self.render_update_status(buf, area);
        }
        self.render_restart(buf, area);
        if self.ui.debug {
            self.render_debug(buf, area);
        }
    }

    /// Réaction aux événements de la mise à jour automatique.
    pub fn react_update(&mut self, ev: UpdateEvent) {
        let now = self.now;
        match &ev {
            UpdateEvent::UpToDate => {}
            UpdateEvent::Downloading { version } => {
                self.log(
                    Kind::Info,
                    format!("Nouvelle version {version} : téléchargement…"),
                );
            }
            UpdateEvent::Installed { version } => {
                self.log(
                    Kind::Purchase,
                    format!("Mise à jour {version} installée : le jeu redémarre tout seul"),
                );
                self.toasts.push(
                    now,
                    theme::GREEN,
                    format!("✦ Mise à jour {version} installée"),
                    "Le jeu redémarre tout seul dans quelques secondes",
                );
                self.restart_at = Some((now + RESTART_DELAY, version.clone()));
            }
            UpdateEvent::ManualOnly {
                version,
                reason,
                url,
            } => {
                self.log(
                    Kind::Info,
                    format!("Version {version} disponible ({reason}) : {url}"),
                );
                self.toasts.push(
                    now,
                    theme::CYAN,
                    format!("Version {version} disponible"),
                    "Télécharge-la sur la page GitHub du jeu",
                );
            }
            UpdateEvent::Failed(msg) => {
                self.log(Kind::Save, format!("Mises à jour : {msg}"));
            }
        }
        self.update_status = Some(ev);
    }

    /// Version et état de la mise à jour, en bas à droite de l'écran titre.
    fn render_update_status(&self, buf: &mut Buffer, area: Rect) {
        let version = display_version(self.updates.current_version());
        let (status, color) = match &self.update_status {
            None if self.updates.is_enabled() => {
                ("recherche de mises à jour…".to_string(), theme::GREY)
            }
            None => (String::new(), theme::GREY),
            Some(UpdateEvent::UpToDate) => ("à jour".to_string(), theme::GREY),
            Some(UpdateEvent::Downloading { version }) => {
                (format!("téléchargement de {version}…"), theme::CYAN)
            }
            Some(UpdateEvent::Installed { version }) => {
                (format!("{version} installée, redémarrage…"), theme::GREEN)
            }
            Some(UpdateEvent::ManualOnly { version, .. }) => {
                (format!("{version} disponible sur GitHub"), theme::CYAN)
            }
            Some(UpdateEvent::Failed(_)) => (String::new(), theme::GREY),
        };
        let text = if status.is_empty() {
            format!("{version} ")
        } else {
            format!("{version} · {status} ")
        };
        let y = area.bottom().saturating_sub(1);
        let w = text.chars().count() as u16;
        let x = area.right().saturating_sub(w + 1).max(area.x);
        draw::clear(
            buf,
            Rect::new(x, y, w.min(area.width), 1).intersection(area),
        );
        draw::text(buf, x, y, &text, theme::fg(color), area.right());
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
pub fn run(
    service: GameService,
    startup: Vec<GameEvent>,
    updates: UpdateService,
) -> io::Result<RunOutcome> {
    let mut guard = TerminalGuard::new()?;
    let mut app = App::new(service, startup);
    app.set_updates(updates);
    let result = app.run_loop(guard.terminal());
    let save_error = app.service.save().err().map(|e| e.to_string());
    drop(guard);
    let restart = app.should_restart() && save_error.is_none();
    result.map(|_| RunOutcome {
        save_error,
        restart,
    })
}
