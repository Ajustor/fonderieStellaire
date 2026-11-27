//! Classement en ligne : fiche de score, port vers le relais et service.
//!
//! Seuls les appareils **synchronisés** (lien enregistré) qui ont choisi un
//! **pseudo** apparaissent au classement : l'identité du joueur dérive du code
//! de lien, partagé par tous ses appareils. La fiche de score part en clair
//! (signée) ; la partie, elle, reste chiffrée de bout en bout.
//!
//! Comme la synchronisation, tout le réseau se fait dans un thread : la
//! présentation envoie des demandes et lit les [`LeaderboardEvent`] avec
//! [`LeaderboardService::poll`].

use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::snapshot::GameSnapshot;

/// Fiche envoyée au relais.
#[derive(Clone, Debug, PartialEq)]
pub struct ScoreCard {
    /// 1 à 4 ; 5 une fois l'Acte IV terminé.
    pub act: u8,
    /// Complétion globale (0 → 1).
    pub progress: f64,
    /// log10 de la poussière récoltée au total.
    pub total_log: f64,
    pub supernovas: u32,
    pub crunches: u32,
    pub big_bangs: u32,
    pub achievements: u32,
    /// Temps de jeu (secondes).
    pub play_time: f64,
    /// Meilleur score du Défi du jour : (jour depuis 1970, score).
    pub daily: Option<(u64, f64)>,
    pub platform: &'static str,
    pub version: String,
    /// Heure d'envoi (secondes Unix).
    pub sent_at: u64,
    /// Début de la partie (dernier « Nouvelle partie », secondes Unix) : le
    /// relais repart de zéro pour une partie plus récente.
    pub started_at: u64,
}

/// Acte atteint : 1 + premier Big Crunch + Grand Attracteur éveillé +
/// Convergence + Cosmos éveillé.
pub fn act_of(snap: &GameSnapshot) -> u8 {
    1 + u8::from(snap.crunches > 0 || snap.big_bangs > 0)
        + u8::from(snap.attractor_awakened)
        + u8::from(snap.converged)
        + u8::from(snap.awakened)
}

/// Fiche de la partie principale. `today` : jour courant (depuis 1970).
pub fn score_card(
    snap: &GameSnapshot,
    today: u64,
    platform: &'static str,
    version: &str,
    now: u64,
) -> ScoreCard {
    ScoreCard {
        act: act_of(snap),
        progress: snap.progress.completion.clamp(0.0, 1.0),
        total_log: snap.total_all.max(1.0).log10(),
        supernovas: snap.supernovas,
        crunches: snap.crunches,
        big_bangs: snap.big_bangs,
        achievements: snap.achievements_unlocked as u32,
        play_time: snap.play_time.max(0.0),
        daily: (snap.daily_best > 0.0).then_some((today, snap.daily_best)),
        platform,
        version: version.to_string(),
        sent_at: now,
        started_at: snap.started_at,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Board {
    #[default]
    General,
    Daily,
}

/// Une ligne du classement.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Entry {
    pub rank: u32,
    pub pseudo: String,
    pub me: bool,
    pub act: u8,
    pub progress: f64,
    pub total_log: f64,
    pub supernovas: u32,
    pub crunches: u32,
    pub big_bangs: u32,
    pub play_time: f64,
    pub platform: String,
    /// Score du Défi du jour (classement du jour seulement).
    pub score: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Standings {
    pub board: Board,
    pub day: u64,
    /// Nombre total de joueurs classés.
    pub total: u32,
    pub entries: Vec<Entry>,
    /// Ligne du joueur, même hors des premiers.
    pub me: Option<Entry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LbError {
    /// Pseudo refusé (raison donnée par le relais).
    Invalid(String),
    /// Pseudo déjà pris par un autre joueur.
    Taken,
    /// Cet appareil n'est pas synchronisé.
    NotLinked,
    Network(String),
}

impl fmt::Display for LbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LbError::Invalid(why) => write!(f, "pseudo refusé : {why}"),
            LbError::Taken => f.write_str("ce pseudo est déjà pris"),
            LbError::NotLinked => f.write_str("synchronise d'abord cet appareil"),
            LbError::Network(e) => f.write_str(e),
        }
    }
}

