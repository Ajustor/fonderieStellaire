//! Classement en ligne, par le relais de synchronisation (`crates/relay`).
//!
//! L'identité du joueur dérive du **code de lien** (`relay_sync`) : un jeton
//! distinct de celui de la partie, dont le relais ne garde que l'empreinte.
//! Le pseudo est gardé par le relais et recopié dans `leaderboard.json`, avec
//! le code du lien auquel il appartient (un nouveau lien = nouvelle identité).
//!
//! La fiche de score est signée par HMAC-SHA256 avec la clé du classement,
//! injectée à la compilation par la CI (`FONDERIE_LEADERBOARD_KEY`, secret du
//! dépôt, 64 caractères hexadécimaux) ; le relais reçoit la même valeur dans
//! `LEADERBOARD_KEY`. Elle est indépendante de la clé des sauvegardes. Une
//! compilation locale utilise la clé de développement, que le relais accepte
//! quand `LEADERBOARD_KEY` n'est pas fourni.

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use application::{Board, Entry, LbError, LeaderboardBackend, ScoreCard, Standings};
use serde::{Deserialize, Serialize};

use crate::relay_sync::{self, hex};
use crate::save_crypto;

const STATE_FILE: &str = "leaderboard.json";
/// Clé du classement des versions officielles, vérifiée à la compilation : une
/// valeur qui n'est pas 64 caractères hexadécimaux fait échouer la compilation
/// au lieu de produire une version dont toutes les fiches seraient refusées.
const OFFICIAL_KEY: Option<[u8; 32]> = parse_key(option_env!("FONDERIE_LEADERBOARD_KEY"));
const MAX_BODY: u64 = 512 * 1024;
const LIMIT: u32 = 20;

#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
#[serde(default)]
struct LocalState {
    code: String,
    pseudo: String,
}

#[derive(Serialize)]
struct CardJson<'a> {
    act: u8,
    progress: f64,
    total_log: f64,
    supernovas: u32,
    crunches: u32,
    big_bangs: u32,
    achievements: u32,
    play_time: f64,
    daily: Option<DailyJson>,
    platform: &'a str,
    version: &'a str,
    sent_at: u64,
    started_at: u64,
}

#[derive(Serialize)]
struct DailyJson {
    day: u64,
    score: f64,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct EntryJson {
    rank: u32,
    pseudo: String,
    me: bool,
    act: u8,
    progress: f64,
    total_log: f64,
    supernovas: u32,
    crunches: u32,
    big_bangs: u32,
    play_time: f64,
    platform: String,
    score: Option<f64>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct StandingsJson {
    day: u64,
    total: u32,
    entries: Vec<EntryJson>,
    me: Option<EntryJson>,
}

impl From<EntryJson> for Entry {
    fn from(e: EntryJson) -> Self {
        Entry {
            rank: e.rank,
            pseudo: e.pseudo,
            me: e.me,
            act: e.act,
            progress: e.progress,
            total_log: e.total_log,
            supernovas: e.supernovas,
            crunches: e.crunches,
            big_bangs: e.big_bangs,
            play_time: e.play_time,
            platform: e.platform,
            score: e.score,
        }
    }
}

/// Corps JSON d'une fiche (exactement les octets signés).
pub fn card_json(card: &ScoreCard) -> Vec<u8> {
    let json = CardJson {
        act: card.act,
        progress: card.progress,
        total_log: card.total_log,
        supernovas: card.supernovas,
        crunches: card.crunches,
        big_bangs: card.big_bangs,
        achievements: card.achievements,
        play_time: card.play_time,
        daily: card.daily.map(|(day, score)| DailyJson { day, score }),
        platform: card.platform,
        version: &card.version,
        sent_at: card.sent_at,
        started_at: card.started_at,
    };
    serde_json::to_vec(&json).unwrap_or_default()
}

/// Lit une clé hexadécimale (espaces et retours à la ligne ignorés). Absente
/// ou vide : `None`. Mal formée : erreur de compilation (fonction `const`).
const fn parse_key(value: Option<&str>) -> Option<[u8; 32]> {
    let bytes = match value {
        Some(v) => v.as_bytes(),
        None => return None,
    };
    let mut key = [0u8; 32];
    let mut digits = 0;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        i += 1;
        let nibble = match c {
            b' ' | b'\t' | b'\r' | b'\n' => continue,
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            b'A'..=b'F' => c - b'A' + 10,
            _ => panic!("FONDERIE_LEADERBOARD_KEY : caractère non hexadécimal"),
        };
        assert!(
            digits < 64,
            "FONDERIE_LEADERBOARD_KEY : 64 caractères hexadécimaux attendus"
        );
        key[digits / 2] |= if digits % 2 == 0 { nibble << 4 } else { nibble };
        digits += 1;
    }
    if digits == 0 {
        return None;
    }
    assert!(
        digits == 64,
        "FONDERIE_LEADERBOARD_KEY : 64 caractères hexadécimaux attendus"
    );
    Some(key)
}

/// Clé de développement, la même que celle du relais sans `LEADERBOARD_KEY`.
fn dev_key() -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::digest(b"fonderie-leaderboard/fonderie-stellaire/cle-de-developpement").into()
}

