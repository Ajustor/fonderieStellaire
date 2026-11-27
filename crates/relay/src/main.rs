//! Relais de synchronisation de Fonderie Stellaire.
//!
//! Garde, pour chaque lien entre appareils, la dernière partie envoyée et son
//! numéro de révision. Il ne voit que des octets chiffrés de bout en bout : le
//! code de lien ne quitte jamais les appareils, seul un jeton qui en est
//! dérivé lui parvient, et il ne stocke que l'empreinte de ce jeton.
//!
//! API (toutes les requêtes `/v1` portent `Authorization: Bearer <64 hex>`) :
//!
//! - `GET /v1/save` → 200 (corps = partie, en-tête `X-Rev`) ou 404 ;
//! - `PUT /v1/save` avec `X-Base-Rev: n` → 200 (`X-Rev: n+1`) si la partie
//!   stockée en est bien à la révision `n` (0 : aucune), sinon 409 (`X-Rev` :
//!   révision actuelle) ;
//! - `GET /health` → 200.
//!
//! Classement (voir [`leaderboard`]) :
//!
//! - `GET /` → page du classement ;
//! - `GET` / `PUT` / `DELETE /v1/player` (Bearer) : pseudo du joueur ;
//! - `POST /v1/score` (Bearer, `X-Signature`) : fiche de score signée ;
//! - `GET /v1/leaderboard?board=general|daily&day=N&limit=N&me=ID`.
//!
//! Configuration (variables d'environnement) : `PORT` (8080), `DATA_DIR`
//! (`/data`), `MAX_BODY_KB` (1024), `RATE_PER_MIN` (60 requêtes par minute et
//! par adresse), `EXPIRE_DAYS` (365 : parties non synchronisées depuis plus
//! longtemps effacées), `TRUST_PROXY` (1 : adresse lue dans
//! `X-Forwarded-For`, derrière le proxy de Dokploy), `LEADERBOARD_KEY` (clé de
//! signature des fiches de score, 64 caractères hexadécimaux).

mod leaderboard;

use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use leaderboard::{Board, BoardKind, Card, ClaimError, Reject, SubmitError};

use sha2::{Digest, Sha256};
use tiny_http::{Header, Method, Request, Response, Server};

struct Config {
    port: u16,
    data: PathBuf,
    max_body: usize,
    rate_per_min: u32,
    expire: Duration,
    trust_proxy: bool,
    leaderboard_key: Option<[u8; 32]>,
}

fn env_or<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::var(name)
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(default)
}

impl Config {
    fn from_env() -> Self {
        Config {
            port: env_or("PORT", 8080),
            data: PathBuf::from(env_or("DATA_DIR", "/data".to_string())),
            max_body: env_or("MAX_BODY_KB", 1024usize) * 1024,
            rate_per_min: env_or("RATE_PER_MIN", 60),
            expire: Duration::from_secs(env_or("EXPIRE_DAYS", 365u64) * 86_400),
            trust_proxy: env_or("TRUST_PROXY", 1u8) != 0,
            leaderboard_key: std::env::var("LEADERBOARD_KEY")
                .ok()
                .and_then(|k| leaderboard::parse_key(&k)),
        }
    }
}

/// Réponse indépendante de `tiny_http` (testable).
#[derive(Debug, PartialEq, Eq)]
struct Reply {
    status: u16,
    rev: Option<u64>,
    body: Vec<u8>,
    content_type: &'static str,
}

impl Reply {
    fn text(status: u16, msg: &str) -> Self {
        Reply {
            status,
            rev: None,
            body: msg.as_bytes().to_vec(),
            content_type: "text/plain; charset=utf-8",
        }
    }

    fn json(status: u16, value: &serde_json::Value) -> Self {
        Reply {
            status,
            rev: None,
            body: value.to_string().into_bytes(),
            content_type: "application/json",
        }
    }

    fn data(status: u16, rev: Option<u64>, body: Vec<u8>) -> Self {
        Reply {
            status,
            rev,
            body,
            content_type: "application/octet-stream",
        }
    }
}

