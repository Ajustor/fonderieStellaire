//! Mise à jour automatique : vérifie en arrière-plan s'il existe une version
//! plus récente, la télécharge et l'installe ; elle prend effet au prochain
//! lancement. Ne bloque jamais le jeu : tout se passe dans un thread et la
//! présentation lit les [`UpdateEvent`] avec [`UpdateService::poll`].

use std::fmt;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

/// Description d'une version publiée.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseInfo {
    /// Version, avec ou sans `v` (`v0.2.0`, `0.2.0`).
    pub version: String,
    /// Page de la version (pour une installation manuelle).
    pub page_url: String,
    /// Binaire pour la plateforme courante, s'il existe.
    pub binary_url: Option<String>,
    /// Somme SHA-256 du binaire.
    pub checksum_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateError(pub String);

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for UpdateError {}

/// Port : source des mises à jour (implémenté par l'infrastructure).
pub trait UpdateProvider: Send {
    /// Dernière version publiée.
    fn latest(&self) -> Result<ReleaseInfo, UpdateError>;
    /// Raison pour laquelle l'installation automatique est impossible
    /// (version de développement, pas de droits d'écriture…), sinon `None`.
    fn install_blocker(&self, release: &ReleaseInfo) -> Option<String>;
    /// Télécharge, vérifie et remplace l'exécutable courant.
    fn install(&self, release: &ReleaseInfo) -> Result<(), UpdateError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateEvent {
    /// La version installée est la plus récente.
    UpToDate,
    /// Une version plus récente est en cours de téléchargement.
    Downloading { version: String },
    /// La nouvelle version est installée : elle sera utilisée au prochain lancement.
    Installed { version: String },
    /// Une version plus récente existe mais doit être installée à la main.
    ManualOnly {
        version: String,
        reason: String,
        /// Page de la version.
        url: String,
        /// Lien direct du fichier à installer (APK Android), s'il existe.
        download: Option<String>,
    },
    /// Vérification ou installation impossible (hors-ligne…).
    Failed(String),
}

/// Version sémantique simplifiée `majeur.mineur.correctif` (le reste est ignoré).
pub fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let s = s.trim().trim_start_matches(['v', 'V']);
    let core = s.split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    let major = parts.next()??;
    let minor = parts.next().unwrap_or(Some(0))?;
    let patch = parts.next().unwrap_or(Some(0))?;
    Some((major, minor, patch))
}

/// Vrai si `candidate` est strictement plus récente que `current`.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

/// Affichage normalisé d'une version : `v0.2.0`.
pub fn display_version(v: &str) -> String {
    match parse_version(v) {
        Some((a, b, c)) => format!("v{a}.{b}.{c}"),
        None => v.to_string(),
    }
}

fn check_and_install(provider: &dyn UpdateProvider, current: &str, tx: &Sender<UpdateEvent>) {
    let release = match provider.latest() {
        Ok(r) => r,
        Err(e) => {
            let _ = tx.send(UpdateEvent::Failed(e.0));
            return;
        }
    };
    if !is_newer(&release.version, current) {
        let _ = tx.send(UpdateEvent::UpToDate);
        return;
    }
    let version = display_version(&release.version);
    if let Some(reason) = provider.install_blocker(&release) {
        let _ = tx.send(UpdateEvent::ManualOnly {
            version,
            reason,
            url: release.page_url,
            download: release.binary_url,
        });
        return;
    }
    let _ = tx.send(UpdateEvent::Downloading {
        version: version.clone(),
    });
    let event = match provider.install(&release) {
        Ok(()) => UpdateEvent::Installed { version },
        Err(e) => UpdateEvent::Failed(e.0),
    };
    let _ = tx.send(event);
}

pub struct UpdateService {
    current: String,
    rx: Option<Receiver<UpdateEvent>>,
}

impl UpdateService {
    /// Aucune vérification (tests, captures d'écran…).
    pub fn disabled(current: impl Into<String>) -> Self {
        UpdateService {
            current: current.into(),
            rx: None,
        }
    }

    /// Lance la vérification (et l'installation) dans un thread séparé.
    pub fn spawn(provider: Box<dyn UpdateProvider>, current: impl Into<String>) -> Self {
        let current = current.into();
        let (tx, rx) = mpsc::channel();
        let cur = current.clone();
        let spawned = thread::Builder::new()
            .name("mise-a-jour".into())
            .spawn(move || check_and_install(provider.as_ref(), &cur, &tx));
        UpdateService {
            current,
            rx: spawned.ok().map(|_| rx),
        }
    }

    pub fn current_version(&self) -> &str {
        &self.current
    }

