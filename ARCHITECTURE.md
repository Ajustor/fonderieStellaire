# Architecture

Fonderie Stellaire suit une **Clean Architecture** en quatre couches. La séparation
est **imposée par le compilateur** : chaque couche est une crate du workspace, et
une crate ne peut utiliser que ce qu'elle déclare dans son `Cargo.toml`.

```
        ┌──────────────────────────────────────────────┐
        │  bin (composition root : câble tout)         │
        │   ┌──────────────────────────────────────┐   │
        │   │ presentation (ratatui/crossterm)     │   │
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
| `infrastructure` | `application`, `domain`, `serde`, `serde_json`, `dirs` | terminal |
| `presentation` | `application`, `domain`, `ratatui`, `crossterm` | fichiers, `serde` |
| `bin` (`fonderie-stellaire`) | toutes | logique métier |

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
  des reliques équipées et bonus d'ensemble) et l'agrégat `GameState`
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
- **infrastructure** — implémentations des ports : `JsonSaveRepository`
  (miroir `serde` du DTO avec `#[serde(default)]`, écriture `.tmp` + renommage,
  `.bak` si corrompu), `SystemClock`, `XorShift64`, et `GitHubUpdater`
  (`github_updater.rs` : API des releases via `ureq`, vérification SHA-256 avec
  `sha2`, remplacement de l'exécutable ; sous Windows l'ancien devient `.old`).
- **presentation** — boucle de jeu, entrées, rendu et effets. Ne modifie jamais
  l'état : elle envoie des `Command` et lit le `GameSnapshot`. Tous les états
  purement visuels (tweens, particules, focus, défilement, toasts, journal)
  vivent ici.
- **bin** — composition root : instancie les adapters, les injecte dans
  `GameService`, gère `--reset`, `--export`, `--import`, `--simulate` et
  `--version` (version injectée
  par la CI depuis le tag via `FONDERIE_VERSION`), lance `presentation::run`.

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
 Chaque frame (≈ 30 FPS) : GameService.tick(dt) ──► événements ──► react()
                           GameService.fill_snapshot(&mut snap)
                           render(snap, Visuals) ──► Buffer ratatui ──► terminal
```

Toutes les animations sont **basées sur le temps** (`now` en secondes depuis le
lancement), jamais sur le nombre de frames. La logique avance avec le delta temps
réel mesuré par `Instant`.

## Organisation de la présentation

```
presentation/src/
├── app.rs          boucle, Intent → Command, réactions aux événements
├── terminal.rs     raw mode, écran alternatif, panic hook, garde RAII
├── input.rs        KeyEvent → Intent (ne garde que Press) + HoldDetector
│                   (appui distinct ou touche maintenue, pour la récolte continue)
├── layout.rs       découpage de l'écran de jeu
├── ui.rs           focus, sélections, défilement, popups (Supernova, Infos,
│                   Atelier), défilement du journal
├── visuals.rs      état des effets de l'écran de jeu
├── screens/        title (hyperespace + logo), game, supernova, too_small
├── widgets/        header, generators, upgrades, sky, sparkline, journal,
│                   help_bar, toast, modal, codex (écran Infos), shop (Atelier)
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

## Guides

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