/// Page du classement (autonome : ni script ni police externes).
const SITE: &str = include_str!("site.html");

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Valeur d'un paramètre de la requête (`?a=1&b=2`).
fn query<'a>(url: &'a str, name: &str) -> Option<&'a str> {
    url.split_once('?')?
        .1
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .find(|(k, _)| *k == name)
        .map(|(_, v)| v)
}

/// Une requête du classement (indépendante de `tiny_http`, testable).
struct LbRequest<'a> {
    method: &'a Method,
    url: &'a str,
    auth: Option<&'a str>,
    signature: Option<&'a str>,
    body: &'a [u8],
    now: u64,
}

/// Routes du classement ; `None` si la requête ne le concerne pas.
fn handle_leaderboard(board: &mut Board, key: &[u8; 32], req: &LbRequest) -> Option<Reply> {
    let url = req.url;
    let path = url.split('?').next().unwrap_or(url);
    let player_route = matches!(
        (req.method, path),
        (Method::Get | Method::Put | Method::Delete, "/v1/player") | (Method::Post, "/v1/score")
    );
    if player_route {
        let Some(id) = slot_id(req.auth) else {
            return Some(Reply::text(401, "jeton manquant ou invalide"));
        };
        return Some(player_request(board, key, req, path, &id));
    }
    match (req.method, path) {
        (Method::Get, "/" | "/index.html") => Some(Reply {
            status: 200,
            rev: None,
            body: SITE.as_bytes().to_vec(),
            content_type: "text/html; charset=utf-8",
        }),
        (Method::Get, "/v1/leaderboard") => {
            let kind = match query(url, "board") {
                Some("daily") => BoardKind::Daily,
                _ => BoardKind::General,
            };
            let day = query(url, "day")
                .and_then(|d| d.parse().ok())
                .unwrap_or(req.now / 86_400);
            let limit = query(url, "limit")
                .and_then(|l| l.parse().ok())
                .unwrap_or(20);
            let me = query(url, "me").filter(|m| m.len() == 64);
            Some(Reply::json(200, &board.standings(kind, day, limit, me)))
        }
        _ => None,
    }
}

/// Routes qui désignent un joueur (identifiant = empreinte du jeton).
fn player_request(
    board: &mut Board,
    key: &[u8; 32],
    req: &LbRequest,
    path: &str,
    id: &str,
) -> Reply {
    match (req.method, path) {
        (Method::Get, "/v1/player") => match board.player(id) {
            Some(p) => Reply::json(200, &serde_json::json!({ "id": id, "pseudo": p.pseudo })),
            None => Reply::text(404, "aucun pseudo"),
        },
        (Method::Put, "/v1/player") => {
            let pseudo = serde_json::from_slice::<serde_json::Value>(req.body)
                .ok()
                .and_then(|v| v["pseudo"].as_str().map(str::to_string));
            let Some(pseudo) = pseudo else {
                return Reply::text(400, "pseudo manquant");
            };
            match board.claim(id, &pseudo, req.now) {
                Ok(p) => Reply::json(200, &serde_json::json!({ "id": id, "pseudo": p })),
                Err(ClaimError::Taken) => Reply::text(409, "pseudo déjà pris"),
                Err(ClaimError::Invalid(why)) => Reply::text(400, why),
                Err(ClaimError::Storage) => Reply::text(500, "erreur de stockage"),
            }
        }
        (Method::Delete, "/v1/player") => match board.withdraw(id) {
            Ok(()) => Reply::text(204, ""),
            Err(_) => Reply::text(500, "erreur de stockage"),
        },
        _ => {
            if !leaderboard::verify(key, req.body, req.signature) {
                return Reply::text(400, "signature invalide");
            }
            let Ok(card) = serde_json::from_slice::<Card>(req.body) else {
                return Reply::text(400, "fiche illisible");
            };
            match board.submit(id, card, req.now) {
                Ok(()) => Reply::text(204, ""),
                Err(SubmitError::NoPseudo) => Reply::text(404, "aucun pseudo"),
                Err(SubmitError::Rejected(Reject::Older)) => Reply::text(
                    409,
                    "partie plus ancienne ou moins jouée qu'une fiche déjà reçue",
                ),
                Err(SubmitError::Rejected(Reject::Invalid(why))) => Reply::text(400, why),
                Err(SubmitError::Storage) => Reply::text(500, "erreur de stockage"),
            }
        }
    }
}

