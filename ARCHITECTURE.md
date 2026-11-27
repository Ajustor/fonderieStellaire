# Architecture

Fonderie Stellaire suit une **Clean Architecture** en quatre couches. La séparation
est **imposée par le compilateur** : chaque couche est une crate du workspace, et
une crate ne peut utiliser que ce qu'elle déclare dans son `Cargo.toml`.

```
        ┌──────────────────────────────────────────────┐
        │  bin / android (composition roots)           │
        │   ┌──────────────────────────────────────┐   │
        │   │ presentation (terminal ou mobile)    │   │
        │   │ infrastructure (fichiers, horloge…)  │   │
        │   │   ┌──────────────────────────────┐   │   │
        │   │   │ application (cas d'usage,    │   │   │
        │   │   │ ports = traits)              │   │   │
        │   │   │   ┌──────────────────────┐   │   │   │
        │   │   │   │ domain (règles pures)│   │   │   │
        │   │   │   └──────────────────────┘   │   │   │
        │   │   └──────────────────────────────┘   │   │
        │   └──────────────────────────────────────┘   │
        └──────────────────────────────────────────────┘
```

## Règle de dépendance

**Les dépendances pointent uniquement vers l'intérieur.**

| Crate | Dépend de | Interdit |
|---|---|---|
| `domain` | *rien* (aucune crate externe, pas même `serde`) | I/O, horloge, aléa implicite |
| `application` | `domain` | terminal, fichiers, `serde` |
| `infrastructure` | `application`, `domain`, `serde`, `serde_json`, `dirs`, `sha2` ; `ureq` et `getrandom` derrière les features `updater` / `sync` | terminal |
| `presentation` | `application`, `domain`, `ratatui`, `qrcode` ; `crossterm` derrière la feature `terminal` (par défaut) | fichiers, `serde` |
| `bin` (`fonderie-stellaire`) | toutes | logique métier |
| `android` (`fonderie-android`, lib `libfonderie.so`) | `domain`, `application`, `infrastructure` (sans `updater`), `presentation` (sans `terminal`), `uniffi` | logique métier, `crossterm` |
| `uniffi-bindgen` | `uniffi` | outil de build : génère les liaisons Kotlin |
| `relay` (`fonderie-relay`) | `tiny_http`, `sha2`, `serde`, `serde_json` | tout le reste : serveur à part, hors des couches du jeu |

Vérification rapide :

```sh
grep -rlE "ratatui|crossterm" crates | grep -v crates/presentation   # → rien
grep -rlE "serde|std::fs"     crates | grep -v crates/infrastructure # → rien
```

### Ce que fait chaque couche