/// Clé de signature des fiches : officielle si la CI l'a fournie.
pub fn signing_key() -> [u8; 32] {
    OFFICIAL_KEY.unwrap_or_else(dev_key)
}

/// Signature hexadécimale d'un corps avec la clé `key`.
pub fn sign(key: &[u8; 32], body: &[u8]) -> String {
    hex(&save_crypto::hmac(key, &[body]))
}

/// Empreinte publique d'un jeton (identifiant du joueur sur le relais).
pub fn player_id(token: &str) -> String {
    use sha2::{Digest, Sha256};
    hex(&Sha256::digest(token.as_bytes()))
}

pub struct RelayLeaderboard {
    url: String,
    dir: PathBuf,
    agent: ureq::Agent,
    key: [u8; 32],
}

fn net(e: ureq::Error) -> LbError {
    match e {
        ureq::Error::Status(429, _) => {
            LbError::Network("trop de requêtes : nouvel essai plus tard".into())
        }
        ureq::Error::Status(code, resp) => {
            let msg = resp.into_string().unwrap_or_default();
            LbError::Network(format!("le relais a répondu {code} {}", msg.trim()))
        }
        ureq::Error::Transport(t) => {
            LbError::Network(format!("relais injoignable (hors ligne ?) : {t}"))
        }
    }
}

impl RelayLeaderboard {
    /// Classement sur le relais `url` ; lien et pseudo gardés dans `dir`.
    pub fn new(url: impl Into<String>, dir: &Path) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(8))
            .timeout(Duration::from_secs(20))
            .user_agent(concat!("fonderie-stellaire/", env!("CARGO_PKG_VERSION")))
            .build();
        RelayLeaderboard {
            url: url.into(),
            dir: dir.to_path_buf(),
            agent,
            key: signing_key(),
        }
    }

    /// Relais configuré dans cette version (même adresse que la synchro).
    pub fn from_env(dir: &Path) -> Option<Self> {
        relay_sync::relay_url().map(|u| Self::new(u, dir))
    }

    fn code(&self) -> Option<String> {
        relay_sync::stored_code(&self.dir)
    }

    fn token(&self) -> Result<String, LbError> {
        self.code()
            .and_then(|c| relay_sync::leaderboard_token(&c))
            .ok_or(LbError::NotLinked)
    }

    fn state_path(&self) -> PathBuf {
        self.dir.join(STATE_FILE)
    }

    fn read_state(&self) -> LocalState {
        fs::read_to_string(self.state_path())
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    fn write_pseudo(&self, pseudo: Option<&str>) -> Result<(), LbError> {
        let io = |e: io::Error| LbError::Network(format!("enregistrement du pseudo : {e}"));
        let path = self.state_path();
        let Some(pseudo) = pseudo else {
            return match fs::remove_file(&path) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => Err(io(e)),
                _ => Ok(()),
            };
        };
        let state = LocalState {
            code: self.code().unwrap_or_default(),
            pseudo: pseudo.to_string(),
        };
        fs::create_dir_all(&self.dir).map_err(io)?;
        let json = serde_json::to_string(&state).unwrap_or_default();
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, json).map_err(io)?;
        fs::rename(&tmp, &path).map_err(io)
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{path}", self.url)
    }
}

