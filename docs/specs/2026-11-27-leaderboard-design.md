# Leaderboard — design

Date : 2026-11-27 · Statut : validé, à planifier

## Objectif

Un classement public des joueurs, consultable sur un site et dans le jeu
(terminal et Android) :

- **Général** : avancée dans le jeu (% de progression globale).
- **Défi du jour** : meilleur score de chaque joueur pour une date donnée.

## Principes

- **Réservé aux joueurs synchronisés, et seulement avec un pseudo.** Le pseudo
  se saisit dans l'écran de synchronisation, au moment de créer ou de rejoindre
  un lien (et modifiable ensuite). Il est **facultatif pour la synchro** et
  **obligatoire pour le classement** : sans pseudo, rien n'est envoyé au
  classement.
- **La synchro reste chiffrée de bout en bout.** Le serveur ne lit jamais la
  partie : le jeu envoie à part une **fiche de score** en clair, signée.
- **Un seul service** : le relais existant (`crates/relay`) gagne l'API du
  classement et sert la page du site.
- Parité terminal / Android (voir ARCHITECTURE.md).

## Identité

L'identité du joueur est **dérivée du code de lien**, comme le jeton de synchro
(`relay_sync.rs`) :

- `player_secret` = SHA-256(« fonderie-sync/leaderboard » ‖ secret du lien),
  envoyé en `Authorization: Bearer` ;
- `player_id` = SHA-256 du `player_secret` (hexadécimal) : calculé par le
  serveur comme par le jeu, public, c'est la seule chose que le serveur garde.

Deux appareils liés ont donc la même identité sans rien stocker de plus.
Le pseudo est gardé par le serveur (clé : `player_id`) et recopié localement
dans `leaderboard.json` (à côté de `sync.json`, avec le code du lien auquel il
appartient) ; un appareil qui rejoint un lien récupère le pseudo par
`GET /v1/player`. **Délier** l'appareil ou créer un
nouveau lien donne une nouvelle identité ; l'ancienne entrée expire avec les
parties abandonnées (`EXPIRE_DAYS`).

Pseudo : 3 à 16 caractères (lettres, y compris accentuées, chiffres, `-`, `_`,
espace non initial ni final), unique sans tenir compte de la casse ni des
accents, au premier arrivé. Renommer libère l'ancien. Vider le pseudo retire le
joueur du classement (`DELETE /v1/player`).

## Fiche de score

Construite par `application` depuis le `GameSnapshot` :

| Champ | Contenu |
|---|---|
| `progress` | % de progression globale (`ProgressView::completion`) |
| `act` | 1 + (au moins un Big Crunch) + Grand Attracteur éveillé + Convergence + Cosmos éveillé (5 = Acte IV terminé) |
| `total_all` | poussière récoltée au total (log10, pour éviter les très grands nombres) |
| `supernovas`, `crunches`, `big_bangs` | compteurs |
| `achievements` | succès débloqués |
| `play_time` | temps de jeu en secondes |
| `daily` | `{ day, score }` du meilleur Défi du jour de la date, si joué |
| `platform` | `terminal` ou `android` |
| `version` | version du jeu |
| `sent_at` | horodatage d'envoi (secondes Unix) |

Signée par HMAC-SHA256 avec une clé propre au classement, indépendante de la
clé des saves : secret `FONDERIE_LEADERBOARD_KEY` du dépôt (64 caractères
hexadécimaux) injecté à la compilation, même valeur dans `LEADERBOARD_KEY` du
relais. Sans clé (compilation locale), une clé de développement est utilisée
et le relais officiel refuse ses fiches.

## Serveur (`crates/relay`)

Routes, en plus de la synchro :

- `GET /v1/player` (Bearer secret) → `{ "id", "pseudo" }` ou 404.
- `PUT /v1/player` (Bearer secret, corps `{ "pseudo" }`) → 200, 409 si le
  pseudo est pris, 400 s'il est invalide.
- `DELETE /v1/player` → 204 : retire le joueur des classements.
- `POST /v1/score` (Bearer secret, corps = fiche JSON, en-tête
  `X-Signature` = HMAC hexadécimal du corps) → 204 ; 400 si signature ou
  vraisemblance invalide ; 404 si aucun pseudo n'est enregistré ; 409 si la
  partie est moins jouée qu'une fiche déjà reçue.
- `GET /v1/leaderboard?board=general|daily&day=N&limit=N&me=ID` (jour = jours
  depuis 1970, comme le Défi du jour) → JSON : rang, pseudo, stats, et la ligne
  du joueur `me` même hors du top.
