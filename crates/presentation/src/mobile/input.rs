//! Entrées tactiles du mode mobile : toucher, maintien, défilement, balayage,
//! retour Android. Chaque toucher est résolu par le hit-test de l'image affichée.

use application::Command;

use super::{Confirm, Confirming, Hit, MobileUi, Press, Request, Tab, TabInfo, LONG_PRESS};
use super::{REPEAT_EVERY, REPEAT_START};
use crate::app::{App, Screen};
use crate::input::{Intent, Phase};

impl App {
    fn mobile_mut(&mut self) -> Option<&mut MobileUi> {
        self.mobile.as_mut()
    }

    /// Une séquence plein écran est en cours (barre d'onglets masquée).
    pub fn in_sequence(&self) -> bool {
        self.screen == Screen::Title
            || self.vis.nova.is_some()
            || self.vis.bang.is_some()
            || self.vis.ending.is_some()
            || self.ui.portal.is_some()
            || self.ui.wipe_at.is_some()
            || self.vis.key_ceremony.is_some()
            || self.vis.relic_card.is_some()
    }

    /// Un écran du terminal (popup) est ouvert : affiché en grille dense, piloté
    /// par une rangée de boutons.
    pub fn legacy_open(&self) -> bool {
        let ui = &self.ui;
        ui.popup.is_some()
            || ui.challenges.is_some()
            || ui.lab.is_some()
            || ui.cosmos.is_some()
            || ui.daily.is_some()
            || ui.mega.is_some()
            || ui.relics.is_some()
            || ui.starmap.is_some()
            || ui.event.is_some()
            || ui.shop.is_some()
            || ui.codex.is_some()
            || ui.options.is_some()
            || ui.portal.is_some()
    }

    /// La grille veut des cellules plus petites (≈ 88 colonnes) : lancement,
    /// séquences et écrans hérités du terminal.
    pub fn mobile_wants_dense(&self) -> bool {
        self.in_sequence() || self.legacy_open()
    }

    pub fn mobile_bar_visible(&self) -> bool {
        self.mobile.is_some() && !self.in_sequence()
    }

    pub fn mobile_tabs(&self) -> Vec<TabInfo> {
        let Some(m) = self.mobile.as_ref() else {
            return Vec::new();
        };
        let s = &self.snap;
        Tab::ALL
            .iter()
            .map(|&tab| {
                let (unlocked, badge) = match tab {
                    Tab::Foundry => (true, s.comet_progress.is_some()),
                    Tab::Factories => (true, s.generators.iter().any(|g| g.affordable)),
                    Tab::Upgrade => (
                        true,
                        s.upgrades.iter().any(|u| u.affordable)
                            || s.perks.iter().any(|p| p.affordable && !p.owned)
                            || s.constellations.iter().any(|c| c.affordable && !c.maxed),
                    ),
                    Tab::Cosmos => (
                        cosmos_unlocked(s),
                        s.pending_event.is_some()
                            || s.crunch_available
                            || (s.lab_unlocked
                                && s.research.is_none()
                                && s.lab.iter().any(|l| l.available)),
                    ),
                    Tab::More => (true, false),
                };
                TabInfo {
                    tab,
                    unlocked,
                    active: m.tab == tab,
                    badge,
                }
            })
            .collect()
    }

    pub fn mobile_select_tab(&mut self, tab: Tab) {
        let now = self.now;
        let unlocked = tab != Tab::Cosmos || cosmos_unlocked(&self.snap);
        if let Some(m) = self.mobile_mut() {
            if unlocked && m.tab != tab {
                m.tab = tab;
                m.detail = None;
                m.press = None;
                m.confirm = None;
                m.tab_changed_at = now;
            }
        }
    }