struct Store {
    dir: PathBuf,
    max_body: usize,
}

/// Jeton valide : 64 caractères hexadécimaux (SHA-256 côté appareil).
fn slot_id(auth: Option<&str>) -> Option<String> {
    let token = auth?.trim().strip_prefix("Bearer ")?.trim();
    if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    // On ne stocke que l'empreinte du jeton : le disque seul ne permet pas d'écrire.
    let digest = Sha256::digest(token.to_ascii_lowercase().as_bytes());
    Some(digest.iter().map(|b| format!("{b:02x}")).collect())
}

impl Store {
    fn path(&self, id: &str) -> PathBuf {
        self.dir.join(&id[..2]).join(format!("{id}.bin"))
    }

    /// Partie stockée : révision (8 octets) puis contenu.
    fn read(&self, id: &str) -> io::Result<Option<(u64, Vec<u8>)>> {
        match fs::read(self.path(id)) {
            Ok(raw) if raw.len() >= 8 => {
                let rev = u64::from_le_bytes(raw[..8].try_into().unwrap_or([0; 8]));
                Ok(Some((rev, raw[8..].to_vec())))
            }
            Ok(_) => Ok(None),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn write(&self, id: &str, rev: u64, body: &[u8]) -> io::Result<()> {
        let path = self.path(id);
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut raw = Vec::with_capacity(8 + body.len());
        raw.extend_from_slice(&rev.to_le_bytes());
        raw.extend_from_slice(body);
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, raw)?;
        fs::rename(tmp, path)
    }

    fn handle(
        &self,
        method: &Method,
        url: &str,
        auth: Option<&str>,
        base_rev: Option<&str>,
        body: &[u8],
    ) -> Reply {
        let path = url.split('?').next().unwrap_or(url);
        match (method, path) {
            (Method::Get, "/health") => return Reply::text(200, "ok"),
            (Method::Get | Method::Put, "/v1/save") => {}
            _ => return Reply::text(404, "introuvable"),
        }
        let Some(id) = slot_id(auth) else {
            return Reply::text(401, "jeton manquant ou invalide");
        };
        let current = match self.read(&id) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("lecture {id} : {e}");
                return Reply::text(500, "erreur de stockage");
            }
        };
        if *method == Method::Get {
            return match current {
                Some((rev, data)) => Reply::data(200, Some(rev), data),
                None => Reply::text(404, "aucune partie"),
            };
        }
        if body.len() > self.max_body {
            return Reply::text(413, "partie trop grosse");
        }
        if body.is_empty() {
            return Reply::text(400, "partie vide");
        }
        let Some(base) = base_rev.and_then(|v| v.trim().parse::<u64>().ok()) else {
            return Reply::text(400, "X-Base-Rev manquant");
        };
        let rev = current.as_ref().map_or(0, |(r, _)| *r);
        if rev != base {
            return Reply::data(409, Some(rev), b"conflit".to_vec());
        }
        match self.write(&id, rev + 1, body) {
            Ok(()) => Reply::data(200, Some(rev + 1), b"ok".to_vec()),
            Err(e) => {
                eprintln!("écriture {id} : {e}");
                Reply::text(500, "erreur de stockage")
            }
        }
    }

    /// Efface les parties non synchronisées depuis `max_age`.
    fn expire(&self, max_age: Duration) -> usize {
        let now = SystemTime::now();
        let mut removed = 0;
        let Ok(dirs) = fs::read_dir(&self.dir) else {
            return 0;
        };
        // Seuls les dossiers des parties (deux caractères) : pas `leaderboard`.
        for sub in dirs.flatten().filter(|d| d.file_name().len() == 2) {
            let Ok(files) = fs::read_dir(sub.path()) else {
                continue;
            };
            for f in files.flatten() {
                let old = f
                    .metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| now.duration_since(t).ok())
                    .is_some_and(|age| age > max_age);
                if old && fs::remove_file(f.path()).is_ok() {
                    removed += 1;
                }
            }
        }
        removed
    }
}