- `GET /` → page HTML du site, embarquée dans le binaire (`include_str!`) ;
  onglets Général et Défi du jour, choix de la date, `?p=<pseudo>` met une
  ligne en avant. Aucun script externe.

Stockage : fichiers JSON dans `DATA_DIR/leaderboard/` (joueurs, général, un
fichier par jour), écriture `.tmp` + renommage. Les classements quotidiens de
plus de 30 jours sont effacés.

Vraisemblance (fiche ignorée si elle échoue, réponse 400) :

- `play_time` ne recule jamais pour un même joueur (tolérance : une partie
  reprise par la synchro peut le faire reculer → on garde le maximum déjà vu et
  on n'enregistre pas la fiche) ;
- acte plafonné selon `play_time` : acte 2 ≥ 1 h 30, acte 3 ≥ 25 h, acte 4 ≥
  40 h, acte 5 ≥ 48 h (environ un tiers du temps de jeu du simulateur à 3 h
  par jour, le joueur le plus efficace en temps de jeu) ;
- `sent_at` à moins de 10 minutes de l'heure du serveur ;
- score du Défi du jour fini, entre 0 et 1e40, et `day` = aujourd'hui ou hier.

Tri : général par `act`, puis `progress`, puis `total_all` ; Défi du jour par `score` puis
ancienneté de l'envoi. Limite de débit : celle du relais.

Dépendances ajoutées au relais : `serde`, `serde_json`, `hmac` (ou HMAC écrit
avec `sha2`, comme `save_crypto`). La règle de CI « serde seulement dans
infrastructure ou relay » reste vraie.

## Client

**application**
- Port `LeaderboardBackend` : `claim(pseudo)`, `withdraw()`, `submit(card)`,
  `top(board, date, limit)`.
- `LeaderboardService` (thread, sur le modèle de `SyncService`) : envoie la
  fiche au lancement, toutes les 5 minutes, à la fin de chaque Défi du jour et
  à la fermeture, si un pseudo est enregistré ; garde en cache le dernier
  classement reçu ; publie des `LeaderboardEvent` (pseudo accepté / pris,
  classement reçu, erreur réseau).
- `score_card(&GameSnapshot)` : construction de la fiche (testable).

**infrastructure**
- `relay_leaderboard.rs` (feature `sync`, `ureq`) : dérivation de
  l'identité depuis le code de lien, signature, appels HTTP. Même adresse que
  la synchro (`FONDERIE_SYNC_URL`, sinon le relais officiel). Le pseudo est
  ajouté à `sync.json`.

**Terminal**
- Écran de synchronisation (`widgets/sync.rs`) : champ **Pseudo (facultatif,
  pour apparaître au classement)** à la création ou à la jonction d'un lien,
  et action pour le modifier ou le vider ensuite. Nouveau champ de saisie
  texte réutilisable.
- Nouvel écran **Classement** (Options › Classement : toutes les lettres
  sont déjà des raccourcis) : top 20 général et top 20 du jour (←/→), la
  ligne du joueur en évidence, son rang s'il est hors du top, et l'adresse du
  site. Sans lien ou sans pseudo, il explique comment entrer au classement.

**Android**
- Fenêtres de synchronisation (`SyncDialogs.kt`) : même champ pseudo
  facultatif, et modification ensuite, via une méthode UniFFI
  `Game::set_pseudo`.
- Sous-onglet **Classement** dans *Plus* : mêmes contenus en portrait, lien
  vers le site (ouverture du navigateur par `Request::OpenUrl`).

## Hors code

- `android/play/politique-confidentialite.md` et la section « sécurité des
  données » du Play Store : déclarer le pseudo et les statistiques publiés,
  uniquement si le joueur saisit un pseudo.
- README (section Synchronisation → Classement, hébergement du relais :
  `LEADERBOARD_KEY`), ARCHITECTURE (port, routes, fichiers), CHANGELOG
  (ordinateur et Android).
- CI : secret `FONDERIE_LEADERBOARD_KEY` (compilations), même valeur dans
  `LEADERBOARD_KEY` au déploiement du relais.

## Tests

- **relay** : validation et unicité du pseudo, signature refusée, chaque règle
  de vraisemblance, tri et `around`, purge des jours anciens, rendu de `/`.
- **application** : `score_card`, `LeaderboardService` avec faux backend
  (cadence, rien sans pseudo, cache).
- **infrastructure** : dérivation de l'identité (vecteurs fixes), signature.
- **presentation** : écran Classement et champ pseudo (`tests/render.rs`,
  `tests/readability.rs`), sous-onglet mobile (`tests/mobile.rs`).

## Hors périmètre

- Modération (suppression ou bannissement manuels).
- Classement sans synchronisation.
- Historique de progression par joueur.