    /// Doigt posé sur la cellule (`col`, `row`).
    pub fn touch_down(&mut self, col: u16, row: u16) {
        if self.mobile.is_none() || self.screen == Screen::Title {
            self.on_harvest_input(Phase::Press);
            return;
        }
        if self.in_sequence() && self.ui.portal.is_none() {
            // Toucher accélère ou passe, comme une touche au clavier.
            self.on_intent(Intent::Buy);
            return;
        }
        let now = self.now;
        let Some((rect, hit)) = self.mobile.as_ref().and_then(|m| m.hits.at(col, row)) else {
            return;
        };
        if hit == Hit::Harvest {
            self.ui.harvest_at = Some((col as f64, row as f64));
            self.on_harvest_input(Phase::Press);
        }
        if let Some(m) = self.mobile_mut() {
            m.press = Some(Press {
                rect,
                hit,
                at: now,
                last_repeat: now,
                consumed: false,
                last: (col, row),
                moved: false,
            });
        }
    }

    /// Doigt toujours posé (appelé à chaque image) : récolte continue, achats
    /// en rafale, appui long.
    pub fn touch_hold(&mut self) {
        let now = self.now;
        if self.mobile.is_none() || self.screen == Screen::Title {
            self.on_harvest_input(Phase::Repeat);
            return;
        }
        let Some(p) = self.mobile.as_ref().and_then(|m| m.press) else {
            return;
        };
        let held = now - p.at;
        match p.hit {
            Hit::Harvest => {
                if held >= REPEAT_START {
                    self.on_harvest_input(Phase::Repeat);
                }
            }
            hit if hit.repeats() => {
                if held >= REPEAT_START && now - p.last_repeat >= REPEAT_EVERY {
                    self.fire(hit);
                    if let Some(p) = self.mobile_mut().and_then(|m| m.press.as_mut()) {
                        p.last_repeat = now;
                        p.consumed = true;
                    }
                }
            }
            Hit::Details(d) if held >= LONG_PRESS && !p.consumed => {
                if let Some(m) = self.mobile_mut() {
                    m.detail = Some(d);
                    if let Some(p) = m.press.as_mut() {
                        p.consumed = true;
                    }
                }
            }
            _ => {}
        }
    }

    /// Doigt levé sur la cellule (`col`, `row`) : un toucher simple agit ici.
    pub fn touch_up(&mut self, col: u16, row: u16) {
        if self.mobile.is_none() || self.screen == Screen::Title {
            self.on_harvest_input(Phase::Release);
            return;
        }
        let Some(p) = self.mobile_mut().and_then(|m| m.press.take()) else {
            return;
        };
        if p.hit == Hit::Harvest {
            self.on_harvest_input(Phase::Release);
            return;
        }
        if p.hit == Hit::Slider {
            let done = self
                .mobile
                .as_ref()
                .and_then(|m| m.confirm)
                .filter(|c| c.slide >= SLIDE_DONE);
            match done {
                Some(c) => self.execute(c.what),
                None => {
                    if let Some(c) = self.mobile_mut().and_then(|m| m.confirm.as_mut()) {
                        c.slide = 0.0;
                    }
                }
            }
            return;
        }
        if p.hit == Hit::Map {
            if !p.moved {
                let snap = self.snap.clone();
                if let Some(m) = self.mobile_mut() {
                    if let Some(i) = super::cosmos::system_at(&snap, m, p.rect, col, row) {
                        m.map_sel = i;
                    }
                }
            }
            return;
        }
        let inside =
            col >= p.rect.x && col < p.rect.right() && row >= p.rect.y && row < p.rect.bottom();
        if p.consumed || !inside {
            return;
        }
        match p.hit {
            // Un toucher bref sur une carte ouvre aussi sa fiche.
            Hit::Details(d) => {
                if let Some(m) = self.mobile_mut() {
                    m.detail = Some(d);
                }
            }
            hit => self.fire(hit),
        }
    }

