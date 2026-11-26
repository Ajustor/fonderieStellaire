//! Synchronisation de la partie entre appareils, à travers un relais en ligne.
//!
//! Deux appareils partagent un **code de lien** (affiché en QR code ou à
//! taper). Chacun envoie sa partie au relais avec la révision sur laquelle
//! elle s'appuie ; si un autre appareil a écrit entre-temps, le relais refuse
//! (conflit) : on relit sa version et on garde **la partie la plus jouée**
//! ([`prefer_remote`]). L'autre est remplacée (et gardée en `.bak`).
//!
//! Tout le réseau se fait dans un thread : la présentation envoie des
//! demandes et lit les [`SyncEvent`] avec [`SyncService::poll`]. Le relais et
//! le chiffrement de bout en bout sont dans l'infrastructure (port
//! [`SyncBackend`]).

use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::save_dto::SaveData;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncError(pub String);

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SyncError {}

/// Partie stockée sur le relais, avec sa révision.
#[derive(Clone, Debug, PartialEq)]
pub struct RemoteSave {
    pub rev: u64,
    pub data: SaveData,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PushOutcome {
    /// Enregistrée : nouvelle révision du relais.
    Stored { rev: u64 },
    /// Un autre appareil a écrit depuis `base_rev`.
    Conflict,
}

/// Port : le relais et le lien enregistré sur cet appareil.
pub trait SyncBackend: Send {
    /// Code du lien enregistré, s'il y en a un.
    fn link_code(&self) -> Option<String>;
    /// Crée un nouveau lien (nouveau secret) et l'enregistre. Renvoie son code.
    fn create_link(&mut self) -> Result<String, SyncError>;
    /// Rejoint le lien d'un autre appareil. Renvoie le code normalisé.
    fn join_link(&mut self, code: &str) -> Result<String, SyncError>;
    /// Oublie le lien (la partie reste sur le relais pour les autres appareils).
    fn unlink(&mut self) -> Result<(), SyncError>;
    /// Dernière révision du relais connue de cet appareil (0 : aucune).
    fn synced_rev(&self) -> u64;
    fn set_synced_rev(&mut self, rev: u64) -> Result<(), SyncError>;
    /// Partie du relais (`None` : rien d'enregistré pour ce lien).
    fn pull(&self) -> Result<Option<RemoteSave>, SyncError>;
    /// Envoie la partie si le relais en est toujours à `base_rev`.
    fn push(&self, data: &SaveData, base_rev: u64) -> Result<PushOutcome, SyncError>;
}

/// En cas de conflit : la partie du relais l'emporte si elle a été plus jouée.
///
/// `play_time` n'avance qu'en jeu (pas avec les gains hors-ligne) : un
/// appareil resté fermé ne prend donc jamais le pas sur celui où l'on a joué.
/// À égalité, la plus productive gagne.
pub fn prefer_remote(remote: &SaveData, local: &SaveData) -> bool {
    if remote.play_time != local.play_time {
        return remote.play_time > local.play_time;
    }
    remote.total_all > local.total_all
}

/// Issue d'un rapprochement avec le relais.
#[derive(Clone, Debug, PartialEq)]
pub enum Reconciled {
    /// La partie locale est sur le relais.
    Pushed,
    /// La partie du relais est plus avancée : à reprendre sur cet appareil.
    Adopt(Box<RemoteSave>),
}

/// Envoie `local`, ou décide de reprendre la partie du relais en cas de conflit.
pub fn reconcile(backend: &mut dyn SyncBackend, local: &SaveData) -> Result<Reconciled, SyncError> {
    let mut base = backend.synced_rev();
    for _ in 0..3 {
        match backend.push(local, base)? {
            PushOutcome::Stored { rev } => {
                backend.set_synced_rev(rev)?;
                return Ok(Reconciled::Pushed);
            }
            PushOutcome::Conflict => match backend.pull()? {
                None => base = 0,
                Some(remote) if prefer_remote(&remote.data, local) => {
                    // La révision n'est retenue qu'une fois la partie reprise
                    // (`SyncService::adopted`) : si l'app s'arrête avant, le
                    // prochain envoi refera le même choix.
                    return Ok(Reconciled::Adopt(Box::new(remote)));
                }
                Some(remote) => base = remote.rev,
            },
        }
    }
    Err(SyncError(
        "le relais a changé pendant l'envoi : nouvel essai plus tard".into(),
    ))
}

#[derive(Clone, Debug, PartialEq)]
pub enum SyncEvent {
    /// Nouveau lien créé sur cet appareil.
    Linked {
        code: String,
    },
    /// Cet appareil a rejoint un lien.
    Joined {
        code: String,
    },
    Unlinked,
    /// Partie envoyée au relais.
    Synced,
    /// Partie plus avancée sur le relais : à reprendre, puis appeler
    /// [`SyncService::adopted`] (ou [`SyncService::adopt_failed`]).
    Adopt(Box<RemoteSave>),
    Failed(String),
}

/// État affiché dans l'interface.
#[derive(Clone, Debug, PartialEq)]
pub enum SyncState {
    Idle,
    Working,
    Done { at: Instant },
    Error(String),
}

enum Job {
    Sync(Box<SaveData>),
    Create(Box<SaveData>),
    Join(String, Box<SaveData>),
    Unlink,
    /// La partie du relais a été reprise (ou non) : reprendre les envois.
    Adopted(Option<u64>),
}

pub struct SyncService {
    tx: Option<Sender<Job>>,
    rx: Option<Receiver<SyncEvent>>,
    pending: Arc<AtomicUsize>,
    code: Option<String>,
    state: SyncState,
    /// Révision d'une partie à reprendre (envois suspendus d'ici là).
    awaiting: Option<u64>,
}

impl SyncService {
    /// Synchronisation indisponible (relais non configuré, tests…).
    pub fn disabled() -> Self {
        SyncService {
            tx: None,
            rx: None,
            pending: Arc::new(AtomicUsize::new(0)),
            code: None,
            state: SyncState::Idle,
            awaiting: None,
        }
    }

