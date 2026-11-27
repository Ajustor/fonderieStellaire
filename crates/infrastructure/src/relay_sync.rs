//! Synchronisation par le relais en ligne (`crates/relay`).
//!
//! Le **code de lien** (25 caractères, base 32 de Crockford, dont un de
//! contrôle) porte un secret de 120 bits tiré au hasard. Tout en dérive :
//!
//! - le jeton d'accès envoyé au relais (`Authorization: Bearer …`), qui
//!   désigne aussi l'emplacement de la partie ;
//! - les clés de chiffrement et de signature : le relais ne reçoit que des
//!   octets chiffrés et ne connaît jamais le code.
//!
//! La partie envoyée est la sauvegarde habituelle (déjà chiffrée avec la clé
//! du jeu), scellée une seconde fois avec la clé du lien.
//!
//! Le lien est gardé dans `sync.json`, à côté de la sauvegarde.

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use application::{PushOutcome, RemoteSave, SaveData, SyncBackend, SyncError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::json_repository::{decode_save, encode_save, DecodeError};
use crate::save_crypto;

/// Préfixe du QR code (caractères du mode alphanumérique des QR codes).
pub const QR_PREFIX: &str = "FONDERIE:";
const SECRET_LEN: usize = 15;
const CODE_CHARS: usize = 25;
/// Base 32 de Crockford : ni I, L, O ni U.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// Taille maximale d'une partie reçue.
const MAX_BODY: u64 = 2 * 1024 * 1024;
const LINK_FILE: &str = "sync.json";

/// Relais officiel du jeu.
pub const DEFAULT_RELAY: &str = "https://fonderie-stellaire-sync.darthoit.eu";

/// Adresse du relais : variable d'environnement `FONDERIE_SYNC_URL` (tests,
/// relais personnel), sinon celle donnée à la compilation, sinon le relais
/// officiel.
pub fn relay_url() -> Option<String> {
    let runtime = std::env::var("FONDERIE_SYNC_URL").ok();
    let built = option_env!("FONDERIE_SYNC_URL").map(str::to_string);
    runtime
        .into_iter()
        .chain(built)
        .chain(Some(DEFAULT_RELAY.to_string()))
        .map(|u| u.trim().trim_end_matches('/').to_string())
        .find(|u| u.starts_with("https://") || u.starts_with("http://"))
}

fn check_char(secret: &[u8]) -> u8 {
    ALPHABET[(Sha256::digest(secret)[0] & 31) as usize]
}

/// Code lisible : `XXXXX-XXXXX-XXXXX-XXXXX-XXXXX`.
pub fn format_code(secret: &[u8; SECRET_LEN]) -> String {
    let mut chars = Vec::with_capacity(CODE_CHARS);
    for chunk in secret.chunks(5) {
        let mut n = 0u64;
        for &b in chunk {
            n = (n << 8) | u64::from(b);
        }
        for i in (0..8).rev() {
            chars.push(ALPHABET[((n >> (5 * i)) & 31) as usize]);
        }
    }
    chars.push(check_char(secret));
    chars
        .chunks(5)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect::<Vec<_>>()
        .join("-")
}

/// Relit un code tapé ou scanné (tirets, espaces, minuscules, `O`/`0`,
/// `I`/`L`/`1` tolérés ; préfixe du QR code accepté).
pub fn parse_code(text: &str) -> Option<[u8; SECRET_LEN]> {
    let text = text.rsplit(':').next().unwrap_or(text);
    let mut values = Vec::with_capacity(CODE_CHARS);
    for c in text.chars() {
        let c = match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            c if c.is_ascii_alphanumeric() => c,
            '-' | ' ' | '_' | '.' => continue,
            _ => return None,
        };
        values.push(ALPHABET.iter().position(|&a| a == c as u8)? as u8);
    }
    if values.len() != CODE_CHARS {
        return None;
    }
    let mut secret = [0u8; SECRET_LEN];
    for (g, group) in values[..CODE_CHARS - 1].chunks(8).enumerate() {
        let n = group.iter().fold(0u64, |n, &v| (n << 5) | u64::from(v));
        for i in 0..5 {
            secret[g * 5 + i] = (n >> (8 * (4 - i))) as u8;
        }
    }
    (ALPHABET[values[CODE_CHARS - 1] as usize] == check_char(&secret)).then_some(secret)
}

fn derive(label: &str, secret: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"fonderie-sync/");
    h.update(label.as_bytes());
    h.update(secret);
    h.finalize().into()
}

/// Jeton du classement pour le lien `code` (distinct du jeton de la partie).
pub fn leaderboard_token(code: &str) -> Option<String> {
    parse_code(code).map(|s| hex(&derive("leaderboard", &s)))
}

