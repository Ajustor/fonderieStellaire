//! Mises à jour depuis les releases GitHub : API `releases/latest`, binaire
//! brut de la plateforme + sa somme SHA-256, remplacement de l'exécutable.

use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use application::{ReleaseInfo, UpdateError, UpdateProvider};
use sha2::{Digest, Sha256};

const USER_AGENT: &str = "fonderie-stellaire-updater";
/// Taille maximale acceptée pour un binaire téléchargé.
const MAX_BINARY: u64 = 100 * 1024 * 1024;

/// Triple de la plateforme courante, si des binaires sont publiés pour elle.
pub fn current_target() -> Option<&'static str> {
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some("x86_64-pc-windows-msvc")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("x86_64-unknown-linux-gnu")
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some("x86_64-apple-darwin")
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("aarch64-apple-darwin")
    } else {
        None
    }
}

/// Vrai dans un conteneur Docker ou Podman.
fn in_container() -> bool {
    Path::new("/.dockerenv").exists() || Path::new("/run/.containerenv").exists()
}

/// Nom de l'asset « binaire brut » publié pour une plateforme.
pub fn asset_name(target: &str) -> String {
    let ext = if target.contains("windows") {
        ".exe"
    } else {
        ""
    };
    format!("fonderie-stellaire-{target}{ext}")
}

fn err(msg: impl Into<String>) -> UpdateError {
    UpdateError(msg.into())
}

/// Extrait les informations utiles de la réponse JSON de l'API GitHub.
pub fn parse_release(json: &str, target: Option<&str>) -> Result<ReleaseInfo, UpdateError> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| err(format!("réponse illisible : {e}")))?;
    let version = v["tag_name"]
        .as_str()
        .ok_or_else(|| err("réponse sans numéro de version"))?
        .to_string();
    let page_url = v["html_url"].as_str().unwrap_or_default().to_string();
    let find = |name: &str| {
        v["assets"].as_array().and_then(|assets| {
            assets
                .iter()
                .find(|a| a["name"].as_str() == Some(name))
                .and_then(|a| a["browser_download_url"].as_str())
                .map(str::to_string)
        })
    };
    let (binary_url, checksum_url) = match target {
        Some(t) => {
            let name = asset_name(t);
            (find(&name), find(&format!("{name}.sha256")))
        }
        None => (None, None),
    };
    Ok(ReleaseInfo {
        version,
        page_url,
        binary_url,
        checksum_url,
    })
}

/// Une release sans le binaire d'une plateforme pourtant publiée est en cours
/// de publication (les binaires arrivent quelques minutes après la création de
/// la release) : erreur passagère, la vérification sera refaite au prochain
/// lancement. Sans plateforme connue, la release est rendue telle quelle.
pub fn require_binary(
    release: ReleaseInfo,
    target: Option<&str>,
) -> Result<ReleaseInfo, UpdateError> {
    if target.is_some() && release.binary_url.is_none() {
        return Err(err(format!(
            "version {} en cours de publication : nouvel essai au prochain lancement",
            release.version
        )));
    }
    Ok(release)
}

/// Vérifie `bytes` contre un fichier `.sha256` (« <hex>  <nom> » ou « <hex> »).
pub fn verify_sha256(bytes: &[u8], checksum_file: &str) -> Result<(), UpdateError> {
    let expected = checksum_file
        .split_whitespace()
        .next()
        .ok_or_else(|| err("somme de contrôle vide"))?
        .to_ascii_lowercase();
    let actual: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if actual == expected {
        Ok(())
    } else {
        Err(err("somme SHA-256 incorrecte, mise à jour annulée"))
    }
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

/// Remplace l'exécutable `exe` par `bytes`. Sous Windows, l'exécutable en cours
/// ne peut pas être écrasé mais peut être renommé : l'ancien devient `.old`
/// (supprimé au lancement suivant).
pub fn replace_executable(exe: &Path, bytes: &[u8]) -> io::Result<()> {
    let new = sibling(exe, ".new");
    fs::write(&new, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&new, fs::Permissions::from_mode(0o755))?;
    }
    if cfg!(windows) {
        let old = sibling(exe, ".old");
        let _ = fs::remove_file(&old);
        fs::rename(exe, &old)?;
        if let Err(e) = fs::rename(&new, exe) {
            // On remet l'ancien en place pour ne jamais laisser le jeu cassé.
            let _ = fs::rename(&old, exe);
            let _ = fs::remove_file(&new);
            return Err(e);
        }
    } else {
        fs::rename(&new, exe)?;
    }
    Ok(())
}

