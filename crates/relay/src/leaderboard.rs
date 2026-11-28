//! Classement : joueurs (pseudo et dernière fiche de score) et Défi du jour.
//!
//! Le jeu envoie une **fiche de score** en clair, signée par HMAC-SHA256 avec
//! la clé du classement des versions officielles (`LEADERBOARD_KEY`, même
//! valeur que le secret `FONDERIE_LEADERBOARD_KEY` de leur compilation). Le
//! relais vérifie la signature et la vraisemblance, garde la dernière fiche de
//! chaque joueur et son meilleur score du jour. Un joueur est désigné par
//! l'empreinte de son jeton (comme une partie synchronisée) ; il n'apparaît
//! qu'après avoir choisi un pseudo.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Temps de jeu minimal (secondes) pour chaque acte (index = acte). Environ un
/// tiers de celui du simulateur à 3 h de jeu par jour.
const MIN_PLAY: [f64; 6] = [0.0, 0.0, 5_400.0, 90_000.0, 144_000.0, 172_800.0];
/// Écart toléré entre l'horloge de l'appareil et celle du relais.
const MAX_CLOCK_SKEW: u64 = 600;
/// Score du Défi du jour au-delà duquel une fiche est refusée.
const MAX_DAILY_SCORE: f64 = 1e40;
/// Jours de Défi du jour conservés.
pub const DAILY_DAYS: u64 = 30;
pub const MAX_LIMIT: usize = 100;
const PLAYERS_FILE: &str = "players.json";

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Daily {
    pub day: u64,
    pub score: f64,
}