/// Port : le relais, vu du classement.
pub trait LeaderboardBackend: Send {
    /// Un lien de synchronisation est enregistré sur cet appareil.
    fn linked(&self) -> bool;
    /// Pseudo connu localement pour le lien actuel.
    fn pseudo(&self) -> Option<String>;
    /// Relit le pseudo enregistré sur le relais (appareil qui rejoint un lien).
    fn refresh_pseudo(&mut self) -> Result<Option<String>, LbError>;
    /// Réserve ou change le pseudo ; renvoie le pseudo nettoyé.
    fn claim(&mut self, pseudo: &str) -> Result<String, LbError>;
    /// Retire le joueur du classement.
    fn withdraw(&mut self) -> Result<(), LbError>;
    fn submit(&self, card: &ScoreCard) -> Result<(), LbError>;
    fn standings(&self, board: Board, day: u64) -> Result<Standings, LbError>;
    /// Adresse du site du classement.
    fn site_url(&self) -> String;
}

#[derive(Clone, Debug, PartialEq)]
pub enum LeaderboardEvent {
    Claimed(String),
    Taken,
    Invalid(String),
    Withdrawn,
    /// Pseudo relu depuis le relais (`None` : aucun).
    Pseudo(Option<String>),
    Submitted,
    Standings(Box<Standings>),
    Failed(String),
}

enum Job {
    Claim(String),
    Withdraw,
    Refresh,
    Submit(Box<ScoreCard>),
    Fetch(Board, u64),
}

pub struct LeaderboardService {
    tx: Option<Sender<Job>>,
    rx: Option<Receiver<LeaderboardEvent>>,
    pending: Arc<AtomicUsize>,
    pseudo: Option<String>,
    site: String,
    general: Option<Standings>,
    daily: Option<Standings>,
    error: Option<String>,
    fetched_at: Option<Instant>,
}

impl LeaderboardService {
    /// Classement indisponible (relais non configuré, tests…).
    pub fn disabled() -> Self {
        LeaderboardService {
            tx: None,
            rx: None,
            pending: Arc::new(AtomicUsize::new(0)),
            pseudo: None,
            site: String::new(),
            general: None,
            daily: None,
            error: None,
            fetched_at: None,
        }
    }

    pub fn spawn(backend: Box<dyn LeaderboardBackend>) -> Self {
        let pseudo = backend.pseudo();
        let site = backend.site_url();
        let (job_tx, job_rx) = mpsc::channel();
        let (ev_tx, ev_rx) = mpsc::channel();
        let pending = Arc::new(AtomicUsize::new(0));
        let p = Arc::clone(&pending);
        let spawned = thread::Builder::new()
            .name("classement".into())
            .spawn(move || worker(backend, &job_rx, &ev_tx, &p));
        match spawned {
            Ok(_) => LeaderboardService {
                tx: Some(job_tx),
                rx: Some(ev_rx),
                pending,
                pseudo,
                site,
                ..Self::disabled()
            },
            Err(_) => Self::disabled(),
        }
    }

    pub fn available(&self) -> bool {
        self.tx.is_some()
    }

    /// Pseudo du joueur (il apparaît au classement).
    pub fn pseudo(&self) -> Option<&str> {
        self.pseudo.as_deref()
    }

    pub fn site_url(&self) -> &str {
        &self.site
    }

    pub fn busy(&self) -> bool {
        self.pending.load(Ordering::SeqCst) > 0
    }

    /// Dernière erreur (effacée par la réponse suivante).
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Dernier classement reçu.
    pub fn standings(&self, board: Board) -> Option<&Standings> {
        match board {
            Board::General => self.general.as_ref(),
            Board::Daily => self.daily.as_ref(),
        }
    }

    /// Secondes depuis la dernière réception d'un classement.
    pub fn fetched_ago(&self) -> Option<f64> {
        self.fetched_at.map(|t| t.elapsed().as_secs_f64())
    }

    fn send(&mut self, job: Job) {
        if let Some(tx) = &self.tx {
            self.pending.fetch_add(1, Ordering::SeqCst);
            if tx.send(job).is_err() {
                self.pending.fetch_sub(1, Ordering::SeqCst);
                self.tx = None;
            }
        }
    }

    pub fn claim(&mut self, pseudo: &str) {
        self.send(Job::Claim(pseudo.to_string()));
    }

    pub fn withdraw(&mut self) {
        self.send(Job::Withdraw);
    }

    /// Relit le pseudo depuis le relais (après avoir rejoint un lien).
    pub fn refresh(&mut self) {
        self.send(Job::Refresh);
    }

    /// Envoie la fiche (rien sans pseudo).
    pub fn submit(&mut self, card: ScoreCard) {
        if self.pseudo.is_some() {
            self.send(Job::Submit(Box::new(card)));
        }
    }

