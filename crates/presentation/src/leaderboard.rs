//! Classement en ligne, côté `App` : envoi régulier de la fiche de score,
//! pseudo, fenêtre du classement (terminal) et réactions aux réponses du
//! relais. Commun au terminal et au mode mobile.

use std::time::Duration;

use application::leaderboard::score_card;
use application::{Board, LeaderboardEvent, LeaderboardService};

use crate::app::App;
use crate::input::Intent;
use crate::theme;
use crate::ui::Popup;
use crate::widgets::leaderboard::LeaderboardView;

/// Envoi de la fiche de score (secondes).
pub const LEADERBOARD_INTERVAL: f64 = 300.0;
/// Rafraîchissement du classement affiché (secondes).
pub const LEADERBOARD_REFRESH: f64 = 60.0;
/// Longueur maximale d'un pseudo.
pub const PSEUDO_MAX: usize = 16;

/// Touche reçue pendant la saisie du pseudo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PseudoKey {
    Char(char),
    Backspace,
    Submit,
    Cancel,
}

impl App {
    /// Branche le classement : relit le pseudo (appareil qui a rejoint un
    /// lien) et envoie la fiche.
    pub fn set_leaderboard(&mut self, leaderboard: LeaderboardService) {
        self.leaderboard = leaderboard;
        if self.sync.linked() {
            self.leaderboard.refresh();
        }
        self.leaderboard_submit();
    }

    /// Jour courant (depuis 1970), selon l'horloge du jeu.
    fn lb_today(&self) -> u64 {
        self.service.now_unix() / 86_400
    }

    /// Envoie la fiche de la partie principale (jamais pendant un Défi du
    /// jour : l'écran montre alors l'univers de poche).
    pub fn leaderboard_submit(&mut self) {
        self.lb_timer = 0.0;
        if self.service.in_daily() || !self.sync.linked() || self.leaderboard.pseudo().is_none() {
            return;
        }
        let platform = if self.mobile.is_some() {
            "android"
        } else {
            "terminal"
        };
        let version = application::updates::display_version(self.updates.current_version());
        let card = score_card(
            &self.snap,
            self.lb_today(),
            platform,
            version.trim_start_matches('v'),
            self.service.now_unix(),
        );
        self.leaderboard.submit(card);
    }

    /// Fiche finale et attente des derniers envois (fermeture, arrière-plan).
    pub fn leaderboard_flush(&mut self, timeout: Duration) {
        self.leaderboard_submit();
        self.leaderboard.flush(timeout);
    }

    /// Choisit (ou retire, si vide) le pseudo du classement.
    pub fn set_pseudo(&mut self, pseudo: &str) {
        self.lb_user = true;
        if pseudo.trim().is_empty() {
            self.leaderboard.withdraw();
        } else {
            self.leaderboard.claim(pseudo.trim());
        }
    }

    /// Avant d'oublier le lien : le joueur quitte le classement (son identité
    /// dérive du lien). Attend la réponse quelques secondes au plus.
    pub(crate) fn withdraw_before_unlink(&mut self) {
        if self.leaderboard.pseudo().is_some() {
            self.leaderboard.withdraw();
            self.leaderboard.flush(Duration::from_secs(3));
        }
    }

    pub fn leaderboard_view(&self) -> LeaderboardView {
        let board = self.ui.leaderboard_board;
        LeaderboardView {
            available: self.leaderboard.available(),
            linked: self.sync.linked(),
            pseudo: self.leaderboard.pseudo().map(str::to_string),
            board,
            standings: self.leaderboard.standings(board).cloned(),
            busy: self.leaderboard.busy(),
            error: self.leaderboard.error().map(str::to_string),
            fetched_ago: self.leaderboard.fetched_ago(),
            site: self.leaderboard.site_url().to_string(),
        }
    }

    /// Demande les deux classements (affichage).
    pub fn leaderboard_fetch(&mut self) {
        self.lb_fetch_timer = 0.0;
        let day = self.lb_today();
        self.leaderboard.fetch(Board::General, day);
        self.leaderboard.fetch(Board::Daily, day);
    }

    /// Le classement est affiché (fenêtre du terminal ou onglet mobile).
    fn leaderboard_shown(&self) -> bool {
        self.ui.leaderboard.is_some()
            || self
                .mobile
                .as_ref()
                .is_some_and(|m| m.screen == crate::mobile::LEADERBOARD_SCREEN)
    }