    pub fn is_enabled(&self) -> bool {
        self.rx.is_some()
    }

    /// Prochain événement disponible, sans attendre.
    pub fn poll(&mut self) -> Option<UpdateEvent> {
        self.rx.as_ref()?.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    #[test]
    fn versions() {
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("0.10"), Some((0, 10, 0)));
        assert_eq!(parse_version("2.0.0-beta.1"), Some((2, 0, 0)));
        assert_eq!(parse_version("master"), None);
        assert!(is_newer("v0.2.0", "0.1.9"));
        assert!(
            is_newer("0.1.10", "0.1.9"),
            "comparaison numérique, pas textuelle"
        );
        assert!(!is_newer("v0.1.0", "0.1.0"));
        assert!(!is_newer("v0.0.9", "0.1.0"));
        assert!(!is_newer("nimporte", "0.1.0"));
        assert_eq!(display_version("0.2"), "v0.2.0");
    }

    struct FakeProvider {
        version: String,
        blocker: Option<String>,
        install_result: Result<(), UpdateError>,
        installed: Arc<Mutex<bool>>,
    }

    impl UpdateProvider for FakeProvider {
        fn latest(&self) -> Result<ReleaseInfo, UpdateError> {
            if self.version.is_empty() {
                return Err(UpdateError("hors-ligne".into()));
            }
            Ok(ReleaseInfo {
                version: self.version.clone(),
                page_url: "https://exemple/releases".into(),
                binary_url: Some("https://exemple/bin".into()),
                checksum_url: None,
            })
        }
        fn install_blocker(&self, _: &ReleaseInfo) -> Option<String> {
            self.blocker.clone()
        }
        fn install(&self, _: &ReleaseInfo) -> Result<(), UpdateError> {
            *self.installed.lock().unwrap() = true;
            self.install_result.clone()
        }
    }

    fn provider(version: &str) -> (FakeProvider, Arc<Mutex<bool>>) {
        let flag = Arc::new(Mutex::new(false));
        (
            FakeProvider {
                version: version.into(),
                blocker: None,
                install_result: Ok(()),
                installed: flag.clone(),
            },
            flag,
        )
    }

    fn collect(mut s: UpdateService) -> Vec<UpdateEvent> {
        let mut out = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            match s.poll() {
                Some(e) => {
                    let last = !matches!(e, UpdateEvent::Downloading { .. });
                    out.push(e);
                    if last {
                        break;
                    }
                }
                None => thread::sleep(Duration::from_millis(5)),
            }
        }
        out
    }

    #[test]
    fn installs_newer_version() {
        let (p, installed) = provider("v0.2.0");
        let events = collect(UpdateService::spawn(Box::new(p), "0.1.0"));
        assert_eq!(
            events,
            [
                UpdateEvent::Downloading {
                    version: "v0.2.0".into()
                },
                UpdateEvent::Installed {
                    version: "v0.2.0".into()
                }
            ]
        );
        assert!(*installed.lock().unwrap());
    }

    #[test]
    fn up_to_date_installs_nothing() {
        let (p, installed) = provider("v0.1.0");
        assert_eq!(
            collect(UpdateService::spawn(Box::new(p), "0.1.0")),
            [UpdateEvent::UpToDate]
        );
        assert!(!*installed.lock().unwrap());
    }

    #[test]
    fn blocked_install_is_manual_only() {
        let (mut p, installed) = provider("v1.0.0");
        p.blocker = Some("version de développement".into());
        let events = collect(UpdateService::spawn(Box::new(p), "0.1.0"));
        assert!(
            matches!(&events[..], [UpdateEvent::ManualOnly { version, .. }] if version == "v1.0.0")
        );
        assert!(!*installed.lock().unwrap());
    }

    #[test]
    fn failures_are_reported() {
        let (p, _) = provider("");
        assert_eq!(
            collect(UpdateService::spawn(Box::new(p), "0.1.0")),
            [UpdateEvent::Failed("hors-ligne".into())]
        );
        let (mut p, _) = provider("v9.0.0");
        p.install_result = Err(UpdateError("somme incorrecte".into()));
        let events = collect(UpdateService::spawn(Box::new(p), "0.1.0"));
        assert_eq!(
            events.last(),
            Some(&UpdateEvent::Failed("somme incorrecte".into()))
        );
    }

    #[test]
    fn disabled_service_is_silent() {
        let mut s = UpdateService::disabled("0.1.0");
        assert!(!s.is_enabled());
        assert_eq!(s.poll(), None);
        assert_eq!(s.current_version(), "0.1.0");
    }
}