/// Limite de débit par adresse, par fenêtres d'une minute.
struct RateLimit {
    per_min: u32,
    window: Instant,
    counts: HashMap<String, u32>,
}

impl RateLimit {
    fn allow(&mut self, who: &str, now: Instant) -> bool {
        if now.duration_since(self.window) >= Duration::from_secs(60) {
            self.window = now;
            self.counts.clear();
        }
        let n = self.counts.entry(who.to_string()).or_insert(0);
        *n += 1;
        *n <= self.per_min
    }
}

fn header<'a>(req: &'a Request, name: &str) -> Option<&'a str> {
    req.headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
}

fn client(req: &Request, trust_proxy: bool) -> String {
    let forwarded = trust_proxy
        .then(|| header(req, "X-Forwarded-For"))
        .flatten()
        .and_then(|v| v.split(',').next())
        .map(|v| v.trim().to_string());
    forwarded
        .or_else(|| req.remote_addr().map(|a| a.ip().to_string()))
        .unwrap_or_default()
}

fn respond(req: Request, reply: Reply) {
    let mut resp = Response::from_data(reply.body).with_status_code(reply.status);
    if let Some(rev) = reply.rev {
        if let Ok(h) = Header::from_bytes("X-Rev", rev.to_string()) {
            resp.add_header(h);
        }
    }
    if let Ok(h) = Header::from_bytes("Cache-Control", "no-store") {
        resp.add_header(h);
    }
    if let Ok(h) = Header::from_bytes("Content-Type", reply.content_type) {
        resp.add_header(h);
    }
    let _ = req.respond(resp);
}

fn serve(cfg: &Config, store: &Store, board: &mut Board, server: &Server) {
    let key = cfg.leaderboard_key.unwrap_or_else(leaderboard::dev_key);
    let mut limit = RateLimit {
        per_min: cfg.rate_per_min,
        window: Instant::now(),
        counts: HashMap::new(),
    };
    let mut next_expire = Instant::now();
    loop {
        if Instant::now() >= next_expire {
            let n = store.expire(cfg.expire);
            if n > 0 {
                println!("{n} partie(s) expirée(s) effacée(s)");
            }
            let n = board.expire(now_unix(), cfg.expire.as_secs());
            if n > 0 {
                println!("{n} entrée(s) du classement effacée(s)");
            }
            next_expire = Instant::now() + Duration::from_secs(86_400);
        }
        let mut req = match server.recv_timeout(Duration::from_secs(60)) {
            Ok(Some(r)) => r,
            Ok(None) => continue,
            Err(e) => {
                eprintln!("réception : {e}");
                continue;
            }
        };
        let who = client(&req, cfg.trust_proxy);
        if !limit.allow(&who, Instant::now()) {
            respond(req, Reply::text(429, "trop de requêtes"));
            continue;
        }
        // Lecture bornée : un octet de plus que la limite suffit à la refuser.
        let mut body = Vec::new();
        let read = req
            .as_reader()
            .take(cfg.max_body as u64 + 1)
            .read_to_end(&mut body);
        if read.is_err() {
            respond(req, Reply::text(400, "corps illisible"));
            continue;
        }
        let lb = LbRequest {
            method: req.method(),
            url: req.url(),
            auth: header(&req, "Authorization"),
            signature: header(&req, "X-Signature"),
            body: &body,
            now: now_unix(),
        };
        let reply = handle_leaderboard(board, &key, &lb).unwrap_or_else(|| {
            store.handle(
                req.method(),
                req.url(),
                header(&req, "Authorization"),
                header(&req, "X-Base-Rev"),
                &body,
            )
        });
        respond(req, reply);
    }
}

/// `--health` : interroge le relais local (HEALTHCHECK du conteneur).
fn health(port: u16) -> bool {
    use std::io::Write;
    let Ok(mut s) = std::net::TcpStream::connect(("127.0.0.1", port)) else {
        return false;
    };
    let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
    let mut out = String::new();
    s.write_all(b"GET /health HTTP/1.0\r\n\r\n").is_ok()
        && s.read_to_string(&mut out).is_ok()
        && out.starts_with("HTTP/1.")
        && out.contains(" 200 ")
}