impl LeaderboardBackend for RelayLeaderboard {
    fn linked(&self) -> bool {
        self.code().is_some()
    }

    fn pseudo(&self) -> Option<String> {
        let state = self.read_state();
        let code = self.code()?;
        (state.code == code && !state.pseudo.is_empty()).then_some(state.pseudo)
    }

    fn refresh_pseudo(&mut self) -> Result<Option<String>, LbError> {
        let token = self.token()?;
        let pseudo = match self
            .agent
            .get(&self.endpoint("/v1/player"))
            .set("Authorization", &format!("Bearer {token}"))
            .call()
        {
            Ok(resp) => resp
                .into_string()
                .ok()
                .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
                .and_then(|v| v["pseudo"].as_str().map(str::to_string)),
            Err(ureq::Error::Status(404, _)) => None,
            Err(e) => return Err(net(e)),
        };
        self.write_pseudo(pseudo.as_deref())?;
        Ok(pseudo)
    }

    fn claim(&mut self, pseudo: &str) -> Result<String, LbError> {
        let token = self.token()?;
        let body = serde_json::json!({ "pseudo": pseudo }).to_string();
        let resp = self
            .agent
            .put(&self.endpoint("/v1/player"))
            .set("Authorization", &format!("Bearer {token}"))
            .set("Content-Type", "application/json")
            .send_string(&body);
        let pseudo = match resp {
            Ok(resp) => resp
                .into_string()
                .ok()
                .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
                .and_then(|v| v["pseudo"].as_str().map(str::to_string))
                .unwrap_or_else(|| pseudo.trim().to_string()),
            Err(ureq::Error::Status(409, _)) => return Err(LbError::Taken),
            Err(ureq::Error::Status(400, resp)) => {
                return Err(LbError::Invalid(
                    resp.into_string().unwrap_or_default().trim().to_string(),
                ))
            }
            Err(e) => return Err(net(e)),
        };
        self.write_pseudo(Some(&pseudo))?;
        Ok(pseudo)
    }

    fn withdraw(&mut self) -> Result<(), LbError> {
        let token = self.token()?;
        match self
            .agent
            .delete(&self.endpoint("/v1/player"))
            .set("Authorization", &format!("Bearer {token}"))
            .call()
        {
            Ok(_) | Err(ureq::Error::Status(404, _)) => self.write_pseudo(None),
            Err(e) => Err(net(e)),
        }
    }

    fn submit(&self, card: &ScoreCard) -> Result<(), LbError> {
        let token = self.token()?;
        let body = card_json(card);
        match self
            .agent
            .post(&self.endpoint("/v1/score"))
            .set("Authorization", &format!("Bearer {token}"))
            .set("X-Signature", &sign(&self.key, &body))
            .set("Content-Type", "application/json")
            .send_bytes(&body)
        {
            Ok(_) => Ok(()),
            // Partie moins jouée qu'un autre appareil du lien : rien à faire.
            Err(ureq::Error::Status(409, _)) => Ok(()),
            Err(e) => Err(net(e)),
        }
    }

    fn standings(&self, board: Board, day: u64) -> Result<Standings, LbError> {
        let mut url = format!(
            "{}?board={}&day={day}&limit={LIMIT}",
            self.endpoint("/v1/leaderboard"),
            match board {
                Board::General => "general",
                Board::Daily => "daily",
            }
        );
        if let Ok(token) = self.token() {
            url.push_str(&format!("&me={}", player_id(&token)));
        }
        let resp = self.agent.get(&url).call().map_err(net)?;
        let mut raw = Vec::new();
        resp.into_reader()
            .take(MAX_BODY)
            .read_to_end(&mut raw)
            .map_err(|e| LbError::Network(format!("lecture du classement : {e}")))?;
        let json: StandingsJson = serde_json::from_slice(&raw)
            .map_err(|_| LbError::Network("classement illisible".into()))?;
        Ok(Standings {
            board,
            day: json.day,
            total: json.total,
            entries: json.entries.into_iter().map(Entry::from).collect(),
            me: json.me.map(Entry::from),
        })
    }