    pub(crate) fn update_leaderboard(&mut self, real_dt: f64) {
        while let Some(ev) = self.leaderboard.poll() {
            self.react_leaderboard(ev);
        }
        self.lb_timer += real_dt;
        if self.lb_timer >= LEADERBOARD_INTERVAL {
            self.leaderboard_submit();
        }
        let shown = self.leaderboard_shown();
        if shown && !self.lb_shown {
            self.leaderboard_fetch();
        } else if shown {
            self.lb_fetch_timer += real_dt;
            if self.lb_fetch_timer >= LEADERBOARD_REFRESH {
                self.leaderboard_fetch();
            }
        }
        self.lb_shown = shown;
    }

    fn react_leaderboard(&mut self, ev: LeaderboardEvent) {
        let now = self.now;
        let user = std::mem::take(&mut self.lb_user);
        match ev {
            LeaderboardEvent::Claimed(p) => {
                self.ui.pseudo_input = None;
                self.toasts.push(
                    now,
                    theme::GOLD,
                    format!("★ Pseudo « {p} » enregistré"),
                    "Ta fonderie apparaît au classement",
                );
                self.leaderboard_submit();
                self.leaderboard_fetch();
            }
            LeaderboardEvent::Taken => {
                self.toasts.push(
                    now,
                    theme::ORANGE,
                    "Pseudo déjà pris",
                    "Choisis-en un autre",
                );
            }
            LeaderboardEvent::Invalid(why) => {
                self.toasts.push(now, theme::ORANGE, "Pseudo refusé", why);
            }
            LeaderboardEvent::Withdrawn => {
                if user {
                    self.toasts.push(
                        now,
                        theme::GREY,
                        "Classement",
                        "Tu n'apparais plus au classement",
                    );
                }
            }
            LeaderboardEvent::Pseudo(Some(_)) => self.leaderboard_submit(),
            LeaderboardEvent::Pseudo(None) | LeaderboardEvent::Submitted => {}
            LeaderboardEvent::Standings(_) => {}
            LeaderboardEvent::Failed(msg) => {
                // Hors ligne en arrière-plan : visible dans la fenêtre, sans toast.
                if user {
                    self.toasts
                        .push(now, theme::ORANGE, "Classement injoignable", msg);
                }
            }
        }
    }

    /// Options › Classement (le classement est demandé à l'image suivante).
    pub(crate) fn open_leaderboard(&mut self) {
        self.ui.leaderboard = Some(Popup {
            opened_at: self.now,
            closing_at: None,
        });
    }

    /// Touches de la fenêtre du classement (terminal).
    pub(crate) fn on_leaderboard_intent(&mut self, intent: Intent) {
        match intent {
            Intent::NextPanel | Intent::PrevPanel | Intent::Up | Intent::Down => {
                self.ui.leaderboard_board = match self.ui.leaderboard_board {
                    Board::General => Board::Daily,
                    Board::Daily => Board::General,
                };
            }
            Intent::Buy | Intent::Yes if self.sync.linked() => self.start_pseudo_input(),
            Intent::Buy | Intent::Yes => {
                // Pas de lien : la synchronisation d'abord.
                self.ui.leaderboard = None;
                self.open_sync_view();
            }
            Intent::Escape | Intent::No | Intent::Options | Intent::Quit => {
                if let Some(p) = self.ui.leaderboard.as_mut() {
                    p.closing_at = Some(self.now);
                }
            }
            _ => {}
        }
    }

    /// Ouvre la saisie du pseudo (terminal), préremplie avec l'actuel.
    pub(crate) fn start_pseudo_input(&mut self) {
        if self.leaderboard.available() && self.sync.linked() {
            let current = self.leaderboard.pseudo().unwrap_or_default().to_string();
            self.ui.pseudo_input = Some(current);
        }
    }

    /// Une touche pendant la saisie du pseudo.
    pub fn on_pseudo_key(&mut self, key: PseudoKey) {
        let Some(text) = self.ui.pseudo_input.as_mut() else {
            return;
        };
        match key {
            PseudoKey::Char(c)
                if (c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
                    && text.chars().count() < PSEUDO_MAX =>
            {
                text.push(c);
            }
            PseudoKey::Char(_) => {}
            PseudoKey::Backspace => {
                text.pop();
            }
            PseudoKey::Submit => {
                let pseudo = text.clone();
                let changed = self.leaderboard.pseudo() != Some(pseudo.trim());
                if changed && !(pseudo.trim().is_empty() && self.leaderboard.pseudo().is_none()) {
                    self.set_pseudo(&pseudo);
                }
                self.ui.pseudo_input = None;
            }
            PseudoKey::Cancel => self.ui.pseudo_input = None,
        }
    }
}