/// Vrai si l'exécutable vient d'un `cargo build` (dossier `target/`).
pub fn is_dev_build(exe: &Path) -> bool {
    exe.components()
        .any(|c| matches!(c, Component::Normal(n) if n == "target"))
}

pub struct GitHubUpdater {
    repo: String,
    exe: Option<PathBuf>,
    agent: ureq::Agent,
}

impl GitHubUpdater {
    /// `repo` au format `propriétaire/dépôt`.
    pub fn new(repo: impl Into<String>) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(8))
            .timeout(Duration::from_secs(120))
            .user_agent(USER_AGENT)
            .build();
        GitHubUpdater {
            repo: repo.into(),
            exe: std::env::current_exe().ok(),
            agent,
        }
    }

    /// Supprime l'ancien exécutable laissé par une mise à jour précédente.
    pub fn cleanup_previous(&self) {
        if let Some(exe) = &self.exe {
            let _ = fs::remove_file(sibling(exe, ".old"));
            let _ = fs::remove_file(sibling(exe, ".new"));
        }
    }

    fn get(&self, url: &str) -> Result<ureq::Response, UpdateError> {
        self.agent
            .get(url)
            .set("Accept", "application/vnd.github+json")
            .call()
            .map_err(|e| match e {
                ureq::Error::Status(404, _) => err("aucune version publiée pour l'instant"),
                ureq::Error::Status(code, _) => err(format!("GitHub a répondu {code}")),
                ureq::Error::Transport(_) => err("pas de connexion à GitHub"),
            })
    }

    fn download(&self, url: &str) -> Result<Vec<u8>, UpdateError> {
        let mut bytes = Vec::new();
        self.get(url)?
            .into_reader()
            .take(MAX_BINARY)
            .read_to_end(&mut bytes)
            .map_err(|e| err(format!("téléchargement interrompu : {e}")))?;
        Ok(bytes)
    }
}

impl UpdateProvider for GitHubUpdater {
    fn latest(&self) -> Result<ReleaseInfo, UpdateError> {
        let url = format!("https://api.github.com/repos/{}/releases/latest", self.repo);
        let body = self
            .get(&url)?
            .into_string()
            .map_err(|e| err(format!("réponse illisible : {e}")))?;
        let target = current_target();
        require_binary(parse_release(&body, target)?, target)
    }

    fn install_blocker(&self, release: &ReleaseInfo) -> Option<String> {
        let Some(exe) = &self.exe else {
            return Some("emplacement du jeu inconnu".into());
        };
        if current_target().is_none() || release.binary_url.is_none() {
            return Some("pas de binaire pour cette plateforme".into());
        }
        if is_dev_build(exe) {
            return Some("version de développement (cargo)".into());
        }
        // Dans un conteneur, on met à jour l'image, pas l'exécutable.
        if in_container() {
            return Some("image Docker : mets-la à jour avec `docker pull`".into());
        }
        // Peut-on écrire à côté de l'exécutable ?
        let dir = exe.parent()?;
        let probe = dir.join(".fonderie-maj-test");
        match fs::write(&probe, b"") {
            Ok(()) => {
                let _ = fs::remove_file(&probe);
                None
            }
            Err(_) => Some(format!("pas les droits d'écriture sur {}", dir.display())),
        }
    }

