# ✦ Fonderie Stellaire ✦

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

### Binaires prêts à l'emploi

Sur la page **Releases** du dépôt, télécharge l'archive de ton système :

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
Windows Terminal.

### Image Docker (GitHub Container Registry)

```sh
docker run -it --rm -v fonderie:/data ghcr.io/<utilisateur>/<depot>:latest
```

`-it` est indispensable (c'est un jeu en terminal) ; le volume `fonderie` garde
ta sauvegarde entre deux parties. Remplace `<utilisateur>/<depot>` par le nom du
dépôt GitHub (en minuscules).

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
  À racheter après chaque Supernova.
- **Comètes** : toutes les 90 à 240 s. Capture-les avec `C` pendant leurs 15 s de
  traversée : *Tempête solaire* (production ×7 pendant 30 s) ou *Pluie d'étoiles*
  (gain immédiat).
- **Supernova** : `floor(√(récolte du cycle / 100 M))` fragments, chacun donnant
  **+5 %** de production pour toujours. Remet à zéro poussière, générateurs et
  améliorations.
- **Succès** : 12 succès, **+1 %** de production chacun.
- **Hors-ligne** : ta fonderie produit à 50 % pendant ton absence (12 h max).

## Sauvegarde

Sauvegarde automatique toutes les 30 s et à la fermeture, dans :

| Système | Emplacement |
|---|---|
| Windows | `%APPDATA%\fonderie-stellaire\save.json` |
| Linux | `~/.local/share/fonderie-stellaire/save.json` |

L'écriture est atomique (fichier `.tmp` puis renommage). Une sauvegarde
corrompue est renommée en `save.json.bak` et une nouvelle partie démarre.

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
  `ghcr.io/<utilisateur>/<depot>` (tags `0.1.0`, `0.1`, `latest`).

> Après la première publication, le paquet `ghcr.io` est **privé** par défaut :
> rends-le public dans *GitHub → ton profil → Packages → fonderie-stellaire →
> Package settings → Change visibility* pour que tout le monde puisse le
> télécharger.