    /// Doigt qui glisse sur la cellule (`col`, `row`). Renvoie `true` si la zone
    /// touchée suit le doigt elle-même (curseur, carte) : la plateforme ne
    /// transforme alors pas le geste en défilement ou en balayage.
    pub fn touch_move(&mut self, col: u16, row: u16) -> bool {
        let Some(m) = self.mobile.as_mut() else {
            return false;
        };
        let Some(p) = m.press.as_mut() else {
            return false;
        };
        match p.hit {
            Hit::Slider => {
                let r = p.rect;
                let travel = r.width.saturating_sub(2 + 5).max(1) as f64;
                let k = (col as f64 - r.x as f64 - 1.0 - 2.0) / travel;
                p.moved = true;
                if let Some(c) = m.confirm.as_mut() {
                    c.slide = k.clamp(0.0, 1.0);
                }
                true
            }
            Hit::Map => {
                let (dx, dy) = (col as i32 - p.last.0 as i32, row as i32 - p.last.1 as i32);
                if dx != 0 || dy != 0 {
                    p.moved = true;
                    p.last = (col, row);
                    let view = p.rect;
                    m.map_pan.0 -= dx;
                    m.map_pan.1 -= dy;
                    super::cosmos::clamp_pan(m, view.width, view.height);
                }
                true
            }
            _ => false,
        }
    }

    /// Le geste devient un glissement : le toucher en cours est annulé.
    pub fn touch_cancel(&mut self) {
        let was_harvest = self
            .mobile_mut()
            .and_then(|m| m.press.take())
            .is_some_and(|p| p.hit == Hit::Harvest);
        if was_harvest {
            self.on_harvest_input(Phase::Release);
        }
    }

    /// Défilement vertical de l'écran courant (`rows` > 0 : vers le bas).
    pub fn scroll_by(&mut self, rows: i32) {
        if self.legacy_open() {
            let intent = if rows > 0 { Intent::Down } else { Intent::Up };
            for _ in 0..rows.unsigned_abs().min(5) {
                self.on_intent(intent);
            }
            return;
        }
        let journal = self
            .mobile
            .as_ref()
            .is_some_and(|m| m.screen == super::screens::JOURNAL);
        if journal {
            // Le journal défile depuis le bas : glisser vers le bas remonte le temps.
            let max = self.journal.len().saturating_sub(1);
            let s = self.ui.journal_scroll as i64 - rows as i64;
            self.ui.journal_scroll = s.clamp(0, max as i64) as usize;
            return;
        }
        if let Some(m) = self.mobile_mut() {
            let i = m.screen;
            let s = m.scroll[i] as i32 + rows;
            m.scroll[i] = s.clamp(0, m.scroll_max[i] as i32) as u16;
        }
    }

    /// Balayage horizontal : sous-onglet voisin, sinon onglet voisin.
    pub fn swipe(&mut self, dir: i32) {
        if self.in_sequence() && self.ui.portal.is_none() {
            return;
        }
        if self.legacy_open() {
            self.on_intent(if dir > 0 {
                Intent::NextPanel
            } else {
                Intent::PrevPanel
            });
            return;
        }
        let snap_subs = super::screens::sub_tabs(&self.snap, self.mobile.as_ref().map(|m| m.tab));
        let Some(m) = self.mobile.as_mut() else {
            return;
        };
        let t = m.tab.index();
        let cur = m.sub[t] as i32 + dir;
        if !snap_subs.is_empty() && cur >= 0 && (cur as usize) < snap_subs.len() {
            m.sub[t] = cur as usize;
            m.detail = None;
            return;
        }
        let tabs: Vec<Tab> = self
            .mobile_tabs()
            .into_iter()
            .filter(|t| t.unlocked)
            .map(|t| t.tab)
            .collect();
        let Some(pos) = tabs
            .iter()
            .position(|&x| Some(x) == self.mobile.as_ref().map(|m| m.tab))
        else {
            return;
        };
        let next = pos as i32 + dir;
        if next >= 0 && (next as usize) < tabs.len() {
            self.mobile_select_tab(tabs[next as usize]);
        }
    }