    fn install(&self, release: &ReleaseInfo) -> Result<(), UpdateError> {
        let exe = self
            .exe
            .as_ref()
            .ok_or_else(|| err("emplacement du jeu inconnu"))?;
        let bin_url = release
            .binary_url
            .as_ref()
            .ok_or_else(|| err("pas de binaire pour cette plateforme"))?;
        let sum_url = release
            .checksum_url
            .as_ref()
            .ok_or_else(|| err("somme de contrôle absente de la version"))?;
        let checksum = String::from_utf8_lossy(&self.download(sum_url)?).into_owned();
        let bytes = self.download(bin_url)?;
        if bytes.is_empty() {
            return Err(err("binaire téléchargé vide"));
        }
        verify_sha256(&bytes, &checksum)?;
        replace_executable(exe, &bytes).map_err(|e| err(format!("installation impossible : {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    const JSON: &str = r#"{
        "tag_name": "v0.2.0",
        "html_url": "https://github.com/o/r/releases/tag/v0.2.0",
        "assets": [
            {"name": "fonderie-stellaire-x86_64-unknown-linux-gnu", "browser_download_url": "https://dl/linux"},
            {"name": "fonderie-stellaire-x86_64-unknown-linux-gnu.sha256", "browser_download_url": "https://dl/linux.sha256"},
            {"name": "fonderie-stellaire-x86_64-pc-windows-msvc.exe", "browser_download_url": "https://dl/win"},
            {"name": "fonderie-stellaire-x86_64-pc-windows-msvc.exe.sha256", "browser_download_url": "https://dl/win.sha256"}
        ]
    }"#;

    #[test]
    fn picks_platform_assets() {
        let r = parse_release(JSON, Some("x86_64-pc-windows-msvc")).unwrap();
        assert_eq!(r.version, "v0.2.0");
        assert_eq!(r.binary_url.as_deref(), Some("https://dl/win"));
        assert_eq!(r.checksum_url.as_deref(), Some("https://dl/win.sha256"));
        let r = parse_release(JSON, Some("x86_64-unknown-linux-gnu")).unwrap();
        assert_eq!(r.binary_url.as_deref(), Some("https://dl/linux"));
        let r = parse_release(JSON, Some("aarch64-apple-darwin")).unwrap();
        assert_eq!(r.binary_url, None);
        assert!(parse_release("{}", None).is_err());
        assert!(parse_release("pas du json", None).is_err());
    }

    /// v0.17.1 : la CI Android a publié la release avec seule l'APK, avant les
    /// binaires des ordinateurs. Une telle release est « en cours de
    /// publication » : on réessaiera, sans proposer de téléchargement manuel.
    #[test]
    fn a_release_without_our_binary_yet_is_not_ready() {
        let apk_only = r#"{
            "tag_name": "v0.17.1",
            "html_url": "https://github.com/o/r/releases/tag/v0.17.1",
            "assets": [
                {"name": "fonderie-stellaire.apk", "browser_download_url": "https://dl/apk"},
                {"name": "fonderie-stellaire.apk.sha256", "browser_download_url": "https://dl/apk.sha256"}
            ]
        }"#;
        let target = Some("x86_64-pc-windows-msvc");
        let r = parse_release(apk_only, target).unwrap();
        let e = require_binary(r, target).unwrap_err();
        assert!(e.0.contains("en cours de publication"), "{}", e.0);
        // Binaire présent : rien ne change.
        let r = parse_release(JSON, target).unwrap();
        assert_eq!(require_binary(r.clone(), target), Ok(r));
        // Plateforme sans binaire publié : la page reste proposée.
        let r = parse_release(apk_only, None).unwrap();
        assert_eq!(require_binary(r.clone(), None), Ok(r));
    }

    #[test]
    fn checksum_verification() {
        // sha256("abc")
        let sum = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  fichier";
        assert!(verify_sha256(b"abc", sum).is_ok());
        assert!(verify_sha256(b"abd", sum).is_err());
        assert!(verify_sha256(b"abc", &sum.to_uppercase()).is_ok());
        assert!(verify_sha256(b"abc", "").is_err());
    }

    /// Appel réseau réel : `cargo test -p infrastructure -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn live_github_check() {
        let u = GitHubUpdater::new("Ajustor/fonderieStellaire");
        println!("{:?}", u.latest());
    }

    #[test]
    fn dev_builds_are_detected() {
        assert!(is_dev_build(Path::new(
            "/home/a/jeu/target/release/fonderie-stellaire"
        )));
        assert!(!is_dev_build(Path::new(
            "/home/a/.local/bin/fonderie-stellaire"
        )));
    }

    #[test]
    fn replaces_executable_and_keeps_old_on_windows() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("fonderie-maj-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("jeu.exe");
        fs::write(&exe, b"ancienne version").unwrap();
        replace_executable(&exe, b"nouvelle version").unwrap();
        assert_eq!(fs::read(&exe).unwrap(), b"nouvelle version");
        assert!(!sibling(&exe, ".new").exists());
        if cfg!(windows) {
            assert_eq!(
                fs::read(sibling(&exe, ".old")).unwrap(),
                b"ancienne version"
            );
        }
        let _ = fs::remove_dir_all(dir);
    }
}
