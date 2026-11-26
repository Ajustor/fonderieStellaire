# ✦ Fonderie Stellaire ✦

[![ci](https://github.com/Ajustor/fonderieStellaire/actions/workflows/ci.yml/badge.svg)](https://github.com/Ajustor/fonderieStellaire/actions/workflows/ci.yml)
[![release](https://img.shields.io/github/v/release/Ajustor/fonderieStellaire)](https://github.com/Ajustor/fonderieStellaire/releases/latest)

Un **idle game cosmique** qui tourne dans ton terminal, écrit en Rust.

Tu diriges une fonderie qui récolte de la **poussière d'étoile** (✦). Achète des
sondes, des drones, des sphères de Dyson… jusqu'aux Tisseurs de galaxies, attrape
les comètes au vol et, quand ta fonderie est assez puissante, déclenche une
**Supernova** pour renaître plus fort.

```
╭─ ✦ FONDERIE STELLAIRE ✦ ─────────────────────────────────────────────────────────────────────────╮
│  █  ▀▀█ ▀▀█   █ █ █▀▀ ▀▀█                                                              +3.00k /s │
│  █  █▀▀  ▀█   ▀▀█ ▀▀█   █  millions                                     Supernova : +2 fragments │
│  ▀  ▀▀▀ ▀▀▀ ▀   ▀ ▀▀▀   ▀  de poussières d'étoile                           ◆ 12 (+60%)   ★ 2/12 │
╰──────────────────────────────────────────────────────────────────────────────────────────────────╯
╭ Générateurs ─ achat ×1 ─────────────────────────────╮╭ Ciel ─────────────────────────────────────╮
│▶ ✧ Sonde                            ×42         27/s││    ░░░░·░░▒▒▒▒▒▒▒▒░░░░░░░░. .        ·    │
│    █████████████████░░░░░░░░░░░░░░░░░░░  5.31k (1%) ││  + ░░░░✦▒✦▒▒▒▒▒▒▒▒▒▒░░░░░○.░              │
│  ◇ Drone collecteur                 ×30         98/s││     ░░░░░·░░▒▒▒▒▒░░░░░░░░░░ ·   ·◇        │
│    █████████████████░░░░░░░░░░░░░░░░░░░  6.62k (3%) ││                 ░░░░░░░░··◆        ░░░.░░▒│
│  ◆ Moissonneur orbital              ×26        679/s││                  .            . ░░·░░▒▒▒◝.│
│    █████████████████░░░░░░░░░░░░░░░░░░  41.6k (23%) ││·.    .  .     .   .  ·     ·  . ░░░░░░.░▒▒│
│  ○ Station lunaire                  ×12        920/s││                    ✦.          · ░░░░▒▒▓▓▓│
│    █████████████████░░░░░░░░░░░░░░░░░░  64.2k (31%) ││                                     ░▒████│
│  ● Sphère de Dyson                  ×3       1.27k/s│╰───────────────────────────────────────────╯
│    █████████████████░░░░░░░░░░░░░░░░░░░  198k (42%) │╭ Améliorations (11) ───────────────────────╮
│  ▲ Forge stellaire                  ×0           0/s││▶ Gants d'astronaute                 1.00k │
│    █████████████████░░░░░░░░░░░░░░░░░░░  1.40M (0%) ││  Alliage renforcé · Drones collect… 1.50k │
│  ✦ Usine à nébuleuses               ×0           0/s││  Calibrage fin · Sondes             1.80k │
│    █████████████████░░░░░░░░░░░░░░░░░░░  20.0M (0%) ││  Calibrage fin · Drones collecteurs 12.0k │
│  ◈ Moteur quantique                 ×0           0/s││  Alliage renforcé · Moissonneurs o… 16.5k │
│    ███████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  330M (0%) ││ ───────────────────────────────────────── │
│   ??? ─ encore 2.13Md ✦ à récolter                  ││ +1% de la prod/s par clic                 │
╰─────────────────────────────────────────────────────╯╰───────────────────────────────────────────╯
╭ Production ─ max 0/s ───────────────────────────────╮╭ Journal ──────────────────────────────────╮
│ en attente de données…                              ││ 66:40 ✦ Bienvenue dans la Fonderie Stell… │
│                                                     ││                                           │
│                                                     ││                                           │
╰─────────────────────────────────────────────────────╯╰───────────────────────────────────────────╯
 [Espace] Récolter  [↑↓] Choisir  [Entrée] Acheter  [M] ×1  [C] Comète  [P] Supernova  [Q] Quitter
```

Et en vrai, tout bouge : intro en hyperespace, logo qui se décrypte, nébuleuse
qui dérive, soleil qui pulse, objets en orbite qui reflètent tes achats, comètes
à traînée, aurore boréale pendant les tempêtes solaires, feux d'artifice aux
paliers, toasts, et une séquence Supernova plein écran.

---

## Télécharger (sans installer Rust)

### Installation en une ligne (recommandé)

**Windows** (PowerShell) :

```powershell
irm https://raw.githubusercontent.com/Ajustor/fonderieStellaire/master/install.ps1 | iex
```

**Linux** :

```sh
curl -fsSL https://raw.githubusercontent.com/Ajustor/fonderieStellaire/master/install.sh | sh
```

Le script télécharge la dernière version, vérifie sa somme SHA-256 et l'installe
dans ton PATH (`%LOCALAPPDATA%\Programs\fonderie-stellaire` sous Windows,
`~/.local/bin` sous Linux). Ensuite, dans n'importe quel terminal :

```sh
fonderie-stellaire
```

### Binaires prêts à l'emploi

Sur la page [**Releases**](https://github.com/Ajustor/fonderieStellaire/releases/latest),
télécharge l'archive de ton système :

| Système | Archive |
|---|---|
| Windows (x64) | `fonderie-stellaire-x86_64-pc-windows-msvc.zip` |
| Linux (x64) | `fonderie-stellaire-x86_64-unknown-linux-gnu.tar.gz` |

Chaque archive est accompagnée de sa somme `.sha256`.

```sh
# Linux
tar -xzf fonderie-stellaire-x86_64-unknown-linux-gnu.tar.gz
./fonderie-stellaire
```

Sous Windows : décompresse le `.zip` puis lance `fonderie-stellaire.exe` depuis
Windows Terminal. Le binaire n'étant pas signé, SmartScreen peut afficher
« Windows a protégé votre ordinateur » : *Informations complémentaires* →
*Exécuter quand même*.

### Image Docker (GitHub Container Registry)

```sh
docker run -it --rm -v fonderie:/data ghcr.io/ajustor/fonderiestellaire:latest
```

`-it` est indispensable (c'est un jeu en terminal) ; le volume `fonderie` garde
ta sauvegarde entre deux parties.

## Compiler soi-même

### 1. Installer Rust (rustup)

**Windows**

1. Télécharge et lance `rustup-init.exe` depuis <https://rustup.rs>.
2. Accepte l'installation des *Build Tools* de Visual Studio si elle est proposée
   (nécessaire pour l'éditeur de liens MSVC).
3. Ouvre un **nouveau** terminal et vérifie : `cargo --version`.

**Linux**

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
cargo --version
```

(Il faut aussi un compilateur C pour l'édition de liens : `build-essential` sous
Debian/Ubuntu, `base-devel` sous Arch, `gcc` sous Fedora.)

### 2. Lancer le jeu

```sh
cargo run --release
```

Pour repartir de zéro (l'ancienne sauvegarde est conservée en `.bak`) :

```sh
cargo run --release -- --reset
```

> **Conseil Windows** : utilise **Windows Terminal** plutôt que l'ancien
> `cmd.exe` / la console classique : les couleurs RGB, les caractères Unicode et
> les animations y sont bien plus beaux. Taille conseillée : 120×35 ou plus
> (minimum 80×24).

---

## Touches

| Touche | Action |
|---|---|
| `Espace` | Récolter à la main (à maintenir une fois « Récolte continue » achetée) |
| `↑` `↓` / `k` `j` | Déplacer la sélection |
| `Tab` / `←` `→` | Changer de panneau (Générateurs ↔ Améliorations) |
| `Entrée` / `B` | Acheter l'élément sélectionné |
| `1` – `9` | Achat rapide du générateur n° N |
| `M` | Mode d'achat ×1 / ×10 / ×25 / MAX |
| `C` | Capturer la comète qui passe |
| `P` | Supernova (confirmation `O` / `N` / `Échap`) |
| `S` | Sauvegarder maintenant |
| `F` | Effets complets / effets réduits |
| `A` | Acheter toutes les améliorations abordables |
| `E` | Atelier stellaire : bonus permanents payés en fragments |
| `D` | Défis : un cycle avec contrainte, contre une récompense permanente |
| `U` | Big Crunch (après 15 Supernovas) |
| `L` | Laboratoire de matière noire (après 10 Big Crunch) |
| `V` | Univers parallèles et leurs Clés (après 10 Big Crunch) |
| `G` | Cosmogonie : Big Bang, forge primordiale, Grand Attracteur (après 10 Big Crunch) |
| `X` | Carte stellaire : expéditions (Acte III, après l'éveil du Grand Attracteur) |
| `R` | Options : images par seconde, effets réduits, recommencer une partie |
| `T` | Activer / arrêter l'automate (une fois acheté à l'Atelier) |
| `I` | Infos : progression, succès et fonctionnement de chaque mécanique |
| `PgUp` / `PgDn` | Remonter / redescendre dans le journal |
| `F3` | Overlay de debug (FPS, particules, temps de rendu) |
| `Q` / `Échap` | Sauvegarder et quitter |

Majuscules et minuscules sont acceptées.

## Règles en bref

- **Générateurs** : coût ×1.15 à chaque exemplaire ; production doublée à 25, 50,
  100, 200, 300, 400 et 500 exemplaires.
- **Améliorations** : ×2 / ×3 par générateur, multiplicateurs globaux, et bonus
  de clic (un pourcentage de ta production par seconde).
- **Récolte continue** (300 ✦, dès 150 ✦ récoltés) : maintenir `Espace` récolte
  tout seul, 10 fois par seconde. Sans elle, seuls les appuis distincts comptent.
  Elle est conservée après une Supernova.
- **Comètes** : toutes les 90 à 240 s. Capture-les avec `C` pendant leurs 15 s de
  traversée : *Tempête solaire* (production ×7 pendant 30 s) ou *Pluie d'étoiles*
  (gain immédiat). Plus rares : la *comète dorée* (×77 pendant 7 s) et la
  *comète noire* (+1 fragment, mais elle vole 10 % de ta poussière si tu la rates).
- **Supernova** : `floor(√(récolte du cycle / seuil))` fragments, avec un seuil
  de 10 M qui augmente de 25 % à chaque Supernova. Les fragments donnent +10 % de
  production chacun jusqu'à 5, puis le bonus croît en racine carrée. Remet à
  zéro poussière, générateurs et améliorations.
- **Générateurs de prestige** : *Trou blanc*, *Fabrique de quasars* et *Moteur à
  multivers* apparaissent après 1, 5 et 15 Supernovas.
- **Atelier stellaire** (`E`) : dépense tes fragments en 8 bonus permanents
  (départ équipé, comètes plus fréquentes, générateurs moins chers, automate
  d'achat…). Le bonus de production des fragments dépensés est **conservé**.
- **Constellations** (`E`, sous l'Atelier) : 6 améliorations à niveaux sans fin,
  de plus en plus chères (production, clics, comètes, hors-ligne, fragments,
  coûts). Chaque niveau allume une étoile dans le ciel.
- **Défis** (`D`, dès la 1re Supernova) : rejoue un cycle avec une contrainte
  (sans clic, sans amélioration, 3 générateurs, sans comète, ou contre la
  montre) pour gagner une récompense permanente.
- **Big Crunch** (`U`, après 15 Supernovas) : tout le niveau Supernova
  (fragments, Atelier, Constellations, Défis) repart de zéro contre de la
  *matière noire* : production ×2, ×3, ×4… et +25 % de fragments par Crunch.
- **Laboratoire** (`L`, après 10 Big Crunch) : la matière noire finance des
  recherches qui durent un temps réel (de 1 h à 12 h), une à la fois, même jeu
  fermé : automatismes (Supernova, Constellations, comètes), conservation au Big
  Crunch, et bonus de puissance.
- **Univers parallèles** (après 10 Big Crunch) : chaque Big Crunch ouvre un
  écran de portails pour choisir les règles du cycle suivant (Gravité forte,
  Temps dilaté, Nuée, Désert, Entropie). Atteindre l'objectif d'un univers
  rapporte sa **Clé**, un bonus permanent.
- **Big Bang** (30 Big Crunch) : les Big Crunch repartent de zéro contre des
  particules primordiales (+100 % de production, +25 % de fragments chacune).
  Elles servent à forger 3 générateurs primordiaux et à bâtir le **Grand
  Attracteur**, étape par étape et en temps réel. Sa dernière étape clôt
  l'Acte II.
- **Carte stellaire** (Acte III) : une carte générée pour chaque partie, à
  explorer en envoyant des flottes en expédition, en temps réel. Plus un
  système est dangereux, plus le voyage est long et risqué, et plus il
  rapporte (production, fragments, hors-ligne, comètes, flottes, gains
  immédiats).
- **Succès** : 45 succès, **+1 %** de production chacun.
- **Progression** : le pourcentage dans l'en-tête et l'écran `I` indiquent où tu
  en es (générateurs, succès, Atelier, poussière avant le prochain fragment).
- **Hors-ligne** : ta fonderie produit à 50 % pendant ton absence (12 h max).

## Mise à jour automatique

Au lancement, le jeu vérifie en arrière-plan (sans jamais le ralentir) s'il
existe une version plus récente sur GitHub. Si oui, il la télécharge, vérifie sa
somme SHA-256 et remplace l'exécutable, puis le jeu **redémarre tout seul**
sur la nouvelle version (après un compte à rebours de 5 s, et jamais au milieu
d'une Supernova ou d'un Big Bang). L'état s'affiche en bas à droite de l'écran
titre (`v0.9.0 · à jour`), et un message apparaît dans le journal.

- Pas de droits d'écriture sur le dossier du jeu, version compilée avec
  `cargo` ou image Docker : le jeu signale simplement qu'une nouvelle version
  existe (pour Docker : `docker pull`).
- Hors-ligne : rien ne se passe, le jeu démarre normalement.
- `fonderie-stellaire --version` affiche la version installée.

## Sauvegarde

Sauvegarde automatique toutes les 30 s et à la fermeture, dans :

| Système | Emplacement |
|---|---|
| Windows | `%APPDATA%\fonderie-stellaire\save.json` |
| Linux | `~/.local/share/fonderie-stellaire/save.json` |

L'écriture est atomique (fichier `.tmp` puis renommage). Une sauvegarde
corrompue est renommée en `save.json.bak` et une nouvelle partie démarre.

- **Recommencer** : touche `R` (Options) → « Recommencer une nouvelle
  partie », ou `fonderie-stellaire --reset` (l'ancienne passe en `.bak`).
- **Changer d'ordinateur** : `fonderie-stellaire --export ma-partie.json`, puis
  sur l'autre machine `fonderie-stellaire --import ma-partie.json`.

Quand l'équilibrage change trop (comme en v0.9.0), les sauvegardes d'un
format plus ancien (`SAVE_RESET_BELOW`) sont effacées au lancement, et un
message sur l'écran titre l'explique au joueur.

## Développement

```sh
cargo test --workspace          # domaine, service (fakes), fx, rendu TestBackend
cargo clippy --workspace --all-targets
cargo test -p presentation --test render -- --nocapture   # affiche les écrans rendus
```

L'architecture (Clean Architecture en workspace Cargo) est décrite dans
[ARCHITECTURE.md](ARCHITECTURE.md).

### Intégration continue et publication

- **`.github/workflows/ci.yml`** — à chaque push et pull request : formatage,
  clippy (warnings = erreurs), règles d'architecture, tests et build release sous
  Linux **et** Windows, et build de l'image Docker.
- **`.github/workflows/release.yml`** — en poussant un tag de version :

  ```sh
  git tag v0.1.0
  git push origin v0.1.0
  ```

  le workflow teste, compile les binaires Windows et Linux, crée la GitHub
  Release avec les archives et leurs sommes SHA-256, et publie l'image sur
  `ghcr.io/ajustor/fonderiestellaire` (tags `0.1.0`, `0.1`, `latest`).

> Après la première publication, le paquet `ghcr.io` est **privé** par défaut :
> rends-le public dans *GitHub → ton profil → Packages → fonderie-stellaire →
> Package settings → Change visibility* pour que tout le monde puisse le
> télécharger.