/// Code du lien enregistré dans `dir`, s'il y en a un.
pub fn stored_code(dir: &Path) -> Option<String> {
    let text = fs::read_to_string(dir.join(LINK_FILE)).ok()?;
    let file = serde_json::from_str::<LinkFile>(&text).ok()?;
    parse_code(&file.code).map(|s| format_code(&s))
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
struct LinkFile {
    code: String,
    rev: u64,
}

struct Link {
    code: String,
    rev: u64,
    token: String,
    enc: [u8; 32],
    mac: [u8; 32],
}

impl Link {
    fn new(secret: &[u8; SECRET_LEN], rev: u64) -> Self {
        Link {
            code: format_code(secret),
            rev,
            token: hex(&derive("auth", secret)),
            enc: derive("enc", secret),
            mac: derive("mac", secret),
        }
    }
}

pub struct RelaySync {
    url: String,
    path: PathBuf,
    link: Option<Link>,
    agent: ureq::Agent,
}

fn err(msg: impl Into<String>) -> SyncError {
    SyncError(msg.into())
}

fn io_err(action: &str, e: &io::Error) -> SyncError {
    err(format!("{action} impossible : {e}"))
}

fn random_bytes<const N: usize>() -> Result<[u8; N], SyncError> {
    let mut buf = [0u8; N];
    getrandom::getrandom(&mut buf)
        .map_err(|e| err(format!("tirage aléatoire impossible : {e}")))?;
    Ok(buf)
}

impl RelaySync {
    /// Synchronisation avec le relais `url` ; le lien est gardé dans `dir`.
    pub fn new(url: impl Into<String>, dir: &Path) -> Self {
        let path = dir.join(LINK_FILE);
        let link = fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str::<LinkFile>(&t).ok())
            .and_then(|f| parse_code(&f.code).map(|s| Link::new(&s, f.rev)));
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(8))
            .timeout(Duration::from_secs(20))
            .user_agent(concat!("fonderie-stellaire/", env!("CARGO_PKG_VERSION")))
            .build();
        RelaySync {
            url: url.into(),
            path,
            link,
            agent,
        }
    }

    /// Relais configuré dans cette version, lien gardé dans `dir`.
    pub fn from_env(dir: &Path) -> Option<Self> {
        relay_url().map(|u| Self::new(u, dir))
    }

    fn persist(&self) -> Result<(), SyncError> {
        let Some(link) = &self.link else {
            return match fs::remove_file(&self.path) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => Err(io_err("oubli du lien", &e)),
                _ => Ok(()),
            };
        };
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).map_err(|e| io_err("création du dossier", &e))?;
        }
        let file = LinkFile {
            code: link.code.clone(),
            rev: link.rev,
        };
        let json = serde_json::to_string(&file).map_err(|e| err(e.to_string()))?;
        let mut tmp = self.path.as_os_str().to_owned();
        tmp.push(".tmp");
        fs::write(&tmp, json).map_err(|e| io_err("enregistrement du lien", &e))?;
        fs::rename(&tmp, &self.path).map_err(|e| io_err("enregistrement du lien", &e))
    }

    fn link(&self) -> Result<&Link, SyncError> {
        self.link
            .as_ref()
            .ok_or_else(|| err("cet appareil n'est lié à aucun autre"))
    }

    fn endpoint(&self) -> String {
        format!("{}/v1/save", self.url)
    }
}

fn transport(e: &ureq::Transport) -> SyncError {
    err(format!("relais injoignable (hors ligne ?) : {e}"))
}

fn rev_of(resp: &ureq::Response) -> Result<u64, SyncError> {
    resp.header("X-Rev")
        .and_then(|v| v.trim().parse().ok())
        .ok_or_else(|| err("réponse du relais incomplète"))
}

impl SyncBackend for RelaySync {
    fn link_code(&self) -> Option<String> {
        self.link.as_ref().map(|l| l.code.clone())
    }

    fn create_link(&mut self) -> Result<String, SyncError> {
        let secret = random_bytes::<SECRET_LEN>()?;
        self.link = Some(Link::new(&secret, 0));
        self.persist()?;
        Ok(format_code(&secret))
    }

    fn join_link(&mut self, code: &str) -> Result<String, SyncError> {
        let secret = parse_code(code)
            .ok_or_else(|| err("code de lien invalide : vérifie les 25 caractères"))?;
        self.link = Some(Link::new(&secret, 0));
        self.persist()?;
        Ok(format_code(&secret))
    }

    fn unlink(&mut self) -> Result<(), SyncError> {
        self.link = None;
        self.persist()
    }

    fn synced_rev(&self) -> u64 {
        self.link.as_ref().map_or(0, |l| l.rev)
    }

    fn set_synced_rev(&mut self, rev: u64) -> Result<(), SyncError> {
        if let Some(link) = self.link.as_mut() {
            link.rev = rev;
        }
        self.persist()
    }

