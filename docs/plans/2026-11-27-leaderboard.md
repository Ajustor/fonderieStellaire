# Leaderboard — plan d'implémentation

**But :** classement général et du Défi du jour, servi par le relais, alimenté
par les joueurs synchronisés qui ont choisi un pseudo (terminal et Android).

**Architecture :** le relais gagne un module `leaderboard` (API JSON + page
HTML). Le jeu envoie une fiche de score signée (HMAC, clé dérivée du secret des
versions officielles) depuis un `LeaderboardService` en thread (application),
qui passe par le port `LeaderboardBackend` implémenté par `RelayLeaderboard`
(infrastructure). Identité dérivée du code de lien.

**Spec :** `docs/specs/2026-11-27-leaderboard-design.md`.

Vérification à la fin de chaque tâche : `cargo test --workspace`,
`cargo clippy --workspace --all-targets`, `cargo fmt --all`.

---

## Tâche 1 — Relais : stockage et règles du classement

**Fichiers :** créer `crates/relay/src/leaderboard.rs` ; modifier
`crates/relay/Cargo.toml` (`serde` avec `derive`, `serde_json`).

Contenu :

- `pub struct Card { act: u8, progress: f64, total_log: f64, supernovas: u32,
  crunches: u32, big_bangs: u32, achievements: u32, play_time: f64,
  daily: Option<Daily { day: u64, score: f64 }>, platform: String,
  version: String, sent_at: u64 }` (serde).
- `pub fn valid_pseudo(&str) -> Result<String, &'static str>` : trim, 3–16
  caractères, `char::is_alphanumeric`, `-`, `_`, espaces simples internes.
- `pub fn pseudo_key(&str) -> String` : minuscules, accents français retirés
  (àâä→a, éèêë→e, îï→i, ôö→o, ùûü→u, ç→c, ÿ→y, œ→oe, æ→ae).
- `pub fn hmac(key, data) -> [u8; 32]` (RFC 2104 avec `sha2`) et
  `verify(key, body, sig_hex) -> bool` (comparaison en temps constant).
- `pub fn plausible(card, prev_play_time: f64, now: u64) -> Result<(), Reject>`
  avec `Reject::{Invalid(&str), Older}` : champs finis, `progress ∈ [0,1]`,
  `act ∈ 1..=5`, `MIN_PLAY = [0, 0, 5400, 90000, 144000, 172800]` par acte,
  `|sent_at - now| ≤ 600`, `daily.day ∈ {today-1, today}`, score fini dans
  `[0, 1e40]` ; `play_time + 60 < prev_play_time` → `Older`.
- `pub struct Board { dir, players: HashMap<id, Player>, daily:
  HashMap<day, HashMap<id, DailyEntry>> }` avec `Player { pseudo, card:
  Option<Card>, best_play_time, updated }`. Chargement/écriture JSON dans
  `DATA_DIR/leaderboard/players.json` et `daily-<jour>.json` (`.tmp` +
  renommage).
- Méthodes : `player(id)`, `claim(id, pseudo) -> Result<String, Claim>`
  (`Taken`, `Invalid`), `withdraw(id)`, `submit(id, card, now) ->
  Result<(), Submit>` (`NoPseudo`, `Rejected(Reject)`), `standings(board, day,
  limit, me) -> serde_json::Value`, `expire(now, max_age)` (joueurs inactifs,
  jours de plus de 30 jours).
- Tri général `(act, progress, total_log)` décroissants ; du jour `score`
  décroissant puis `at` croissant ; `rank` à partir de 1 ; `me` ajouté même
  hors du top.

Tests (dans le module) : pseudo valide / invalide / doublon accentué
(« Éloïse » vs « eloise ») ; renommage libère l'ancien ; signature refusée ;
chaque règle de `plausible` ; tri et `me` hors du top ; persistance (recharger
un `Board` depuis le dossier) ; expiration.

## Tâche 2 — Relais : routes, site, configuration

**Fichiers :** modifier `crates/relay/src/main.rs` ; créer
`crates/relay/src/site.html`.