    /// Retour Android : ferme la fiche ou la popup, sinon revient à Fonderie,
    /// sinon demande à quitter (`should_quit`).
    pub fn mobile_back(&mut self) {
        if self.screen == Screen::Title || self.vis.ending.is_some() {
            self.on_intent(Intent::Escape);
            return;
        }
        if self.in_sequence() {
            self.on_intent(Intent::Escape);
            return;
        }
        if let Some(m) = self.mobile_mut() {
            if !m.absence.is_empty() {
                m.absence.clear();
                return;
            }
            if m.confirm.take().is_some() || m.detail.take().is_some() {
                return;
            }
            if m.page.take().is_some() {
                return;
            }
        }
        if self.legacy_open() {
            self.on_intent(Intent::Escape);
            return;
        }
        match self.mobile.as_ref().map(|m| m.tab) {
            Some(Tab::Foundry) | None => self.quit = true,
            Some(_) => self.mobile_select_tab(Tab::Foundry),
        }
    }

    /// Ouvre une page du Cosmos (tests, raccourcis).
    pub fn mobile_open(&mut self, page: super::Page) {
        self.mobile_select_tab(Tab::Cosmos);
        if let Some(m) = self.mobile_mut() {
            m.tab = Tab::Cosmos;
        }
        self.fire(Hit::OpenPage(page));
    }

    /// Zones touchables de la dernière image (tests, outils).
    pub fn mobile_hits(&self) -> Vec<(ratatui::layout::Rect, Hit)> {
        self.mobile
            .as_ref()
            .map(|m| m.hits.zones().to_vec())
            .unwrap_or_default()
    }

    /// Prochaine demande à traiter par la plateforme.
    pub fn take_request(&mut self) -> Option<Request> {
        let m = self.mobile_mut()?;
        if m.requests.is_empty() {
            None
        } else {
            Some(m.requests.remove(0))
        }
    }

    /// Message affiché après une action de la plateforme (export, import…).
    pub fn notify(&mut self, title: &str, body: &str) {
        let now = self.now;
        self.toasts
            .push(now, crate::theme::CYAN, title.to_string(), body.to_string());
    }