- **domain** — règles pures et déterministes : catalogue (`catalog.rs`), coûts et
  paliers (`generator.rs`), production (`economy.rs`), améliorations, succès,
  comètes (dont les comètes rares), prestige, Atelier stellaire (`perk.rs`),
  Constellations (`constellation.rs`), Défis (`challenge.rs`), Big Crunch
  (matière noire, dans `GameState`), Laboratoire (`lab.rs` : recherches en temps
  réel, avancées par `tick` et par `apply_offline`), Univers parallèles
  (`universe.rs` : règles appliquées via `GameState::rules`, Clés via
  `key_product`), Big Bang et Grand Attracteur (`cosmos.rs` : particules,
  générateurs primordiaux avec `GeneratorDef::particle_cost`, étapes en temps
  réel avancées comme les recherches), Carte stellaire (`starmap.rs` : carte
  générée depuis une graine sauvegardée, expéditions dont l'issue est tirée au
  départ), Reliques (`relic.rs` : collection, rareté tirée au retour, effets
  des reliques équipées et bonus d'ensemble), Événements cosmiques et
  Convergence (`cosmic.rs` : dilemmes, bonus temporaires, finale de l'Acte III),
  Mégastructures (`megastructure.rs` : grille, effets de voisinage, Éveil du
  Cosmos), Défi du jour (`daily.rs` : règles tirées de la date ; l'échange
  avec la partie principale est fait par `GameService`) et l'agrégat
  `GameState`
  (dont l'automate d'achat et « tout acheter »). Le temps (`dt`) et les tirages
  aléatoires (`roll: f64`) arrivent **en paramètre**. Chaque opération pousse des
  `DomainEvent` dans un `Vec`.
- **application** — `GameService` orchestre les cas d'usage :
  `handle(Command)`, `tick(dt)`, `load()`, `save()`, `fill_snapshot()`. Il parle au
  monde extérieur via les **ports** (`SaveRepository`, `Clock`, `RandomSource`),
  convertit l'état en `SaveData` (DTO pur Rust) et gère l'historique de production
  et l'autosave. Il expose un `GameSnapshot` en lecture seule (dont la
  progression globale). `UpdateService` (`updates.rs`) vérifie les mises à jour
  dans un thread, via le port `UpdateProvider`, et publie des `UpdateEvent`.
  `SyncService` (`sync.rs`) et `LeaderboardService` (`leaderboard.rs`) font de
  même pour la synchronisation et le classement.
- **infrastructure** — implémentations des ports : `JsonSaveRepository`
  (miroir `serde` du DTO avec `#[serde(default)]`, écriture `.tmp` + renommage,
  `.bak` si corrompu), `SystemClock`, `XorShift64`, et `GitHubUpdater`
  (`github_updater.rs` : API des releases via `ureq`, vérification SHA-256 avec
  `sha2`, remplacement de l'exécutable ; sous Windows l'ancien devient `.old` ;
  `ApkReleases` pour Android installé par l'APK : simple avis, la page de la
  version s'ouvre par `Request::OpenUpdate`, pas pour le Play Store).
- **presentation** — boucle de jeu, entrées, rendu et effets. Ne modifie jamais
  l'état : elle envoie des `Command` et lit le `GameSnapshot`. Tous les états
  purement visuels (tweens, particules, focus, défilement, toasts, journal)
  vivent ici. Un même `App` sert les deux plateformes : écrans du terminal
  (feature `terminal`, boucle crossterm) et **mode mobile** (`mobile/`, activé
  par `App::set_mobile(true)`), qui rend dans un `Buffer` ratatui sérialisé par
  `cells.rs`.
- **bin** — composition root du terminal : instancie les adapters, les injecte dans
  `GameService`, gère `--reset`, `--export`, `--import`, `--simulate`,
  `--link` / `--unlink` et `--version` (version injectée
  par la CI depuis le tag via `FONDERIE_VERSION`), lance `presentation::run`.
- **android** — composition root Android : mêmes adapters (sauvegarde dans le
  dossier privé de l'app, pas d'`updater`), objet `Game` exposé à Kotlin par
  UniFFI (`frame`, toucher, onglets, pause/reprise, export/import,
  synchronisation). Voir [Version Android](#version-android).

## Flux d'une action

```
 Touche ──► input.rs ──► Intent ──► app.rs ──► Command ──► GameService.handle()
                                                              │
                                                              ▼
                                            GameState (domaine) modifié
                                                              │
                                            Vec<GameEvent> ◄──┘ (DomainEvent + sauvegarde…)
                                                              │
                         app.rs::react() ◄────────────────────┘
                          ├─► Visuals  : particules, flash, shake, roulement du compteur…
                          ├─► Toasts   : succès, paliers, comètes
                          └─► Journal  : une ligne colorée par événement
                                                              │
 Chaque frame (30 ou 60/s) : GameService.tick(dt) ──► événements ──► react()
                           GameService.fill_snapshot(&mut snap)
                           render(snap, Visuals) ──► Buffer ratatui ──► terminal
                                                              └──► cells.rs ──► Kotlin (Android)
```

Sur Android, le même chemin part d'un **toucher** : `GameActivity.kt` envoie
la cellule touchée à `Game::touch_down/up` → `mobile/input.rs` résout la zone
par le `HitMap` enregistré au rendu → `Hit` → `Intent` / `Command`. La boucle
est cadencée par le `Choreographer` (`Game::frame`, qui ne renvoie que les
lignes modifiées).

Toutes les animations sont **basées sur le temps** (`now` en secondes depuis le
lancement), jamais sur le nombre de frames. La logique avance avec le delta temps
réel mesuré par `Instant`.

## Organisation de la présentation

```
presentation/src/
├── app.rs          boucle, Intent → Command, réactions aux événements
│                   (commun au terminal et au mode mobile)
├── terminal.rs     raw mode, écran alternatif, panic hook, garde RAII
│                   (feature `terminal`)
├── input.rs        KeyEvent → Intent (ne garde que Press) + HoldDetector
│                   (appui distinct ou maintenu, indépendant du clavier)
├── layout.rs       découpage de l'écran de jeu
├── ui.rs           focus, sélections, défilement, popups (Supernova, Infos,
│                   Atelier), défilement du journal
├── visuals.rs      état des effets de l'écran de jeu
├── draw.rs         wrap / paragraph / fit / truncate (textes non coupés)
├── nav.rs          navigation du terminal : écrans de la barre d'onglets
│                   (`ScreenId`, débloqués seulement), `Tab`, menu, aide
├── leaderboard.rs  classement côté App : envoi de la fiche, pseudo (saisie
│                   au clavier), fenêtre, réactions aux réponses du relais
├── cells.rs        Buffer → octets (grille de cellules pour Android)
├── screens/        title (hyperespace + logo), game, supernova, bigbang,
│                   ending (générique), too_small
├── widgets/        header, generators, upgrades, sky, sparkline, journal,
│                   help_bar, toast, modal, codex (Infos), shop (Atelier,
│                   Constellations), options, challenges, lab, portals,
│                   cosmos (Cosmogonie), starmap, relics, event, ceremony,
│                   mega, daily, sync, leaderboard, nav (onglets, menu, aide)
├── mobile/         mode mobile (Android), jamais utilisé par le terminal
│   ├── mod.rs      Tab, Page, Hit, Detail, Confirm, Tip, Request, MobileUi
│   ├── screens.rs  onglets Fonderie / Usines / Améliorer / Cosmos / Plus,
│   │               fiches détail, barre native, bulles, carte d'absence ;
│   │               Plus › Sauver : classement, synchro, export / import
│   ├── cosmos.rs   pages du Cosmos (Événement, Défi du jour, Défis, Labo,
│   │               Cosmogonie, Carte stellaire, Reliques, Mégastructures)
│   │               et confirmation « glisser pour confirmer »
│   └── input.rs    toucher, maintien, défilement, balayage, retour Android
├── fx/             moteur d'effets générique (aucune connaissance du jeu)
│   ├── easing.rs   linear, cubic, sine, back, elastic, bounce
│   ├── tween.rs    Tween<T: Lerp> (f64, (f64,f64), Rgb, Color)
│   ├── timeline.rs then / with / delay / at
│   ├── particles.rs pool fixe de 400 particules, burst / fountain / trail
│   ├── color.rs    lerp, dégradés, HSL, dim / glow
│   ├── noise.rs    value noise 2D + fBm
│   └── effects.rs  flash, fade, offset (shake/slide), glitch, shimmer,
│                   dissolve, typewriter, contour de bordure
├── theme.rs        palette centralisée (aucune couleur en dur ailleurs)
├── bigfont.rs      police 3 lignes
└── fmt.rs          formats courts / longs / durées
```

## Version Android

```
android/app/src/main/java/ajustor/fonderie/
├── GameActivity.kt  cycle de vie (pause = sauvegarde), boucle Choreographer,
│                    gestes → Game::touch_*, vibrations, export/import (SAF)
├── CellView.kt      SurfaceView qui dessine la grille de cellules
├── CellGrid.kt      décodage du format de cells.rs
├── BoxGlyphs.kt     cadres et blocs dessinés en primitives (PRIMITIVES)
├── TabBar.kt        barre d'onglets native (Game::tabs, pastilles)
└── SyncDialogs.kt   fenêtres de synchronisation (QR code, scan, code) et
                     saisie du pseudo du classement
android/app/src/test/…/GlyphCoverageTest.kt   chaque glyphe non ASCII des
                     sources Rust/Kotlin doit être couvert par une police
                     embarquée ou par BoxGlyphs.PRIMITIVES
android/play/        fiche Play Store (textes, icône, bannière, captures,
                     politique de confidentialité)
android/tools/       check_16k.py (pages de 16 Ko), make_icon.py, make_feature.py
```

- Gradle compile `libfonderie.so` avec `cargo ndk` (ABI `arm64-v8a` et
  `x86_64`) puis génère les liaisons Kotlin avec `uniffi-bindgen`. Le
  `versionName` vient de `Cargo.toml` (ou de `FONDERIE_VERSION`), le
  `versionCode` du nombre de commits (ou de `ANDROID_VERSION_CODE` en CI).
- `.github/workflows/android.yml` : tests unitaires, APK et AAB signés,
  vérification des pages de 16 Ko ; sur un tag `v*`, APK joint à la release et
  AAB envoyé sur le Play Store (piste interne).
- Les écrans du terminal qui n'ont pas (encore) d'équivalent mobile restent
  accessibles par `Hit::Key(Intent)` : ils s'affichent tels quels en grille
  dense (`Game::wants_dense`). C'est un repli, pas une cible.

## Parité terminal / Android

**Toute évolution du jeu sort sur les deux plateformes.** Le domaine,
l'application et l'infrastructure sont partagés : seule la présentation
diffère. Dès qu'un changement touche un écran, une touche, un texte d'aide ou
une option du terminal, il faut faire son pendant mobile dans la même version.

Checklist à chaque changement côté terminal :

1. **Nouvel écran ou nouvelle fenêtre** (`widgets/`, `screens/`) : lui donner
   une entrée mobile — un sous-onglet (`mobile::sub_tabs`), une `Page` du
   Cosmos (`mobile/cosmos.rs` : `Page`, `page_screen`, `render_page`, ligne
   dans `cosmos_entries`) ou une fiche `Detail`. Réutiliser les fonctions de
   texte du widget terminal ; seule la mise en page change.
2. **Nouvelle touche / nouvelle action** (`input.rs`, `Intent`) : une zone
   touchable (`Hit`) enregistrée au rendu dans le `HitMap`, et sa résolution
   dans `mobile/input.rs`. Action lourde ou irréversible → une `Confirm`
   (glisser pour confirmer). Achat répétable → `Hit::repeats`.
3. **Textes d'aide** : pas de touche clavier sur mobile (`touchify`, ou texte
   propre au mobile). Les fiches et pages ne doivent rien couper.
4. **Nouveau déblocage** : pastille d'onglet (`TabInfo::badge`) et, si c'est
   une étape clé du début de partie, une bulle `Tip`.
5. **Nouvel événement de domaine** : `App::react_domain` est commun ; vérifier
   l'effet sur l'écran mobile (zones de `ui.layout` remplies par
   `mobile::render`) et une vibration (`take_haptic`) si l'événement compte.
6. **Nouvelle option** : la ligne dans `render_options` (mobile) ; si elle doit
   survivre côté Android hors sauvegarde, la persister dans `GameActivity`.
7. **Nouvelle demande à la plateforme** (fichier, partage, réseau…) :
   variante de `mobile::Request`, méthode UniFFI dans `crates/android`, et
   traitement dans `GameActivity.handleRequests`.
8. **Nouveau caractère non ASCII** : `GlyphCoverageTest` doit passer (sinon
   l'ajouter à `BoxGlyphs` ou choisir un autre glyphe).
9. **Tests** : un cas dans `crates/presentation/tests/mobile.rs` (rendu
   portrait + hit-test), et `cargo test --workspace` + `./gradlew
   testReleaseUnitTest` au vert.
10. **Journal des versions** : la note du `CHANGELOG.md` dit comment faire sur
    ordinateur **et** sur Android.

Les changements purement terminal (raw mode, `--simulate`, mise à jour
automatique…) n'ont pas de pendant ; les changements purement Android
(Gradle, Play Store) non plus.

## Guides

Chaque guide ci-dessous concerne le cœur ou le terminal : appliquer ensuite
la [checklist de parité](#parité-terminal--android).

### Ajouter un générateur

1. `crates/domain/src/catalog.rs` : ajouter une ligne au tableau `GENERATORS`
   (nom, pluriel, symbole à largeur simple, coût de base, prod/s), sa
   description dans `GENERATOR_FLAVORS` et son verrou dans `GENERATOR_UNLOCKS`
   (nombre de Supernovas nécessaires, 0 sinon). Les 4 améliorations du
   générateur sont générées automatiquement (`gen{N}_t{0..3}`).
2. `crates/presentation/src/theme.rs` : ajouter sa couleur à `GENERATOR_COLORS`.
3. Les effets visuels par générateur sont dimensionnés par `theme::GEN_SLOTS`
   (16) : l'augmenter au-delà de 16 générateurs.
4. Optionnel : un glyphe d'orbite particulier dans `widgets/sky.rs`
   (`orbit_glyph`).

Les sauvegardes existantes restent compatibles (`owned` est redimensionné).

### Ajouter une amélioration

- Pour un nouveau niveau par générateur : une ligne dans `GENERATOR_TIERS`.
- Pour une amélioration globale ou de clic : une ligne dans `GLOBAL_UPGRADES` ou
  `CLICK_UPGRADES`.
- Pour un nouvel **effet** : ajouter une variante à `upgrade::Effect`, la prendre
  en compte dans `economy.rs`, et écrire un test.

L'identifiant texte (`key`) doit rester **stable** : c'est lui qui est sauvegardé.

### Ajouter un effet visuel

1. Si l'effet est générique (nouvel easing, nouveau post-effet), l'ajouter dans
   `fx/` avec un test unitaire — sans dépendre du jeu.
2. Déclencher l'effet depuis `App::react` / `react_domain` (`app.rs`) en réponse
   au `DomainEvent` concerné, en stockant un horodatage dans `Visuals`.
3. Le dessiner dans le widget concerné en calculant l'avancement à partir de
   `ctx.now` (jamais du nombre de frames), et respecter `ctx.reduced`
   (mode effets réduits : pas de shake, glitch ni nébuleuse, particules ÷ 4).

### Ajouter une amélioration à l'Atelier

1. `crates/domain/src/catalog.rs` : une ligne dans `PERKS` (clé stable, nom,
   description, symbole à largeur simple, coût en fragments, effet).
2. Pour un nouvel **effet** : une variante dans `perk::PerkEffect`, appliquée
   dans `GameState` (ou `economy.rs`), avec un test. Le `match` de l'écran
   Atelier n'a rien à changer : il affiche le catalogue.

### Ajouter une Constellation

1. `crates/domain/src/catalog.rs` : une ligne dans `CONSTELLATIONS` (clé
   stable, nom, description d'un niveau, symbole, coût de base, croissance du
   coût, niveau max éventuel, effet **par niveau**).
2. Pour un nouvel effet : une variante de `ConstellationEffect`, appliquée dans
   `GameState` (voir `constellation_sum`), et son texte cumulé dans
   `widgets/shop.rs` (`constellation_total`).

### Ajouter un Défi

1. `crates/domain/src/catalog.rs` : une ligne dans `CHALLENGES` (clé stable,
   nom, règle et récompense en clair, symbole, objectif, Supernovas requises,
   `Restriction`, `ChallengeReward`).
2. Une nouvelle contrainte : variante de `challenge::Restriction`, appliquée là
   où l'action a lieu dans `GameState` (voir `active_restriction`), en émettant
   `ChallengeBlocked`. Une nouvelle récompense : variante de `ChallengeReward`,
   prise en compte via `challenge_rewards` (dans `GameState` ou `economy.rs`).

### Ajouter une recherche au Laboratoire

1. `crates/domain/src/catalog.rs` : une ligne dans `LAB` (clé stable, nom,
   description, symbole, coût en matière noire, durée en secondes, prérequis
   éventuel, effet).
2. Un nouvel effet : une variante de `lab::LabEffect`, appliquée dans
   `GameState` via `lab_effects` (les automatismes dans `run_lab_automation`,
   et `is_toggleable` s'ils peuvent être mis en pause). L'écran du Laboratoire
   le range dans sa section d'après l'effet (`widgets/lab.rs`, `SECTIONS`).

### Ajouter un univers parallèle

1. `crates/domain/src/catalog.rs` : une entrée dans `universes()` (clé stable,
   nom, règles en clair, symbole, `UniverseRules`, objectif de la Clé en
   Supernovas, nom et bonus de la Clé).
2. `crates/presentation/src/theme.rs` : sa palette dans `UNIVERSE_PALETTES`
   (couleurs du vortex, du badge, de l'anneau de sa Clé).
3. Optionnel : un effet sur le ciel dans `widgets/sky.rs` (voir `tnow`).

### Ajouter une étape au Grand Attracteur

1. `crates/domain/src/catalog.rs` : une ligne dans `ATTRACTOR` (nom, effet en
   clair, coût en particules, heures de construction, `AttractorEffect`).
   L'ordre du tableau est l'ordre de construction ; la dernière étape déclenche
   la fin de l'Acte II.
2. Un nouvel `AttractorEffect` s'applique dans `GameState` (voir
   `attractor_product`).

### Vérifier le rythme d'une version

`cargo run --release -- --simulate 3` rejoue une partie complète (vrai code du
domaine, stratégie automatique de `application::simulator`) pour un joueur qui
joue 3 h par jour, et affiche quand chaque étape est atteinte. Sans argument :
jeu continu. À relancer après tout changement d'équilibrage.

### Synchronisation entre appareils

- **application** (`sync.rs`) : port `SyncBackend` (lien enregistré, `pull`,
  `push` avec la révision de base), `reconcile` (envoi optimiste ; en cas de
  conflit, `prefer_remote` garde la partie commencée le plus tard
  (`SaveData::started_at`, posé par « Nouvelle partie »), puis celle au plus
  grand `play_time`, qui n'avance qu'en jeu) et `SyncService` (thread, rafales fusionnées, envois
  suspendus entre l'annonce `SyncEvent::Adopt` et `adopted()`, pour qu'une
  partie sur le point d'être remplacée n'écrase pas le relais).
- **infrastructure** (`relay_sync.rs`) : code de lien (120 bits en base 32 de
  Crockford + caractère de contrôle), jeton et clés dérivés par SHA-256,
  partie scellée avec `save_crypto::seal` (chiffrement et HMAC à clés
  distinctes) par-dessus le chiffrement habituel, lien gardé dans `sync.json`.
- **presentation** : `App::update_sync` (envoi toutes les `SYNC_INTERVAL`
  secondes, à la sauvegarde immédiate et au retour d'arrière-plan ; reprise
  d'une partie hors des grandes séquences et du Défi du jour), écran
  `widgets/sync.rs` (QR code en demi-blocs) ; Android passe par
  `Request::Sync` et des fenêtres natives (`SyncDialogs.kt`).
- **relay** : `GET`/`PUT /v1/save` avec `X-Rev` / `X-Base-Rev` (409 en cas de
  conflit), empreinte du jeton comme nom de fichier, limite de débit et
  expiration des parties abandonnées.

L'adresse du relais vient de `FONDERIE_SYNC_URL` (environnement au lancement,
puis compilation), sinon du relais officiel (`relay_sync::DEFAULT_RELAY`).

### Classement en ligne

Réservé aux appareils synchronisés qui ont choisi un pseudo. La partie reste
chiffrée : le jeu envoie à part une **fiche de score** signée.

- **application** (`leaderboard.rs`) : `ScoreCard` et `score_card` (acte =
  1 + premier Big Crunch + Grand Attracteur + Convergence + Éveil du Cosmos),
  port `LeaderboardBackend`, `LeaderboardService` (thread, rafales fusionnées,
  rien n'est envoyé sans pseudo, classements gardés en cache).
- **infrastructure** (`relay_leaderboard.rs`) : identité dérivée du code de
  lien (`relay_sync::leaderboard_token`, distinct du jeton de la partie ;
  identifiant public = SHA-256 du jeton), pseudo recopié dans
  `leaderboard.json` avec le code du lien, signature HMAC-SHA256 avec la clé
  `FONDERIE_LEADERBOARD_KEY` (compilation, vérifiée par une fonction `const` :
  une valeur mal formée fait échouer la compilation), sinon la clé de
  développement.
- **presentation** : `leaderboard.rs` (envoi au lancement, toutes les
  `LEADERBOARD_INTERVAL` secondes, après le Défi du jour, à la fermeture ;
  jamais pendant un Défi du jour ; quitter le classement avant de délier),
  `widgets/leaderboard.rs` (Options › Classement), pseudo dans
  `widgets/sync.rs` (`N`). Mobile : en haut de *Plus › Sauver* ;
  `Request::Pseudo` / `Request::OpenSite` → `SyncDialogs.askPseudo` et le
  navigateur.
- **relay** (`leaderboard.rs`, `site.html`) : `GET`/`PUT`/`DELETE /v1/player`,
  `POST /v1/score` (`X-Signature`), `GET /v1/leaderboard`, page sur `/`.
  Classement général : acte, puis `rank_points` (progression × 1000 + 100 par
  Big Bang + 10 par Big Crunch + 1 par Supernova), puis poussière.
  Vérifie signature et vraisemblance (`plausible` : acte selon le temps de
  jeu, horloge, Défi du jour d'aujourd'hui ou d'hier, temps de jeu qui ne
  recule pas). Fichiers JSON dans `DATA_DIR/leaderboard/`.

La clé du relais (`LEADERBOARD_KEY`) est la même valeur que le secret
`FONDERIE_LEADERBOARD_KEY` du dépôt, injecté à la compilation par
`release.yml`, `android.yml` et en secret BuildKit (image Docker) ;
**changer l'un sans l'autre** fait refuser toutes les fiches des versions
officielles. Elle est indépendante de la clé des sauvegardes.

### Chiffrement des sauvegardes

`infrastructure/src/save_crypto.rs` chiffre (flux dérivé de SHA-256) et signe
(HMAC-SHA256) la sauvegarde. La clé des versions officielles vient du secret
`FONDERIE_SAVE_KEY` du dépôt, injecté à la compilation par `release.yml`
(binaires) et en secret BuildKit (image Docker). **Ne jamais changer ce
secret** : toutes les sauvegardes existantes deviendraient illisibles. Sans
le secret (compilation locale), une clé de développement est utilisée.

### Écrire un texte sans qu'il soit coupé

Utiliser `draw::wrap` / `draw::paragraph` plutôt que `draw::truncate`, et
dimensionner les fenêtres d'après leur contenu. `draw::fit` coupe sans le
signaler, à réserver aux listes dont la ligne sélectionnée est montrée en
entier ailleurs. Le test `tests/readability.rs` rend chaque écran à 4 tailles
et échoue au moindre texte coupé.

### Ajouter un événement de domaine

1. Ajouter la variante dans `domain/src/events.rs` et l'émettre depuis
   `GameState`.
2. Le `match` exhaustif de `App::react_domain` refusera de compiler tant que
   l'événement n'a pas de réaction visuelle : c'est voulu.
3. `react_domain` sert aussi le mode mobile : vérifier que l'effet tombe au
   bon endroit sur l'écran portrait, et ajouter une vibration si l'événement
   compte (étape 5 de la [checklist de parité](#parité-terminal--android)).

## Tests

- **domain** : coûts, achat ×N / MAX, paliers, production, fragments, succès,
  comètes, gains hors-ligne, rythme du début de partie.
- **application** : `GameService` avec `FakeClock`, `FakeRng`,
  `InMemoryRepository` (achat, hors-ligne, Supernova, autosave, aller-retour de
  sauvegarde, sauvegarde corrompue).
- **infrastructure** : repository JSON sur fichiers temporaires, xorshift.
- **presentation** : easing, tweens, timeline, particules, bruit, post-effets,
  formatage, et rendu complet dans un `TestBackend` 100×30 / 80×24 / 120×40
  (`tests/render.rs`, avec mesure du temps de rendu et une session complète
  jusqu'à la Supernova), audit de lisibilité à 4 tailles de terminal
  (`tests/readability.rs`), et profils ignorés par défaut (`--ignored`) :
  temps d'image, volume envoyé au terminal, images du Big Bang.
- **relay** : pseudos, signature, vraisemblance, tri, persistance, routes ;
  bout en bout avec un vrai relais : `FONDERIE_RELAY_TEST=http://127.0.0.1:8080
  cargo test -p infrastructure --test relay_e2e -- --ignored` (synchro et
  classement).
- **mode mobile** : `crates/presentation/tests/mobile.rs` rend chaque onglet
  et page en portrait et vérifie le hit-test (`cargo test -p presentation
  --test mobile -- --nocapture` affiche les écrans).
- **android** : `crates/android` (grille portrait puis lignes modifiées
  seulement, pause = sauvegarde, export/import) ; côté Kotlin,
  `./gradlew testReleaseUnitTest` dans `android/` (dont
  `GlyphCoverageTest`), et `android/tools/check_16k.py` sur l'APK en CI.
