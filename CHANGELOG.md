# Journal des versions

## v0.3.0

### Nouveautés
- **Atelier stellaire** (`E`) : 8 améliorations permanentes achetées avec les
  fragments de Supernova (Héritage, Poigne stellaire, Aimant à comètes, Tempête
  prolongée, Ingénierie frugale, Résonance, Automate, Nova dorée). Dépenser des
  fragments ne retire pas leur bonus de production.
- **Automate** : une fois acheté à l'Atelier, achète chaque seconde le
  générateur le plus rentable ; `T` pour l'activer ou l'arrêter.
- **Tout acheter** (`A`) : achète toutes les améliorations abordables, de la
  moins chère à la plus chère.
- **Progression** : pourcentage de complétion dans l'en-tête, et section
  « Progression » dans l'écran Infos (`I`) : générateurs, succès, Atelier,
  totaux récoltés, poussière restante avant le prochain fragment.
- **12 nouveaux succès** (24 au total).
- **Prochaine amélioration** : quand il n'y a rien à acheter, le panneau montre
  la prochaine amélioration et une barre de progression vers sa condition.
- **Journal défilable** avec `PgUp` / `PgDn`.

### Équilibrage
- Fragments : `floor(√(récolte du cycle / 10 M))` (au lieu de 100 M) et
  **+10 %** de production par fragment (au lieu de +5 %). La première Supernova
  rentable arrive vers 1 h 20 au lieu de plus de 3 h.
- « Récolte continue » est conservée après une Supernova.

### Corrections
- La sauvegarde automatique n'écrit plus dans le journal toutes les 30 s.
- La fonderie continue de produire pendant l'écran titre.
- Ajout du fichier `LICENSE` (MIT).
- Les notes de version de chaque release viennent maintenant de ce fichier.

## v0.2.0

### Nouveautés
- **Descriptions** : chaque générateur et chaque amélioration explique ce qu'il
  fait, avec son gain concret (« → +27/s »).
- **Écran Infos** (`I`) : succès obtenus ou non, et fonctionnement de chaque
  mécanique.
- **Mise à jour automatique** depuis les releases GitHub, appliquée au prochain
  lancement (`--no-update` pour la désactiver, `--version` pour l'afficher).
- **Récolte continue** : amélioration qui permet de maintenir `Espace`.
- **Panneau Production** : couleurs par système producteur, axes, tendance.
- Scripts d'installation en une ligne (Windows et Linux), image Docker sur
  `ghcr.io`.

## v0.1.0

Première version : 9 générateurs, améliorations, comètes, Supernova, 12 succès,
interface animée, sauvegarde et gains hors-ligne.