    fn pull(&self) -> Result<Option<RemoteSave>, SyncError> {
        let link = self.link()?;
        let resp = match self
            .agent
            .get(&self.endpoint())
            .set("Authorization", &format!("Bearer {}", link.token))
            .call()
        {
            Ok(r) => r,
            Err(ureq::Error::Status(404, _)) => return Ok(None),
            Err(ureq::Error::Status(code, _)) => {
                return Err(err(format!("le relais a refusé la lecture ({code})")))
            }
            Err(ureq::Error::Transport(t)) => return Err(transport(&t)),
        };
        let rev = rev_of(&resp)?;
        let mut sealed = Vec::new();
        resp.into_reader()
            .take(MAX_BODY)
            .read_to_end(&mut sealed)
            .map_err(|e| io_err("lecture de la partie", &e))?;
        let plain = save_crypto::open(&link.enc, &link.mac, &sealed)
            .map_err(|_| err("partie du relais illisible (code de lien différent ?)"))?;
        let text = String::from_utf8(plain).map_err(|_| err("partie du relais illisible"))?;
        let data = decode_save(&text).map_err(|e| match e {
            DecodeError::Tampered => {
                err("partie du relais refusée : elle vient d'une autre version du jeu")
            }
            DecodeError::Unreadable => err("partie du relais illisible"),
        })?;
        Ok(Some(RemoteSave { rev, data }))
    }

    fn push(&self, data: &SaveData, base_rev: u64) -> Result<PushOutcome, SyncError> {
        let link = self.link()?;
        let text = encode_save(data).map_err(|e| err(e.0))?;
        let nonce = u64::from_le_bytes(random_bytes::<8>()?);
        let sealed = save_crypto::seal(&link.enc, &link.mac, text.as_bytes(), nonce);
        match self
            .agent
            .put(&self.endpoint())
            .set("Authorization", &format!("Bearer {}", link.token))
            .set("X-Base-Rev", &base_rev.to_string())
            .set("Content-Type", "application/octet-stream")
            .send_bytes(&sealed)
        {
            Ok(resp) => Ok(PushOutcome::Stored {
                rev: rev_of(&resp)?,
            }),
            Err(ureq::Error::Status(409, _)) => Ok(PushOutcome::Conflict),
            Err(ureq::Error::Status(413, _)) => Err(err("partie trop grosse pour le relais")),
            Err(ureq::Error::Status(429, _)) => {
                Err(err("trop de synchronisations : nouvel essai plus tard"))
            }
            Err(ureq::Error::Status(code, _)) => {
                Err(err(format!("le relais a refusé l'envoi ({code})")))
            }
            Err(ureq::Error::Transport(t)) => Err(transport(&t)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip_and_tolerate_typing() {
        let secret: [u8; SECRET_LEN] = core::array::from_fn(|i| (i as u8).wrapping_mul(37) ^ 0xA5);
        let code = format_code(&secret);
        assert_eq!(code.len(), 29);
        assert_eq!(code.matches('-').count(), 4);
        assert_eq!(parse_code(&code), Some(secret));
        assert_eq!(parse_code(&format!("{QR_PREFIX}{code}")), Some(secret));
        let typed = code.to_lowercase().replace('-', " ").replace('0', "o");
        assert_eq!(parse_code(&typed), Some(secret));
    }

    #[test]
    fn a_typo_is_caught_by_the_check_character() {
        let secret = [7u8; SECRET_LEN];
        let code = format_code(&secret);
        let mut chars: Vec<char> = code.chars().collect();
        chars[3] = if chars[3] == 'A' { 'B' } else { 'A' };
        let typo: String = chars.into_iter().collect();
        assert_eq!(parse_code(&typo), None);
        assert_eq!(parse_code("ABC"), None);
        assert_eq!(parse_code("ABCDE-FGHJK-MNPQR-STVWX-YZ01!"), None);
    }

    #[test]
    fn the_link_is_kept_next_to_the_save() {
        let dir = std::env::temp_dir().join(format!("fonderie-sync-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let mut sync = RelaySync::new("http://127.0.0.1:1", &dir);
        assert_eq!(sync.link_code(), None);
        let code = sync.create_link().unwrap();
        sync.set_synced_rev(4).unwrap();
        let again = RelaySync::new("http://127.0.0.1:1", &dir);
        assert_eq!(again.link_code(), Some(code.clone()));
        assert_eq!(again.synced_rev(), 4);
        // Les clés dérivées du même code sont les mêmes ; le jeton ne révèle pas le code.
        let link = again.link.as_ref().unwrap();
        assert_eq!(link.token.len(), 64);
        assert!(!link.token.contains(&code.replace('-', "").to_lowercase()));
        // Hors ligne : erreur lisible, pas de panique.
        assert!(again.pull().is_err());
        sync.unlink().unwrap();
        assert!(!dir.join(LINK_FILE).exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
