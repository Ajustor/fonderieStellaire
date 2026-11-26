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
//! Configuration (variables d'environnement) : `PORT` (8080), `DATA_DIR`
//! (`/data`), `MAX_BODY_KB` (1024), `RATE_PER_MIN` (60 requêtes par minute et
//! par adresse), `EXPIRE_DAYS` (365 : parties non synchronisées depuis plus
//! longtemps effacées), `TRUST_PROXY` (1 : adresse lue dans
//! `X-Forwarded-For`, derrière le proxy de Dokploy).

use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use sha2::{Digest, Sha256};
use tiny_http::{Header, Method, Request, Response, Server};

struct Config {
    port: u16,
    data: PathBuf,
    max_body: usize,
    rate_per_min: u32,
    expire: Duration,
    trust_proxy: bool,
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
        }
    }
}

/// Réponse indépendante de `tiny_http` (testable).
#[derive(Debug, PartialEq, Eq)]
struct Reply {
    status: u16,
    rev: Option<u64>,
    body: Vec<u8>,
}

impl Reply {
    fn text(status: u16, msg: &str) -> Self {
        Reply {
            status,
            rev: None,
            body: msg.as_bytes().to_vec(),
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
                Some((rev, data)) => Reply {
                    status: 200,
                    rev: Some(rev),
                    body: data,
                },
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
            return Reply {
                status: 409,
                rev: Some(rev),
                body: b"conflit".to_vec(),
            };
        }
        match self.write(&id, rev + 1, body) {
            Ok(()) => Reply {
                status: 200,
                rev: Some(rev + 1),
                body: b"ok".to_vec(),
            },
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
        for sub in dirs.flatten() {
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
    let _ = req.respond(resp);
}

fn serve(cfg: &Config, store: &Store, server: &Server) {
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
        let reply = store.handle(
            req.method(),
            req.url(),
            header(&req, "Authorization"),
            header(&req, "X-Base-Rev"),
            &body,
        );
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
    let store = Store {
        dir: cfg.data.clone(),
        max_body: cfg.max_body,
    };
    serve(&cfg, &store, &server);
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