fn main() {
    let cfg = Config::from_env();
    if std::env::args().nth(1).as_deref() == Some("--health") {
        std::process::exit(if health(cfg.port) { 0 } else { 1 });
    }
    if let Err(e) = fs::create_dir_all(&cfg.data) {
        eprintln!("dossier {} inutilisable : {e}", cfg.data.display());
        std::process::exit(1);
    }
    let server = match Server::http(("0.0.0.0", cfg.port)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("port {} indisponible : {e}", cfg.port);
            std::process::exit(1);
        }
    };
    println!(
        "Relais Fonderie Stellaire : port {}, parties dans {}",
        cfg.port,
        cfg.data.display()
    );
    if cfg.leaderboard_key.is_none() {
        println!("LEADERBOARD_KEY absente : classement signé avec la clé de développement");
    }
    let store = Store {
        dir: cfg.data.clone(),
        max_body: cfg.max_body,
    };
    let mut board = Board::load(cfg.data.join("leaderboard"));
    serve(&cfg, &store, &mut board, &server);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(tag: &str) -> Store {
        let dir = std::env::temp_dir().join(format!("fonderie-relay-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        Store { dir, max_body: 64 }
    }

    const TOKEN: &str = "Bearer 00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

    #[test]
    fn revisions_guard_against_lost_updates() {
        let s = store("rev");
        let get = |s: &Store| s.handle(&Method::Get, "/v1/save", Some(TOKEN), None, &[]);
        assert_eq!(get(&s).status, 404);
        let put = |s: &Store, base: &str, body: &[u8]| {
            s.handle(&Method::Put, "/v1/save", Some(TOKEN), Some(base), body)
        };
        let r = put(&s, "0", b"A");
        assert_eq!((r.status, r.rev), (200, Some(1)));
        // Un appareil en retard est refusé et apprend la révision actuelle.
        let r = put(&s, "0", b"B");
        assert_eq!((r.status, r.rev), (409, Some(1)));
        assert_eq!(put(&s, "1", b"C").rev, Some(2));
        let r = get(&s);
        assert_eq!((r.status, r.rev, r.body), (200, Some(2), b"C".to_vec()));
        let _ = fs::remove_dir_all(&s.dir);
    }

    #[test]
    fn bad_requests_are_refused() {
        let s = store("bad");
        let h = |m: &Method, url: &str, auth: Option<&str>, base: Option<&str>, body: &[u8]| {
            s.handle(m, url, auth, base, body).status
        };
        assert_eq!(h(&Method::Get, "/health", None, None, &[]), 200);
        assert_eq!(h(&Method::Get, "/v1/save", None, None, &[]), 401);
        assert_eq!(
            h(&Method::Get, "/v1/save", Some("Bearer abc"), None, &[]),
            401
        );
        assert_eq!(h(&Method::Put, "/v1/save", Some(TOKEN), None, b"x"), 400);
        assert_eq!(
            h(&Method::Put, "/v1/save", Some(TOKEN), Some("0"), &[]),
            400
        );
        assert_eq!(
            h(&Method::Put, "/v1/save", Some(TOKEN), Some("0"), &[0; 65]),
            413
        );
        assert_eq!(h(&Method::Delete, "/v1/save", Some(TOKEN), None, &[]), 404);
        assert_eq!(
            h(&Method::Get, "/../etc/passwd", Some(TOKEN), None, &[]),
            404
        );
        let _ = fs::remove_dir_all(&s.dir);
    }

    #[test]
    fn the_token_itself_is_never_written_to_disk() {
        let s = store("hash");
        s.handle(&Method::Put, "/v1/save", Some(TOKEN), Some("0"), b"x");
        let id = slot_id(Some(TOKEN)).unwrap();
        assert!(s.path(&id).exists());
        assert!(!id.contains("00112233445566778899"));
        let _ = fs::remove_dir_all(&s.dir);
    }

    #[test]
    fn leaderboard_routes() {
        let dir = std::env::temp_dir().join(format!("fonderie-relay-lb-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let mut board = Board::load(dir.clone());
        let key = leaderboard::dev_key();
        let now = 1_800_000_000;
        let mut call =
            |method: Method, url: &str, auth: Option<&str>, sig: Option<&str>, body: &[u8]| {
                let req = LbRequest {
                    method: &method,
                    url,
                    auth,
                    signature: sig,
                    body,
                    now,
                };
                handle_leaderboard(&mut board, &key, &req)
            };
        // Les routes de synchronisation ne sont pas concernées.
        assert!(call(Method::Get, "/v1/save", Some(TOKEN), None, &[]).is_none());
        assert!(call(Method::Get, "/health", None, None, &[]).is_none());
        let page = call(Method::Get, "/", None, None, &[]).unwrap();
        assert_eq!(page.status, 200);
        assert!(page.content_type.starts_with("text/html"));

        let status = |r: Option<Reply>| r.map(|r| r.status);
        assert_eq!(
            status(call(Method::Get, "/v1/player", None, None, &[])),
            Some(401)
        );
        assert_eq!(
            status(call(Method::Get, "/v1/player", Some(TOKEN), None, &[])),
            Some(404)
        );
        let claim = |pseudo: &str| format!(r#"{{"pseudo":"{pseudo}"}}"#);
        assert_eq!(
            status(call(
                Method::Put,
                "/v1/player",
                Some(TOKEN),
                None,
                claim("Nova").as_bytes()
            )),
            Some(200)
        );
        let other = "Bearer ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100";
        assert_eq!(
            status(call(
                Method::Put,
                "/v1/player",
                Some(other),
                None,
                claim("nova").as_bytes()
            )),
            Some(409)
        );
        assert_eq!(
            status(call(
                Method::Put,
                "/v1/player",
                Some(other),
                None,
                claim("x").as_bytes()
            )),
            Some(400)
        );
        let me = call(Method::Get, "/v1/player", Some(TOKEN), None, &[]).unwrap();
        assert!(String::from_utf8_lossy(&me.body).contains("Nova"));

        let card = format!(
            r#"{{"act":1,"progress":0.4,"total_log":9.5,"supernovas":2,"crunches":0,"big_bangs":0,"achievements":7,"play_time":4000,"daily":{{"day":{},"score":12345.0}},"platform":"terminal","version":"0.18.0","sent_at":{now}}}"#,
            now / 86_400
        );
        let sig = leaderboard::hex(&leaderboard::hmac(&key, card.as_bytes()));
        assert_eq!(
            status(call(
                Method::Post,
                "/v1/score",
                Some(TOKEN),
                Some("00"),
                card.as_bytes()
            )),
            Some(400),
            "signature fausse"
        );
        assert_eq!(
            status(call(
                Method::Post,
                "/v1/score",
                Some(other),
                Some(&sig),
                card.as_bytes()
            )),
            Some(404),
            "pas de pseudo"
        );
        assert_eq!(
            status(call(
                Method::Post,
                "/v1/score",
                Some(TOKEN),
                Some(&sig),
                card.as_bytes()
            )),
            Some(204)
        );
        let id = slot_id(Some(TOKEN)).unwrap();
        let general = call(
            Method::Get,
            &format!("/v1/leaderboard?me={id}"),
            None,
            None,
            &[],
        )
        .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&general.body).unwrap();
        assert_eq!(json["entries"][0]["pseudo"], "Nova");
        assert_eq!(json["me"]["rank"], 1);
        let daily = call(Method::Get, "/v1/leaderboard?board=daily", None, None, &[]).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&daily.body).unwrap();
        assert_eq!(json["entries"][0]["score"], 12345.0);

        assert_eq!(
            status(call(Method::Delete, "/v1/player", Some(TOKEN), None, &[])),
            Some(204)
        );
        assert_eq!(
            status(call(Method::Get, "/v1/player", Some(TOKEN), None, &[])),
            Some(404)
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rate_limit_resets_every_minute() {
        let t0 = Instant::now();
        let mut l = RateLimit {
            per_min: 2,
            window: t0,
            counts: HashMap::new(),
        };
        assert!(l.allow("a", t0) && l.allow("a", t0));
        assert!(!l.allow("a", t0));
        assert!(l.allow("b", t0));
        assert!(l.allow("a", t0 + Duration::from_secs(61)));
    }
}