- `Reply` gagne `content_type: &'static str`.
- `Config` : `LEADERBOARD_KEY` (64 hex ; sinon clé de développement =
  SHA-256(`fonderie-leaderboard/` ‖ clé de dév. du jeu), avec un message au
  démarrage).
- Routage dans `serve` : `/`, `/v1/player`, `/v1/score`, `/v1/leaderboard` →
  `Board` ; le reste → `Store::handle` inchangé. Lecture de `X-Signature`.
  L'identifiant joueur = `slot_id(auth)` (empreinte du jeton, fonction
  existante).
- `site.html` : page autonome (CSS et JS en ligne), onglets Général / Défi du
  jour, sélecteur de jour (aujourd'hui et 29 jours avant), `?p=pseudo` en
  évidence, thème sombre aux couleurs du jeu, lisible sur téléphone.
- Expiration du classement dans la tâche quotidienne existante.

Tests : `handle_leaderboard` (fonction pure prenant méthode, chemin, requête,
auth, signature, corps, `now`) : 401 sans jeton, claim 200/409/400, score
204/400/404/409, GET JSON, `/` renvoie du HTML.

## Tâche 3 — Application : fiche de score, port et service

**Fichiers :** créer `crates/application/src/leaderboard.rs` ; exporter dans
`lib.rs`.

- `ScoreCard` (mêmes champs que `Card`, `platform: &'static str`) et
  `score_card(snap, today, platform, version, now) -> ScoreCard` ;
  `act_of(snap)`.
- `Board { General, Daily }`, `Entry { rank, pseudo, act, progress,
  total_log, supernovas, crunches, big_bangs, play_time, platform, score, me }`,
  `Standings { board, day, entries, me: Option<Entry> }`.
- `LbError { Invalid(String), Taken, NotLinked, Network(String) }`.
- `trait LeaderboardBackend: Send { fn linked(&self) -> bool; fn
  pseudo(&self) -> Option<String>; fn refresh_pseudo(&mut self) ->
  Result<Option<String>, LbError>; fn claim(&mut self, &str) ->
  Result<String, LbError>; fn withdraw(&mut self) -> Result<(), LbError>; fn
  submit(&self, &ScoreCard) -> Result<(), LbError>; fn standings(&self,
  Board, u64) -> Result<Standings, LbError>; }`
- `LeaderboardService` (thread, rafales fusionnées comme `SyncService`) :
  `disabled()`, `spawn(backend)`, `available()`, `pseudo()`, `busy()`,
  `claim(String)`, `withdraw()`, `submit(ScoreCard)` (ignoré sans pseudo),
  `fetch(Board, day)`, `refresh()`, `standings(Board)`, `status()`,
  `poll() -> Option<LeaderboardEvent>`, `flush(timeout)`.
- `LeaderboardEvent { Claimed(String), Taken, Invalid(String), Withdrawn,
  Pseudo(Option<String>), Submitted, Standings(Standings), Failed(String) }`.

Tests : `act_of` sur des snapshots construits ; `score_card` (jour du défi
seulement si `daily_best > 0`) ; service avec faux backend : rien n'est envoyé
sans pseudo, claim met à jour le pseudo, `Taken` le laisse vide, la dernière
fiche d'une rafale seule part, standings mis en cache.

## Tâche 4 — Infrastructure : `RelayLeaderboard`

**Fichiers :** créer `crates/infrastructure/src/relay_leaderboard.rs` (feature
`sync`) ; modifier `lib.rs`, `save_crypto.rs` (`pub fn derived_key(label)` =
SHA-256(label ‖ secret du jeu)), `relay_sync.rs` (`pub fn
leaderboard_secret(code) -> Option<String>` = hex de
`derive("leaderboard", secret)`).

- État local `leaderboard.json` : `{ code, pseudo }` ; le pseudo ne vaut que si
  `code` = code de `sync.json` actuel.
- `linked()` relit `sync.json` ; requêtes `ureq` avec `Authorization: Bearer`
  ; signature `X-Signature` = HMAC(`derived_key("fonderie-leaderboard/")`,
  corps).
- Statuts : 409 → `Taken` (claim) ; 400 → `Invalid(message du relais)` ;
  transport → `Network`.

Tests : dérivation stable (vecteur fixe), signature identique à celle du
relais (même vecteur que dans la tâche 1), lecture/écriture de
`leaderboard.json` et invalidation au changement de lien.

## Tâche 5 — Présentation commune et terminal

**Fichiers :** modifier `app.rs`, `ui.rs`, `input.rs` (rien à mapper : la
saisie lit les `KeyEvent` bruts), `widgets/sync.rs`, `widgets/options.rs`,
`screens/game.rs`, `widgets/mod.rs` ; créer `widgets/leaderboard.rs`.

- `App` : `leaderboard: LeaderboardService`, `set_leaderboard`,
  `lb_timer` (envoi toutes les `LEADERBOARD_INTERVAL` = 300 s, au lancement,
  à `DailyEnded`, à la fermeture ; jamais pendant un Défi du jour),
  `leaderboard_view() -> LeaderboardView`, `set_pseudo(&str)`,
  `clear_pseudo()`, réactions aux `LeaderboardEvent` (toasts), après
  `SyncEvent::Joined`/`Linked` → `refresh()`, avant `sync_unlink` →
  `withdraw()`.
- Saisie du pseudo (terminal) : `ui.pseudo_input: Option<String>` ; dans la
  fenêtre de synchro, `[N]` ouvre la saisie (appareil lié). Pendant la saisie,
  `on_terminal_event` capte caractères, Retour arrière, Entrée (valider), Échap
  (annuler). La fenêtre affiche « Pseudo : … » et l'invite.
- Options : nouvel élément **Classement** (`LEADERBOARD_ITEM = 4`,
  `RESET_ITEM = 5`, `ITEMS = 6`) qui ouvre `ui.leaderboard`.
- `widgets/leaderboard.rs` : fenêtre (Général / Du jour, ←/→), 20 lignes
  (rang, pseudo, acte ou score, temps de jeu), ligne du joueur en doré, rang
  hors top, adresse du site, explication si non lié / sans pseudo.
- `bin` : brancher `RelayLeaderboard` à côté de `RelaySync`, `flush` à la
  fermeture.

Tests : `tests/render.rs` (fenêtre Classement remplie et vide, fenêtre de
synchro avec saisie), `tests/readability.rs` (nouvel écran aux 4 tailles).

## Tâche 6 — Mode mobile et Android

**Fichiers :** `mobile/mod.rs` (`Request::{Pseudo, OpenSite}`,
`Hit::LeaderboardBoard(usize)`), `mobile/screens.rs` (sous-onglet
« Classement » dans *Plus*, avant « Sauver » ; `render_leaderboard`),
`mobile/input.rs`, `crates/android/src/lib.rs` (`Request::{Pseudo, OpenSite}`,
`LeaderboardInfo`, `leaderboard_info()`, `set_pseudo(String)`,
`clear_pseudo()`), `SyncDialogs.kt` (`askPseudo()`, proposé après création ou
jonction d'un lien et depuis la fenêtre « Appareil lié »), `GameActivity.kt`
(`PSEUDO` → `askPseudo`, `OPEN_SITE` → `Intent.ACTION_VIEW`).

Tests : `tests/mobile.rs` (onglet Classement rendu, hit-test des boutons),
`crates/android` (le constructeur branche le service), `GlyphCoverageTest`
(aucun nouveau glyphe hors police).

## Tâche 7 — Documentation et publication

- README : Synchronisation → Classement ; Héberger le relais →
  `LEADERBOARD_KEY` (`printf 'fonderie-leaderboard/%s' "$FONDERIE_SAVE_KEY" |
  sha256sum`).
- ARCHITECTURE : relais (routes), port et service, fichiers, tests.
- CHANGELOG : section `v0.18.0` (classement + bonus des fragments à +1 %).
- `android/play/politique-confidentialite.md` et `fiche-fr.txt` si utile.