    fn fire(&mut self, hit: Hit) {
        match hit {
            Hit::Harvest => {}
            Hit::CatchComet => self.on_intent(Intent::CatchComet),
            Hit::BuyGenerator(id) => self.command(Command::BuyGenerator(id)),
            Hit::BuyUpgrade(id) => self.command(Command::BuyUpgrade(id)),
            Hit::BuyPerk(id) => self.command(Command::BuyPerk(id)),
            Hit::BuyConstellation(id) => self.command(Command::BuyConstellation(id)),
            Hit::SetBuyMode(mode) => {
                for _ in 0..4 {
                    if self.snap.buy_mode == mode {
                        break;
                    }
                    self.command(Command::CycleBuyMode);
                    self.service.fill_snapshot(&mut self.snap);
                }
            }
            Hit::ToggleAutoBuy => self.command(Command::ToggleAutoBuy),
            Hit::BuyAllUpgrades => self.command(Command::BuyAllUpgrades),
            Hit::ToggleProduction => {
                if let Some(m) = self.mobile_mut() {
                    m.production_open = !m.production_open;
                }
            }
            Hit::SubTab(i) => {
                if let Some(m) = self.mobile_mut() {
                    let t = m.tab.index();
                    m.sub[t] = i;
                    m.detail = None;
                }
            }
            Hit::Open(tab, sub) => {
                self.mobile_select_tab(tab);
                if let Some(m) = self.mobile_mut() {
                    m.sub[tab.index()] = sub;
                }
            }
            Hit::Details(d) => {
                if let Some(m) = self.mobile_mut() {
                    m.detail = Some(d);
                }
            }
            Hit::CloseDetail => {
                if let Some(m) = self.mobile_mut() {
                    m.detail = None;
                }
            }
            Hit::CycleColors => self.command(Command::CycleColorMode),
            Hit::CycleFrameRate => self.command(Command::CycleFrameRate),
            Hit::ToggleReducedEffects => self.command(Command::ToggleReducedEffects),
            Hit::Request(r) => {
                if let Some(m) = self.mobile_mut() {
                    m.requests.push(r);
                }
            }
            Hit::Key(intent) => self.on_intent(intent),
            Hit::Block | Hit::Slider | Hit::Map => {}
            Hit::CloseAbsence => {
                if let Some(m) = self.mobile_mut() {
                    m.absence.clear();
                }
            }
            Hit::OpenPage(page) => {
                if let Some(m) = self.mobile_mut() {
                    m.page = Some(page);
                    m.scroll[super::cosmos::page_screen(page)] = 0;
                    if page == super::Page::StarMap {
                        m.map_centered = false;
                    }
                }
            }
            Hit::ClosePage => {
                if let Some(m) = self.mobile_mut() {
                    m.page = None;
                }
            }
            Hit::Ask(what) => {
                let now = self.now;
                if let Some(m) = self.mobile_mut() {
                    m.confirm = Some(Confirming {
                        what,
                        slide: 0.0,
                        opened_at: now,
                    });
                }
            }
            Hit::CancelConfirm => {
                if let Some(m) = self.mobile_mut() {
                    m.confirm = None;
                }
            }
            Hit::EventChoice(i) => {
                self.command(Command::ResolveEvent(i));
                if let Some(m) = self.mobile_mut() {
                    m.page = None;
                }
            }
            Hit::DailyToggle => {
                let cmd = if self.snap.daily.is_some() {
                    Command::EndDaily
                } else {
                    Command::StartDaily
                };
                self.command(cmd);
            }
            Hit::Lab(pos) => {
                self.ui.lab_sel = pos;
                self.lab_action();
            }
            Hit::Cosmogony(i) => {
                self.ui.cosmos_sel = i;
                self.ui.cosmos_armed = false;
                self.cosmos_action();
            }
            Hit::ToggleRelic(i) => self.command(Command::ToggleRelic(domain::RelicId(i))),
            Hit::SendFleet => {
                if let Some(sel) = self.mobile.as_ref().map(|m| m.map_sel) {
                    self.command(Command::LaunchExpedition(sel));
                }
            }
            Hit::ToggleAutoFleets => self.command(Command::ToggleAutoFleets),
            Hit::Convergence => self.command(Command::StartConvergence),
            Hit::MegaCell(c) => {
                if let Some(m) = self.mobile_mut() {
                    m.mega_sel = c;
                }
            }
            Hit::Build(kind) => {
                if let Some(sel) = self.mobile.as_ref().map(|m| m.mega_sel) {
                    self.command(Command::PlaceStructure(sel, kind));
                }
            }
        }
    }

    /// Action confirmée en faisant glisser le curseur jusqu'au bout.
    fn execute(&mut self, what: Confirm) {
        if let Some(m) = self.mobile_mut() {
            m.confirm = None;
        }
        match what {
            Confirm::Supernova => self.start_supernova(),
            Confirm::Crunch => self.start_crunch(),
            Confirm::Challenge(i) => {
                self.ui.challenges_sel = i;
                self.ui.challenge_armed = true;
                self.challenge_action();
            }
            Confirm::BigBang => {
                self.ui.cosmos_sel = 0;
                self.ui.cosmos_armed = true;
                self.cosmos_action();
                if let Some(m) = self.mobile_mut() {
                    m.page = None;
                }
            }
            Confirm::NewGame => {
                self.ui.wipe_at = Some(self.now);
                if let Some(m) = self.mobile_mut() {
                    m.tab = Tab::Foundry;
                    m.page = None;
                }
            }
            Confirm::Demolish(cell) => self.command(Command::Demolish(cell)),
            Confirm::Awakening => self.command(Command::StartAwakening),
        }
    }
}

/// Position du curseur à partir de laquelle l'action est confirmée.
const SLIDE_DONE: f64 = 0.92;

/// L'onglet Cosmos s'ouvre avec la première fonctionnalité d'après-partie.
pub fn cosmos_unlocked(s: &application::GameSnapshot) -> bool {
    s.supernovas > 0
        || s.crunches > 0
        || s.challenges.iter().any(|c| c.unlocked)
        || s.pending_event.is_some()
}