    pub fn fetch(&mut self, board: Board, day: u64) {
        self.send(Job::Fetch(board, day));
    }

    /// Le lien a été oublié : plus de pseudo ni de classement personnel.
    pub fn forget(&mut self) {
        self.pseudo = None;
        for s in [&mut self.general, &mut self.daily].into_iter().flatten() {
            s.me = None;
            for e in &mut s.entries {
                e.me = false;
            }
        }
    }

    pub fn poll(&mut self) -> Option<LeaderboardEvent> {
        let ev = self.rx.as_ref()?.try_recv().ok()?;
        self.error = None;
        match &ev {
            LeaderboardEvent::Claimed(p) => self.pseudo = Some(p.clone()),
            LeaderboardEvent::Pseudo(p) => self.pseudo = p.clone(),
            LeaderboardEvent::Withdrawn => self.forget(),
            LeaderboardEvent::Standings(s) => {
                self.fetched_at = Some(Instant::now());
                match s.board {
                    Board::General => self.general = Some((**s).clone()),
                    Board::Daily => self.daily = Some((**s).clone()),
                }
            }
            LeaderboardEvent::Taken => self.error = Some(LbError::Taken.to_string()),
            LeaderboardEvent::Invalid(why) => {
                self.error = Some(LbError::Invalid(why.clone()).to_string());
            }
            LeaderboardEvent::Failed(e) => self.error = Some(e.clone()),
            LeaderboardEvent::Submitted => {}
        }
        Some(ev)
    }

    /// Attend la fin des demandes en cours (fermeture du jeu), au plus `timeout`.
    pub fn flush(&self, timeout: Duration) {
        let end = Instant::now() + timeout;
        while self.busy() && Instant::now() < end {
            thread::sleep(Duration::from_millis(30));
        }
    }
}

fn failure(e: LbError) -> LeaderboardEvent {
    match e {
        LbError::Taken => LeaderboardEvent::Taken,
        LbError::Invalid(why) => LeaderboardEvent::Invalid(why),
        e => LeaderboardEvent::Failed(e.to_string()),
    }
}