/// Fiche de score envoyée par le jeu.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Card {
    pub act: u8,
    pub progress: f64,
    pub total_log: f64,
    pub supernovas: u32,
    pub crunches: u32,
    pub big_bangs: u32,
    pub achievements: u32,
    pub play_time: f64,
    #[serde(default)]
    pub daily: Option<Daily>,
    #[serde(default)]
    pub platform: String,
    #[serde(default)]
    pub version: String,
    pub sent_at: u64,
    /// Début de la partie (dernier « Nouvelle partie », secondes Unix ; 0 :
    /// jamais ou version antérieure à 0.18.1).
    #[serde(default)]
    pub started_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Player {
    pub pseudo: String,
    #[serde(default)]
    pub card: Option<Card>,
    #[serde(default)]
    pub best_play_time: f64,
    /// Début de la partie de la dernière fiche : `best_play_time` ne vaut que
    /// pour cette partie.
    #[serde(default)]
    pub started_at: u64,
    /// Dernière activité (secondes Unix).
    #[serde(default)]
    pub updated: u64,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct DailyEntry {
    pub score: f64,
    /// Heure du premier envoi de ce score (départage les égalités).
    pub at: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Reject {
    Invalid(&'static str),
    /// Partie moins jouée qu'une fiche déjà reçue (appareil en retard).
    Older,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ClaimError {
    Invalid(&'static str),
    Taken,
    Storage,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SubmitError {
    NoPseudo,
    Rejected(Reject),
    Storage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoardKind {
    General,
    Daily,
}

/// Pseudo nettoyé, ou la raison du refus.
pub fn valid_pseudo(raw: &str) -> Result<String, &'static str> {
    let words: Vec<&str> = raw.split_whitespace().collect();
    let pseudo = words.join(" ");
    let n = pseudo.chars().count();
    if n < 3 {
        return Err("pseudo trop court (3 caractères minimum)");
    }
    if n > 16 {
        return Err("pseudo trop long (16 caractères maximum)");
    }
    if !pseudo
        .chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | ' '))
    {
        return Err("lettres, chiffres, espaces, - et _ seulement");
    }
    Ok(pseudo)
}

/// Clé d'unicité : minuscules, sans accents.
pub fn pseudo_key(pseudo: &str) -> String {
    let mut out = String::with_capacity(pseudo.len());
    for c in pseudo.chars().flat_map(char::to_lowercase) {
        match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => out.push('a'),
            'é' | 'è' | 'ê' | 'ë' => out.push('e'),
            'î' | 'ï' | 'í' | 'ì' => out.push('i'),
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => out.push('o'),
            'ù' | 'û' | 'ü' | 'ú' => out.push('u'),
            'ç' => out.push('c'),
            'ÿ' | 'ý' => out.push('y'),
            'ñ' => out.push('n'),
            'œ' => out.push_str("oe"),
            'æ' => out.push_str("ae"),
            ' ' | '-' | '_' => out.push('-'),
            c => out.push(c),
        }
    }
    out
}

/// HMAC-SHA256 (RFC 2104), clé de 32 octets.
pub fn hmac(key: &[u8; 32], data: &[u8]) -> [u8; 32] {
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for (i, k) in key.iter().enumerate() {
        ipad[i] ^= k;
        opad[i] ^= k;
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(data);
    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner.finalize());
    outer.finalize().into()
}

#[cfg(test)]
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Lit 32 octets en hexadécimal.
pub fn parse_key(text: &str) -> Option<[u8; 32]> {
    let t = text.trim();
    if t.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(t.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(out)
}

/// La signature (hexadécimal) correspond au corps.
pub fn verify(key: &[u8; 32], body: &[u8], sig: Option<&str>) -> bool {
    let Some(sig) = sig.and_then(parse_key) else {
        return false;
    };
    let expected = hmac(key, body);
    // Comparaison en temps constant.
    expected
        .iter()
        .zip(sig.iter())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

/// Ordre du classement général : acte, puis Big Bang, Big Crunch,
/// Supernovas, progression de la collection et enfin poussière récoltée.
/// Un prestige de plus passe toujours devant : la collection retombe à
/// chaque Big Crunch (générateurs remis à zéro), elle ne fait que départager.
fn rank_cmp(a: &Card, b: &Card) -> std::cmp::Ordering {
    a.act
        .cmp(&b.act)
        .then(a.big_bangs.cmp(&b.big_bangs))
        .then(a.crunches.cmp(&b.crunches))
        .then(a.supernovas.cmp(&b.supernovas))
        .then(a.progress.total_cmp(&b.progress))
        .then(a.total_log.total_cmp(&b.total_log))
}

/// Points affichés, dans le même ordre que le classement : Big Bang 1 000 000,
/// Big Crunch 10 000, Supernova 100 (99 au plus), progression jusqu'à 99.
pub fn rank_points(c: &Card) -> f64 {
    1e6 * f64::from(c.big_bangs)
        + 1e4 * f64::from(c.crunches)
        + 100.0 * f64::from(c.supernovas.min(99))
        + (c.progress * 99.0).round()
}

/// Clé de développement : celle d'une compilation locale du jeu.
pub fn dev_key() -> [u8; 32] {
    Sha256::digest(b"fonderie-leaderboard/fonderie-stellaire/cle-de-developpement").into()
}

/// La fiche est-elle vraisemblable ? `prev_play_time` : meilleur temps de jeu
/// déjà reçu pour ce joueur.
pub fn plausible(card: &Card, prev_play_time: f64, now: u64) -> Result<(), Reject> {
    let finite = [card.progress, card.total_log, card.play_time]
        .iter()
        .all(|v| v.is_finite());
    if !finite || card.play_time < 0.0 || card.total_log < 0.0 {
        return Err(Reject::Invalid("valeurs invalides"));
    }
    if !(0.0..=1.0).contains(&card.progress) {
        return Err(Reject::Invalid("progression hors limites"));
    }
    if !(1..=5).contains(&card.act) {
        return Err(Reject::Invalid("acte inconnu"));
    }
    if card.play_time < MIN_PLAY[card.act as usize] {
        return Err(Reject::Invalid("progression trop rapide"));
    }
    if card.sent_at.abs_diff(now) > MAX_CLOCK_SKEW {
        return Err(Reject::Invalid("horloge de l'appareil décalée"));
    }
    if card.started_at > now + MAX_CLOCK_SKEW {
        return Err(Reject::Invalid("partie commencée dans le futur"));
    }
    if let Some(d) = card.daily {
        let today = now / 86_400;
        if d.day != today && d.day + 1 != today {
            return Err(Reject::Invalid("Défi du jour d'une autre date"));
        }
        if !d.score.is_finite() || !(0.0..=MAX_DAILY_SCORE).contains(&d.score) {
            return Err(Reject::Invalid("score du jour invalide"));
        }
    }
    if card.play_time + 60.0 < prev_play_time {
        return Err(Reject::Older);
    }
    Ok(())
}

pub struct Board {
    dir: PathBuf,
    players: HashMap<String, Player>,
    daily: HashMap<u64, HashMap<String, DailyEntry>>,
}

fn write_json(path: &PathBuf, value: &impl Serialize) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_vec(value).map_err(io::Error::other)?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, text)?;
    fs::rename(tmp, path)
}

fn read_json<T: for<'de> Deserialize<'de> + Default>(path: &PathBuf) -> T {
    fs::read(path)
        .ok()
        .and_then(|raw| serde_json::from_slice(&raw).ok())
        .unwrap_or_default()
}

impl Board {
    /// Classement gardé dans `dir` (créé au premier enregistrement).
    pub fn load(dir: PathBuf) -> Self {
        let players = read_json(&dir.join(PLAYERS_FILE));
        let mut daily = HashMap::new();
        if let Ok(entries) = fs::read_dir(&dir) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                let day = name
                    .strip_prefix("daily-")
                    .and_then(|r| r.strip_suffix(".json"))
                    .and_then(|d| d.parse::<u64>().ok());
                if let Some(day) = day {
                    daily.insert(day, read_json(&e.path()));
                }
            }
        }
        Board {
            dir,
            players,
            daily,
        }
    }

    fn save_players(&self) -> io::Result<()> {
        write_json(&self.dir.join(PLAYERS_FILE), &self.players)
    }

    fn save_day(&self, day: u64) -> io::Result<()> {
        let path = self.dir.join(format!("daily-{day}.json"));
        match self.daily.get(&day) {
            Some(d) => write_json(&path, d),
            None => match fs::remove_file(path) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
                _ => Ok(()),
            },
        }
    }

    pub fn player(&self, id: &str) -> Option<&Player> {
        self.players.get(id)
    }

    /// Réserve ou change le pseudo du joueur `id`.
    pub fn claim(&mut self, id: &str, raw: &str, now: u64) -> Result<String, ClaimError> {
        let pseudo = valid_pseudo(raw).map_err(ClaimError::Invalid)?;
        let key = pseudo_key(&pseudo);
        let taken = self
            .players
            .iter()
            .any(|(other, p)| other != id && pseudo_key(&p.pseudo) == key);
        if taken {
            return Err(ClaimError::Taken);
        }
        let entry = self.players.entry(id.to_string()).or_insert(Player {
            pseudo: String::new(),
            card: None,
            best_play_time: 0.0,
            started_at: 0,
            updated: now,
        });
        entry.pseudo = pseudo.clone();
        entry.updated = now;
        self.save_players().map_err(|_| ClaimError::Storage)?;
        Ok(pseudo)
    }

    /// Retire le joueur de tous les classements.
    pub fn withdraw(&mut self, id: &str) -> io::Result<()> {
        if self.players.remove(id).is_some() {
            self.save_players()?;
        }
        let days: Vec<u64> = self
            .daily
            .iter_mut()
            .filter_map(|(day, d)| d.remove(id).map(|_| *day))
            .collect();
        for day in days {
            self.save_day(day)?;
        }
        Ok(())
    }

    pub fn submit(&mut self, id: &str, card: Card, now: u64) -> Result<(), SubmitError> {
        let Some(player) = self.players.get_mut(id) else {
            return Err(SubmitError::NoPseudo);
        };
        // Un appareil resté sur l'ancienne partie, après une nouvelle partie
        // sur un autre : refusé, il la reprendra à sa prochaine synchro.
        if card.started_at < player.started_at {
            return Err(SubmitError::Rejected(Reject::Older));
        }
        // Nouvelle partie : le temps de jeu repart de zéro.
        let restarted = card.started_at > player.started_at;
        let prev = if restarted {
            0.0
        } else {
            player.best_play_time
        };
        plausible(&card, prev, now).map_err(SubmitError::Rejected)?;
        if restarted {
            player.started_at = card.started_at;
            player.best_play_time = 0.0;
        }
        player.best_play_time = player.best_play_time.max(card.play_time);
        player.updated = now;
        let daily = card.daily;
        player.card = Some(card);
        self.save_players().map_err(|_| SubmitError::Storage)?;
        if let Some(d) = daily.filter(|d| d.score > 0.0) {
            let day = self.daily.entry(d.day).or_default();
            let better = day.get(id).is_none_or(|e| d.score > e.score);
            if better {
                day.insert(
                    id.to_string(),
                    DailyEntry {
                        score: d.score,
                        at: now,
                    },
                );
                self.save_day(d.day).map_err(|_| SubmitError::Storage)?;
            }
        }
        Ok(())
    }

    /// Classement en JSON : les `limit` premiers, et la ligne de `me`.
    pub fn standings(&self, kind: BoardKind, day: u64, limit: usize, me: Option<&str>) -> Value {
        let mut rows: Vec<(&String, &Player, Option<&DailyEntry>)> = match kind {
            BoardKind::General => self
                .players
                .iter()
                .filter(|(_, p)| p.card.is_some())
                .map(|(id, p)| (id, p, None))
                .collect(),
            BoardKind::Daily => self
                .daily
                .get(&day)
                .into_iter()
                .flatten()
                .filter_map(|(id, e)| self.players.get(id).map(|p| (id, p, Some(e))))
                .collect(),
        };
        rows.sort_by(|a, b| match kind {
            BoardKind::General => match (&a.1.card, &b.1.card) {
                (Some(ca), Some(cb)) => rank_cmp(cb, ca),
                (ca, cb) => cb.is_some().cmp(&ca.is_some()),
            }
            .then(a.1.pseudo.cmp(&b.1.pseudo)),
            BoardKind::Daily => {
                let (ea, eb) = (a.2.map_or(0.0, |e| e.score), b.2.map_or(0.0, |e| e.score));
                eb.total_cmp(&ea)
                    .then(a.2.map_or(0, |e| e.at).cmp(&b.2.map_or(0, |e| e.at)))
                    .then(a.1.pseudo.cmp(&b.1.pseudo))
            }
        });
        let row = |rank: usize, (id, p, e): &(&String, &Player, Option<&DailyEntry>)| {
            let c = p.card.as_ref();
            json!({
                "rank": rank,
                "pseudo": p.pseudo,
                "me": me == Some(id.as_str()),
                "act": c.map_or(0, |c| c.act),
                "progress": c.map_or(0.0, |c| c.progress),
                "total_log": c.map_or(0.0, |c| c.total_log),
                "supernovas": c.map_or(0, |c| c.supernovas),
                "crunches": c.map_or(0, |c| c.crunches),
                "big_bangs": c.map_or(0, |c| c.big_bangs),
                "achievements": c.map_or(0, |c| c.achievements),
                "play_time": c.map_or(0.0, |c| c.play_time),
                "points": c.map_or(0.0, rank_points).round(),
                "platform": c.map_or("", |c| c.platform.as_str()),
                "score": e.map(|e| e.score),
            })
        };
        let entries: Vec<Value> = rows
            .iter()
            .take(limit.min(MAX_LIMIT))
            .enumerate()
            .map(|(i, r)| row(i + 1, r))
            .collect();
        let mine = me.and_then(|me| {
            rows.iter()
                .position(|(id, _, _)| id.as_str() == me)
                .map(|i| row(i + 1, &rows[i]))
        });
        json!({
            "board": match kind { BoardKind::General => "general", BoardKind::Daily => "daily" },
            "day": day,
            "total": rows.len(),
            "entries": entries,
            "me": mine,
        })
    }

    /// Efface les joueurs inactifs depuis `max_age` secondes et les Défis du
    /// jour trop anciens. Renvoie le nombre d'éléments effacés.
    pub fn expire(&mut self, now: u64, max_age: u64) -> usize {
        let before = self.players.len();
        self.players
            .retain(|_, p| now.saturating_sub(p.updated) <= max_age);
        let mut removed = before - self.players.len();
        if removed > 0 {
            let _ = self.save_players();
        }
        let today = now / 86_400;
        let old: Vec<u64> = self
            .daily
            .keys()
            .copied()
            .filter(|&d| d + DAILY_DAYS < today)
            .collect();
        for day in old {
            self.daily.remove(&day);
            let _ = self.save_day(day);
            removed += 1;
        }
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_800_000_000;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fonderie-lb-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    pub fn card(act: u8, progress: f64, play_time: f64) -> Card {
        Card {
            act,
            progress,
            total_log: 12.0,
            supernovas: 3,
            crunches: 0,
            big_bangs: 0,
            achievements: 10,
            play_time,
            daily: None,
            platform: "terminal".into(),
            version: "0.18.0".into(),
            sent_at: NOW,
            started_at: 0,
        }
    }

    #[test]
    fn a_new_game_restarts_the_player_and_older_games_are_refused() {
        let mut b = Board::load(dir("restart"));
        b.claim("p", "Astra", NOW).unwrap();
        b.submit("p", card(4, 0.5, 200_000.0), NOW).unwrap();
        // Nouvelle partie : moins jouée, acceptée car commencée plus tard.
        let mut fresh = card(1, 0.01, 120.0);
        fresh.started_at = NOW - 200;
        assert_eq!(b.submit("p", fresh.clone(), NOW), Ok(()));
        assert_eq!(b.player("p").unwrap().card.as_ref().unwrap().act, 1);
        // L'autre appareil, encore sur l'ancienne partie : refusé.
        assert_eq!(
            b.submit("p", card(4, 0.6, 210_000.0), NOW),
            Err(SubmitError::Rejected(Reject::Older))
        );
        // La nouvelle partie continue normalement…
        fresh.play_time = 400.0;
        assert_eq!(b.submit("p", fresh.clone(), NOW), Ok(()));
        // … et ne recule pas.
        fresh.play_time = 100.0;
        assert_eq!(
            b.submit("p", fresh.clone(), NOW),
            Err(SubmitError::Rejected(Reject::Older))
        );
        // Commencée dans le futur : refusée.
        let mut future = card(1, 0.01, 10.0);
        future.started_at = NOW + 3_600;
        assert!(b.submit("p", future, NOW).is_err());
    }

    #[test]
    fn pseudos_are_cleaned_and_checked() {
        assert_eq!(
            valid_pseudo("  Éloïse   la  Forge "),
            Ok("Éloïse la Forge".into())
        );
        assert!(valid_pseudo("ab").is_err());
        assert!(valid_pseudo("abcdefghijklmnopq").is_err());
        assert!(valid_pseudo("<script>").is_err());
        assert_eq!(valid_pseudo("Nova_7-b"), Ok("Nova_7-b".into()));
        assert_eq!(pseudo_key("Éloïse"), pseudo_key("eloise"));
        assert_eq!(pseudo_key("Nova Forge"), pseudo_key("nova-forge"));
    }

    #[test]
    fn a_pseudo_belongs_to_one_player() {
        let mut b = Board::load(dir("claim"));
        assert_eq!(b.claim("a", "Éloïse", NOW), Ok("Éloïse".into()));
        assert_eq!(b.claim("b", "eloise", NOW), Err(ClaimError::Taken));
        // Le même joueur peut reprendre son pseudo ou en changer.
        assert_eq!(b.claim("a", "ELOISE", NOW), Ok("ELOISE".into()));
        assert_eq!(b.claim("a", "Nova", NOW), Ok("Nova".into()));
        assert_eq!(b.claim("b", "Eloise", NOW), Ok("Eloise".into()));
        assert!(matches!(
            b.claim("c", "x", NOW),
            Err(ClaimError::Invalid(_))
        ));
        let _ = fs::remove_dir_all(&b.dir);
    }

    #[test]
    fn signatures_are_checked() {
        let key = dev_key();
        let body = br#"{"act":1}"#;
        let sig = hex(&hmac(&key, body));
        assert!(verify(&key, body, Some(&sig)));
        assert!(!verify(&key, br#"{"act":5}"#, Some(&sig)));
        assert!(!verify(&key, body, None));
        assert!(!verify(&key, body, Some("zz")));
        // Vecteur fixe partagé avec le jeu (infrastructure::relay_leaderboard).
        assert_eq!(
            hex(&hmac(&[7u8; 32], b"fonderie")),
            "c2b7c56c35860cd8d9c64b1b447dee5d8111029f942350be91d94ce972881c4b"
        );
    }

    #[test]
    fn implausible_cards_are_refused() {
        assert_eq!(plausible(&card(1, 0.1, 100.0), 0.0, NOW), Ok(()));
        let mut c = card(3, 0.5, 3_600.0);
        assert_eq!(
            plausible(&c, 0.0, NOW),
            Err(Reject::Invalid("progression trop rapide"))
        );
        c.play_time = 100_000.0;
        assert_eq!(plausible(&c, 0.0, NOW), Ok(()));
        assert_eq!(plausible(&c, 200_000.0, NOW), Err(Reject::Older));
        assert!(plausible(&card(6, 0.5, 1e9), 0.0, NOW).is_err());
        assert!(plausible(&card(1, 1.5, 100.0), 0.0, NOW).is_err());
        assert!(plausible(&card(1, f64::NAN, 100.0), 0.0, NOW).is_err());
        let mut late = card(1, 0.1, 100.0);
        late.sent_at = NOW - 3_600;
        assert!(plausible(&late, 0.0, NOW).is_err());
        let mut d = card(1, 0.1, 100.0);
        d.daily = Some(Daily {
            day: NOW / 86_400 - 1,
            score: 1e9,
        });
        assert_eq!(plausible(&d, 0.0, NOW), Ok(()));
        d.daily = Some(Daily {
            day: NOW / 86_400 - 2,
            score: 1e9,
        });
        assert!(plausible(&d, 0.0, NOW).is_err());
        d.daily = Some(Daily {
            day: NOW / 86_400,
            score: 1e50,
        });
        assert!(plausible(&d, 0.0, NOW).is_err());
    }

    #[test]
    fn standings_are_sorted_and_show_me_outside_the_top() {
        let mut b = Board::load(dir("rank"));
        let today = NOW / 86_400;
        for (id, act, progress, score) in [
            ("a", 1, 0.9, 5.0),
            ("b", 2, 0.1, 50.0),
            ("c", 1, 0.95, 50.0),
            ("d", 1, 0.2, 1.0),
        ] {
            b.claim(id, &format!("joueur-{id}"), NOW).unwrap();
            let mut c = card(act, progress, 10_000.0);
            c.daily = Some(Daily { day: today, score });
            b.submit(id, c, NOW + id.len() as u64).unwrap();
        }
        assert_eq!(
            b.submit("x", card(1, 0.1, 10.0), NOW),
            Err(SubmitError::NoPseudo)
        );
        let g = b.standings(BoardKind::General, today, 2, Some("d"));
        let names: Vec<&str> = g["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["pseudo"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["joueur-b", "joueur-c"]);
        assert_eq!(g["me"]["rank"], 4);
        assert_eq!(g["me"]["me"], true);
        assert_eq!(g["total"], 4);
        let d = b.standings(BoardKind::Daily, today, 10, None);
        let first = &d["entries"][0];
        assert_eq!(first["score"], 50.0);
        // Égalité : le premier à l'avoir envoyé passe devant.
        assert_eq!(first["pseudo"], "joueur-b");
        // Un moins bon score du jour ne remplace pas le meilleur.
        let mut worse = card(2, 0.1, 10_000.0);
        worse.daily = Some(Daily {
            day: today,
            score: 2.0,
        });
        b.submit("b", worse, NOW).unwrap();
        let d = b.standings(BoardKind::Daily, today, 10, None);
        assert_eq!(d["entries"][0]["pseudo"], "joueur-b");
        let _ = fs::remove_dir_all(&b.dir);
    }

    #[test]
    fn prestiges_come_before_the_collection_in_the_general_ranking() {
        let mut b = Board::load(dir("points"));
        // Cas réel : 3 Big Crunch et 41 % de collection passent devant
        // 2 Big Crunch et 48 %, malgré plus de poussière récoltée.
        let mut ahead = card(2, 0.41, 100_000.0);
        ahead.crunches = 3;
        ahead.supernovas = 5;
        let mut behind = card(2, 0.48, 100_000.0);
        behind.crunches = 2;
        behind.supernovas = 1;
        behind.total_log = 18.8;
        let mut bangs = card(2, 0.1, 100_000.0);
        bangs.big_bangs = 1;
        // Mêmes prestiges : la collection départage, puis la poussière.
        let mut tie = card(2, 0.41, 100_000.0);
        tie.crunches = 3;
        tie.supernovas = 5;
        tie.total_log = 1.0;
        for (id, c) in [("a", ahead), ("b", behind), ("c", bangs), ("t", tie)] {
            b.claim(id, &format!("joueur-{id}"), NOW).unwrap();
            b.submit(id, c, NOW).unwrap();
        }
        let g = b.standings(BoardKind::General, NOW / 86_400, 10, None);
        let names: Vec<&str> = g["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["pseudo"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["joueur-c", "joueur-a", "joueur-t", "joueur-b"]);
        let points: Vec<f64> = g["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["points"].as_f64().unwrap())
            .collect();
        assert_eq!(points, [1_000_310.0, 30_541.0, 30_541.0, 20_148.0]);
        // L'acte passe avant tout.
        b.claim("d", "joueur-d", NOW).unwrap();
        b.submit("d", card(3, 0.0, 100_000.0), NOW).unwrap();
        let g = b.standings(BoardKind::General, NOW / 86_400, 1, None);
        assert_eq!(g["entries"][0]["pseudo"], "joueur-d");
        let _ = fs::remove_dir_all(&b.dir);
    }

    #[test]
    fn the_board_survives_a_restart_and_expires() {
        let path = dir("persist");
        let today = NOW / 86_400;
        {
            let mut b = Board::load(path.clone());
            b.claim("a", "Nova", NOW).unwrap();
            let mut c = card(1, 0.5, 100.0);
            c.daily = Some(Daily {
                day: today,
                score: 9.0,
            });
            b.submit("a", c, NOW).unwrap();
            b.claim("old", "Ancien", NOW - 400 * 86_400).unwrap();
        }
        let mut b = Board::load(path.clone());
        assert_eq!(b.player("a").unwrap().pseudo, "Nova");
        assert_eq!(b.standings(BoardKind::Daily, today, 10, None)["total"], 1);
        // `claim` a remis `updated` à jour : on le vieillit à la main.
        b.players.get_mut("old").unwrap().updated = NOW - 400 * 86_400;
        assert_eq!(b.expire(NOW, 365 * 86_400), 1);
        assert!(b.player("old").is_none());
        assert_eq!(b.expire(NOW + 40 * 86_400, u64::MAX), 1, "jour effacé");
        b.withdraw("a").unwrap();
        assert!(b.player("a").is_none());
        let _ = fs::remove_dir_all(&path);
    }
}