    pub fn spawn(backend: Box<dyn SyncBackend>) -> Self {
        let code = backend.link_code();
        let (job_tx, job_rx) = mpsc::channel();
        let (ev_tx, ev_rx) = mpsc::channel();
        let pending = Arc::new(AtomicUsize::new(0));
        let p = Arc::clone(&pending);
        let spawned = thread::Builder::new()
            .name("synchro".into())
            .spawn(move || worker(backend, &job_rx, &ev_tx, &p));
        match spawned {
            Ok(_) => SyncService {
                tx: Some(job_tx),
                rx: Some(ev_rx),
                pending,
                code,
                state: SyncState::Idle,
                awaiting: None,
            },
            Err(_) => Self::disabled(),
        }
    }

    /// Un relais est configuré dans cette version.
    pub fn available(&self) -> bool {
        self.tx.is_some()
    }

    /// Code du lien de cet appareil.
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    pub fn linked(&self) -> bool {
        self.code.is_some()
    }

    pub fn state(&self) -> &SyncState {
        &self.state
    }

    /// Une demande est en cours.
    pub fn busy(&self) -> bool {
        self.pending.load(Ordering::SeqCst) > 0
    }

    fn send(&mut self, job: Job) {
        if let Some(tx) = &self.tx {
            self.pending.fetch_add(1, Ordering::SeqCst);
            if tx.send(job).is_err() {
                self.pending.fetch_sub(1, Ordering::SeqCst);
                self.tx = None;
            } else {
                self.state = SyncState::Working;
            }
        }
    }

    /// Envoie la partie (si l'appareil est lié et n'attend pas une reprise).
    pub fn sync(&mut self, data: SaveData) {
        if self.linked() && self.awaiting.is_none() {
            self.send(Job::Sync(Box::new(data)));
        }
    }