fn worker(
    mut backend: Box<dyn LeaderboardBackend>,
    jobs: &Receiver<Job>,
    events: &Sender<LeaderboardEvent>,
    pending: &AtomicUsize,
) {
    while let Ok(first) = jobs.recv() {
        let mut batch = vec![first];
        batch.extend(jobs.try_iter());
        // Seule la dernière fiche d'une rafale compte.
        let last_submit = batch.iter().rposition(|j| matches!(j, Job::Submit(_)));
        for (i, job) in batch.into_iter().enumerate() {
            let ev = match job {
                Job::Submit(_) if Some(i) != last_submit => None,
                Job::Submit(card) => Some(match backend.submit(&card) {
                    Ok(()) => LeaderboardEvent::Submitted,
                    Err(e) => failure(e),
                }),
                Job::Claim(p) => Some(match backend.claim(&p) {
                    Ok(p) => LeaderboardEvent::Claimed(p),
                    Err(e) => failure(e),
                }),
                Job::Withdraw => Some(match backend.withdraw() {
                    Ok(()) => LeaderboardEvent::Withdrawn,
                    Err(e) => failure(e),
                }),
                Job::Refresh => Some(match backend.refresh_pseudo() {
                    Ok(p) => LeaderboardEvent::Pseudo(p),
                    Err(e) => failure(e),
                }),
                Job::Fetch(board, day) => Some(match backend.standings(board, day) {
                    Ok(s) => LeaderboardEvent::Standings(Box::new(s)),
                    Err(e) => failure(e),
                }),
            };
            if let Some(ev) = ev {
                let _ = events.send(ev);
            }
            pending.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Log {
        submitted: Vec<ScoreCard>,
        pseudo: Option<String>,
    }

    struct Fake(Arc<Mutex<Log>>);

    impl LeaderboardBackend for Fake {
        fn linked(&self) -> bool {
            true
        }
        fn pseudo(&self) -> Option<String> {
            self.0.lock().unwrap().pseudo.clone()
        }
        fn refresh_pseudo(&mut self) -> Result<Option<String>, LbError> {
            Ok(Some("Relue".into()))
        }
        fn claim(&mut self, pseudo: &str) -> Result<String, LbError> {
            if pseudo.eq_ignore_ascii_case("pris") {
                return Err(LbError::Taken);
            }
            self.0.lock().unwrap().pseudo = Some(pseudo.trim().to_string());
            Ok(pseudo.trim().to_string())
        }
        fn withdraw(&mut self) -> Result<(), LbError> {
            self.0.lock().unwrap().pseudo = None;
            Ok(())
        }
        fn submit(&self, card: &ScoreCard) -> Result<(), LbError> {
            self.0.lock().unwrap().submitted.push(card.clone());
            Ok(())
        }
        fn standings(&self, board: Board, day: u64) -> Result<Standings, LbError> {
            Ok(Standings {
                board,
                day,
                total: 1,
                entries: vec![Entry {
                    rank: 1,
                    pseudo: "Nova".into(),
                    me: true,
                    ..Entry::default()
                }],
                me: None,
            })
        }
        fn site_url(&self) -> String {
            "https://relais".into()
        }
    }

    fn drain(s: &mut LeaderboardService) -> Vec<LeaderboardEvent> {
        s.flush(Duration::from_secs(2));
        std::iter::from_fn(|| s.poll()).collect()
    }

    fn card(play_time: f64) -> ScoreCard {
        score_card(
            &GameSnapshot {
                play_time,
                ..GameSnapshot::default()
            },
            1,
            "terminal",
            "0.18.0",
            0,
        )
    }

    #[test]
    fn act_and_card_follow_the_snapshot() {
        let mut snap = GameSnapshot::default();
        assert_eq!(act_of(&snap), 1);
        snap.crunches = 1;
        assert_eq!(act_of(&snap), 2);
        snap.attractor_awakened = true;
        snap.converged = true;
        snap.awakened = true;
        assert_eq!(act_of(&snap), 5);
        snap.total_all = 1e12;
        snap.progress.completion = 0.25;
        let c = score_card(&snap, 20_000, "android", "0.18.0", 7);
        assert_eq!((c.act, c.progress, c.total_log), (5, 0.25, 12.0));
        assert_eq!(c.daily, None, "pas de Défi du jour joué");
        snap.daily_best = 5e6;
        let c = score_card(&snap, 20_000, "android", "0.18.0", 7);
        assert_eq!(c.daily, Some((20_000, 5e6)));
    }

    #[test]
    fn nothing_is_sent_without_a_pseudo() {
        let log = Arc::new(Mutex::new(Log::default()));
        let mut s = LeaderboardService::spawn(Box::new(Fake(Arc::clone(&log))));
        assert_eq!(s.site_url(), "https://relais");
        s.submit(card(1.0));
        assert!(drain(&mut s).is_empty());
        assert!(log.lock().unwrap().submitted.is_empty());

        s.claim("pris");
        assert_eq!(drain(&mut s), [LeaderboardEvent::Taken]);
        assert_eq!(s.pseudo(), None);
        assert_eq!(s.error(), Some("ce pseudo est déjà pris"));

        s.claim(" Nova ");
        assert_eq!(drain(&mut s), [LeaderboardEvent::Claimed("Nova".into())]);
        assert_eq!(s.pseudo(), Some("Nova"));
        assert_eq!(s.error(), None);
        s.submit(card(2.0));
        assert_eq!(drain(&mut s), [LeaderboardEvent::Submitted]);
        assert_eq!(log.lock().unwrap().submitted.len(), 1);

        s.withdraw();
        assert_eq!(drain(&mut s), [LeaderboardEvent::Withdrawn]);
        assert_eq!(s.pseudo(), None);
    }

    #[test]
    fn pseudo_refresh_and_standings_cache() {
        let log = Arc::new(Mutex::new(Log::default()));
        let mut s = LeaderboardService::spawn(Box::new(Fake(log)));
        s.refresh();
        s.fetch(Board::Daily, 42);
        let ev = drain(&mut s);
        assert_eq!(ev[0], LeaderboardEvent::Pseudo(Some("Relue".into())));
        assert_eq!(s.pseudo(), Some("Relue"));
        let daily = s.standings(Board::Daily).unwrap();
        assert_eq!((daily.day, daily.entries[0].pseudo.as_str()), (42, "Nova"));
        assert!(s.standings(Board::General).is_none());
        assert!(s.fetched_ago().is_some());
        s.forget();
        assert!(!s.standings(Board::Daily).unwrap().entries[0].me);
    }

    #[test]
    fn disabled_service_does_nothing() {
        let mut s = LeaderboardService::disabled();
        assert!(!s.available());
        s.claim("Nova");
        s.fetch(Board::General, 1);
        assert!(!s.busy());
        assert_eq!(s.poll(), None);
    }
}