    fn site_url(&self) -> String {
        format!("{}/classement", self.url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use application::SyncBackend;

    #[test]
    fn leaderboard_keys_are_read_from_hex() {
        assert_eq!(parse_key(None), None);
        assert_eq!(parse_key(Some("")), None);
        assert_eq!(parse_key(Some(" \n")), None);
        let hex64 = "07".repeat(32);
        assert_eq!(parse_key(Some(&hex64)), Some([7u8; 32]));
        assert_eq!(
            parse_key(Some(&format!("{}\r\n", hex64.to_uppercase()))),
            Some([7u8; 32])
        );
        let ab = "ab".repeat(32);
        assert_eq!(parse_key(Some(&ab)), Some([0xab; 32]));
    }

    #[test]
    #[should_panic(expected = "64 caractères")]
    fn a_short_leaderboard_key_is_refused() {
        parse_key(Some("abcd"));
    }

    #[test]
    #[should_panic(expected = "non hexadécimal")]
    fn a_non_hex_leaderboard_key_is_refused() {
        parse_key(Some(&"zz".repeat(32)));
    }

    #[test]
    fn local_builds_sign_with_the_relay_dev_key() {
        // Valeur de `crates/relay/src/leaderboard.rs::dev_key`.
        if OFFICIAL_KEY.is_none() {
            assert_eq!(signing_key(), dev_key());
        }
    }

    #[test]
    fn signature_matches_the_relay() {
        // Même vecteur que `crates/relay/src/leaderboard.rs`.
        assert_eq!(
            sign(&[7u8; 32], b"fonderie"),
            "c2b7c56c35860cd8d9c64b1b447dee5d8111029f942350be91d94ce972881c4b"
        );
    }

    #[test]
    fn card_json_is_what_the_relay_reads() {
        let card = ScoreCard {
            act: 2,
            progress: 0.5,
            total_log: 9.0,
            supernovas: 3,
            crunches: 1,
            big_bangs: 0,
            achievements: 12,
            play_time: 7200.0,
            daily: Some((20_000, 1e6)),
            platform: "terminal",
            version: "0.18.0".into(),
            sent_at: 1_800_000_000,
            started_at: 1_700_000_000,
        };
        let v: serde_json::Value = serde_json::from_slice(&card_json(&card)).unwrap();
        assert_eq!(v["act"], 2);
        assert_eq!(v["daily"]["day"], 20_000);
        assert_eq!(v["daily"]["score"], 1e6);
        assert_eq!(v["started_at"], 1_700_000_000u64);
        assert_eq!(v["platform"], "terminal");
        assert_eq!(v["sent_at"], 1_800_000_000u64);
    }

    #[test]
    fn identity_follows_the_link() {
        let dir = std::env::temp_dir().join(format!("fonderie-lb-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let lb = RelayLeaderboard::new("http://127.0.0.1:1", &dir);
        assert!(!lb.linked());
        assert_eq!(lb.token(), Err(LbError::NotLinked));

        let mut sync = relay_sync::RelaySync::new("http://127.0.0.1:1", &dir);
        let code = sync.create_link().unwrap();
        assert!(lb.linked());
        let token = lb.token().unwrap();
        assert_eq!(token.len(), 64);
        // Jeton du classement ≠ jeton de la partie ; identifiant = son empreinte.
        assert_eq!(relay_sync::leaderboard_token(&code), Some(token.clone()));
        assert_eq!(player_id(&token).len(), 64);
        assert_ne!(player_id(&token), token);

        lb.write_pseudo(Some("Nova")).unwrap();
        assert_eq!(lb.pseudo(), Some("Nova".into()));
        // Nouveau lien : nouvelle identité, l'ancien pseudo ne vaut plus.
        sync.create_link().unwrap();
        assert_eq!(lb.pseudo(), None);
        lb.write_pseudo(None).unwrap();
        assert!(!dir.join(STATE_FILE).exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