    /// Crée un lien et y envoie la partie.
    pub fn create_link(&mut self, data: SaveData) {
        self.awaiting = None;
        self.send(Job::Create(Box::new(data)));
    }

    /// Rejoint le lien `code` ; la partie la plus jouée est gardée.
    pub fn join(&mut self, code: &str, data: SaveData) {
        self.awaiting = None;
        self.send(Job::Join(code.to_string(), Box::new(data)));
    }

    pub fn unlink(&mut self) {
        self.awaiting = None;
        self.send(Job::Unlink);
    }

    /// La partie annoncée par [`SyncEvent::Adopt`] a été reprise.
    pub fn adopted(&mut self) {
        if let Some(rev) = self.awaiting.take() {
            self.send(Job::Adopted(Some(rev)));
        }
    }

    /// La reprise a échoué : les envois reprennent, le choix sera refait.
    pub fn adopt_failed(&mut self) {
        if self.awaiting.take().is_some() {
            self.send(Job::Adopted(None));
        }
    }

    /// Une partie du relais attend d'être reprise.
    pub fn awaiting_adoption(&self) -> bool {
        self.awaiting.is_some()
    }

    pub fn poll(&mut self) -> Option<SyncEvent> {
        let ev = self.rx.as_ref()?.try_recv().ok()?;
        match &ev {
            SyncEvent::Linked { code } | SyncEvent::Joined { code } => {
                self.code = Some(code.clone());
                self.state = SyncState::Done { at: Instant::now() };
            }
            SyncEvent::Unlinked => {
                self.code = None;
                self.state = SyncState::Idle;
            }
            SyncEvent::Synced => self.state = SyncState::Done { at: Instant::now() },
            SyncEvent::Adopt(remote) => {
                self.awaiting = Some(remote.rev);
                self.state = SyncState::Done { at: Instant::now() };
            }
            SyncEvent::Failed(msg) => self.state = SyncState::Error(msg.clone()),
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

fn worker(
    mut backend: Box<dyn SyncBackend>,
    jobs: &Receiver<Job>,
    events: &Sender<SyncEvent>,
    pending: &AtomicUsize,
) {
    // Après une annonce `Adopt`, les envois sont ignorés jusqu'à `Adopted` :
    // ils viennent d'une partie qui va être remplacée.
    let mut blocked = false;
    while let Ok(first) = jobs.recv() {
        let mut batch = vec![first];
        batch.extend(jobs.try_iter());
        let last_sync = batch.iter().rposition(|j| matches!(j, Job::Sync(_)));
        for (i, job) in batch.into_iter().enumerate() {
            let ev = match job {
                // Seul le dernier envoi d'une rafale compte.
                Job::Sync(_) if Some(i) != last_sync || blocked => None,
                Job::Sync(data) => Some(sync_event(backend.as_mut(), &data, &mut blocked)),
                Job::Create(data) => {
                    blocked = false;
                    Some(match backend.create_link() {
                        Ok(code) => {
                            let ev = sync_event(backend.as_mut(), &data, &mut blocked);
                            let _ = events.send(SyncEvent::Linked { code });
                            ev
                        }
                        Err(e) => SyncEvent::Failed(e.0),
                    })
                }
                Job::Join(code, data) => {
                    blocked = false;
                    Some(match backend.join_link(&code) {
                        Ok(code) => {
                            let ev = sync_event(backend.as_mut(), &data, &mut blocked);
                            let _ = events.send(SyncEvent::Joined { code });
                            ev
                        }
                        Err(e) => SyncEvent::Failed(e.0),
                    })
                }
                Job::Unlink => {
                    blocked = false;
                    Some(match backend.unlink() {
                        Ok(()) => SyncEvent::Unlinked,
                        Err(e) => SyncEvent::Failed(e.0),
                    })
                }
                Job::Adopted(rev) => {
                    blocked = false;
                    rev.and_then(|r| backend.set_synced_rev(r).err())
                        .map(|e| SyncEvent::Failed(e.0))
                }
            };
            if let Some(ev) = ev {
                if events.send(ev).is_err() {
                    return;
                }
            }
            pending.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

fn sync_event(backend: &mut dyn SyncBackend, data: &SaveData, blocked: &mut bool) -> SyncEvent {
    match reconcile(backend, data) {
        Ok(Reconciled::Pushed) => SyncEvent::Synced,
        Ok(Reconciled::Adopt(remote)) => {
            *blocked = true;
            SyncEvent::Adopt(remote)
        }
        Err(e) => SyncEvent::Failed(e.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Relais en mémoire partagé entre plusieurs « appareils ».
    #[derive(Default)]
    struct Relay {
        slot: Option<RemoteSave>,
        pushes: u32,
    }

    struct Device {
        relay: Arc<Mutex<Relay>>,
        code: Option<String>,
        rev: u64,
    }

    impl Device {
        fn new(relay: &Arc<Mutex<Relay>>) -> Self {
            Device {
                relay: Arc::clone(relay),
                code: Some("LIEN".into()),
                rev: 0,
            }
        }
    }

    impl SyncBackend for Device {
        fn link_code(&self) -> Option<String> {
            self.code.clone()
        }
        fn create_link(&mut self) -> Result<String, SyncError> {
            self.code = Some("LIEN".into());
            self.rev = 0;
            Ok("LIEN".into())
        }
        fn join_link(&mut self, code: &str) -> Result<String, SyncError> {
            if code != "LIEN" {
                return Err(SyncError("code invalide".into()));
            }
            self.code = Some(code.into());
            self.rev = 0;
            Ok(code.into())
        }
        fn unlink(&mut self) -> Result<(), SyncError> {
            self.code = None;
            Ok(())
        }
        fn synced_rev(&self) -> u64 {
            self.rev
        }
        fn set_synced_rev(&mut self, rev: u64) -> Result<(), SyncError> {
            self.rev = rev;
            Ok(())
        }
        fn pull(&self) -> Result<Option<RemoteSave>, SyncError> {
            Ok(self.relay.lock().unwrap().slot.clone())
        }
        fn push(&self, data: &SaveData, base_rev: u64) -> Result<PushOutcome, SyncError> {
            let mut r = self.relay.lock().unwrap();
            let current = r.slot.as_ref().map_or(0, |s| s.rev);
            if current != base_rev {
                return Ok(PushOutcome::Conflict);
            }
            r.pushes += 1;
            r.slot = Some(RemoteSave {
                rev: current + 1,
                data: data.clone(),
            });
            Ok(PushOutcome::Stored { rev: current + 1 })
        }
    }

    fn save(play_time: f64, total_all: f64) -> SaveData {
        SaveData {
            play_time,
            total_all,
            ..SaveData::default()
        }
    }

    #[test]
    fn the_most_played_save_wins() {
        assert!(prefer_remote(&save(100.0, 1.0), &save(50.0, 9.0)));
        assert!(!prefer_remote(&save(50.0, 9.0), &save(100.0, 1.0)));
        // Gains hors-ligne seulement : à temps de jeu égal, la plus productive.
        assert!(prefer_remote(&save(50.0, 9.0), &save(50.0, 1.0)));
        assert!(!prefer_remote(&save(50.0, 1.0), &save(50.0, 1.0)));
    }

    #[test]
    fn two_devices_take_turns() {
        let relay = Arc::new(Mutex::new(Relay::default()));
        let mut pc = Device::new(&relay);
        let mut phone = Device::new(&relay);

        // Le PC joue et envoie.
        assert_eq!(reconcile(&mut pc, &save(10.0, 1.0)), Ok(Reconciled::Pushed));
        assert_eq!(reconcile(&mut pc, &save(60.0, 5.0)), Ok(Reconciled::Pushed));
        // Le téléphone s'ouvre avec une vieille partie (plus de gains hors-ligne) :
        // il reprend celle du PC.
        let Ok(Reconciled::Adopt(remote)) = reconcile(&mut phone, &save(10.0, 50.0)) else {
            panic!("le téléphone doit reprendre la partie du PC");
        };
        assert_eq!(remote.data.play_time, 60.0);
        phone.set_synced_rev(remote.rev).unwrap();
        // Il joue, envoie : le PC, resté fermé, reprend à son tour.
        assert_eq!(
            reconcile(&mut phone, &save(90.0, 8.0)),
            Ok(Reconciled::Pushed)
        );
        assert!(matches!(
            reconcile(&mut pc, &save(60.0, 80.0)),
            Ok(Reconciled::Adopt(_))
        ));
    }

    #[test]
    fn a_more_played_local_save_overwrites_the_relay() {
        let relay = Arc::new(Mutex::new(Relay::default()));
        let mut a = Device::new(&relay);
        let mut b = Device::new(&relay);
        reconcile(&mut a, &save(10.0, 1.0)).unwrap();
        assert_eq!(reconcile(&mut b, &save(500.0, 1.0)), Ok(Reconciled::Pushed));
        let r = relay.lock().unwrap();
        assert_eq!(r.slot.as_ref().unwrap().data.play_time, 500.0);
        assert_eq!(r.slot.as_ref().unwrap().rev, 2);
    }

    #[test]
    fn service_runs_in_background_and_waits_for_adoption() {
        let relay = Arc::new(Mutex::new(Relay::default()));
        let mut other = Device::new(&relay);
        reconcile(&mut other, &save(300.0, 1.0)).unwrap();

        let mut svc = SyncService::spawn(Box::new(Device::new(&relay)));
        assert!(svc.available() && svc.linked());
        svc.sync(save(5.0, 1.0));
        svc.flush(Duration::from_secs(2));
        let Some(SyncEvent::Adopt(remote)) = svc.poll() else {
            panic!("la partie du relais est plus jouée");
        };
        assert_eq!(remote.data.play_time, 300.0);
        // Tant que la reprise n'est pas faite, rien n'est envoyé.
        svc.sync(save(6.0, 1.0));
        assert!(!svc.busy());
        svc.adopted();
        svc.sync(save(301.0, 2.0));
        svc.flush(Duration::from_secs(2));
        assert_eq!(svc.poll(), Some(SyncEvent::Synced));
        assert_eq!(relay.lock().unwrap().slot.as_ref().unwrap().rev, 2);
    }

    #[test]
    fn bursts_end_with_the_latest_save_and_errors_are_reported() {
        let relay = Arc::new(Mutex::new(Relay::default()));
        let mut dev = Device::new(&relay);
        dev.code = None;
        let mut svc = SyncService::spawn(Box::new(dev));
        assert!(!svc.linked());
        svc.join("FAUX", save(1.0, 1.0));
        svc.flush(Duration::from_secs(2));
        assert!(matches!(svc.poll(), Some(SyncEvent::Failed(_))));
        svc.create_link(save(1.0, 1.0));
        svc.flush(Duration::from_secs(2));
        assert_eq!(
            svc.poll(),
            Some(SyncEvent::Linked {
                code: "LIEN".into()
            })
        );
        assert_eq!(svc.poll(), Some(SyncEvent::Synced));
        for t in 2..20 {
            svc.sync(save(f64::from(t), 1.0));
        }
        svc.flush(Duration::from_secs(2));
        assert!(relay.lock().unwrap().pushes <= 19);
        assert_eq!(
            relay.lock().unwrap().slot.as_ref().unwrap().data.play_time,
            19.0
        );
        svc.unlink();
        svc.flush(Duration::from_secs(2));
        while let Some(ev) = svc.poll() {
            if ev == SyncEvent::Unlinked {
                break;
            }
        }
        assert!(!svc.linked());
    }
}
