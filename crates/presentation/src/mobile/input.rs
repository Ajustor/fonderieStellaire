//! Entrées tactiles du mode mobile : toucher, maintien, défilement, balayage,
//! retour Android. Chaque toucher est résolu par le hit-test de l'image affichée.

use application::Command;

use super::{Hit, MobileUi, Press, Request, Tab, TabInfo, LONG_PRESS};
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
                m.wipe_armed = false;
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
        if self.in_sequence() {
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
        if self.in_sequence() {
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
            if m.detail.take().is_some() {
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
                    m.wipe_armed = false;
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
            Hit::NewGame => {
                let armed = self.mobile.as_ref().is_some_and(|m| m.wipe_armed);
                if armed {
                    if let Some(m) = self.mobile_mut() {
                        m.wipe_armed = false;
                        m.tab = Tab::Foundry;
                    }
                    self.ui.wipe_at = Some(self.now);
                } else if let Some(m) = self.mobile_mut() {
                    m.wipe_armed = true;
                }
            }
            Hit::Request(r) => {
                if let Some(m) = self.mobile_mut() {
                    m.requests.push(r);
                }
            }
            Hit::Key(intent) => self.on_intent(intent),
        }
    }
}

/// L'onglet Cosmos s'ouvre avec la première fonctionnalité d'après-partie.
pub fn cosmos_unlocked(s: &application::GameSnapshot) -> bool {
    s.supernovas > 0
        || s.crunches > 0
        || s.challenges.iter().any(|c| c.unlocked)
        || s.pending_event.is_some()
}
