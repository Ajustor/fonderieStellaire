# Journal des versions

## v0.4.0

### Nouveautés
- **Constellations** (dans l'Atelier, `E`) : 6 améliorations **à niveaux**,
  payées en fragments, dont le coût augmente à chaque niveau. Les fragments ont
  toujours une utilité, même une fois l'Atelier complet.
  - *Le Forgeron* : +20 % de production par niveau.
  - *La Main* : +1 % de la prod/s par clic, par niveau.
  - *La Comète* : tempête solaire +1× et pluie d'étoiles +25 %, par niveau.
  - *Le Sablier* : +2 h de production hors-ligne par niveau.
  - *La Nova* : +10 % de fragments par Supernova, par niveau.
  - *Le Bâtisseur* : générateurs 3 % moins chers par niveau.
- Chaque niveau de Constellation allume une étoile d'une constellation
  dessinée dans le ciel.
- **3 nouveaux générateurs** débloqués par les Supernovas : *Trou blanc*
  (1re Supernova), *Fabrique de quasars* (5e) et *Moteur à multivers* (15e),
  avec leurs 4 améliorations chacun.
- **4 nouveaux succès** (28 au total).

### Équilibrage
- Fin de l'emballement des Supernovas : jusqu'ici les fragments doublaient à
  chaque cycle et les cycles tombaient à moins d'une minute après ~4 h de jeu.
  - Le bonus des fragments reste de +10 % chacun jusqu'à 5 fragments, puis
    croît en racine carrée (40 fragments : +140 % au lieu de +400 %).
  - Chaque Supernova rend la suivante 25 % plus exigeante.
  - Résultat (simulation) : le début de partie ne change pas, et les cycles se
    stabilisent autour de 15–25 min ; le Multivers arrive vers 9 h de jeu.
- Les sauvegardes existantes sont conservées, mais leur bonus de fragments est
  recalculé avec la nouvelle courbe.

### Corrections
- La ligne mystère « ??? » reste visible quand la liste des générateurs remplit
  tout le panneau.

## v0.3.1

### Corrections
- La valeur d'un clic est de nouveau affichée dans l'en-tête au-delà du million.
- En 80 colonnes, la ligne des fragments ne fait plus disparaître la valeur d'un
  clic ni le mot d'échelle (« millions »…) : chaque ligne de l'en-tête a sa
  propre place.
- Les barres de progression des générateurs gardent toutes la même largeur,
  quels que soient le coût et le mode d'achat ; coûts et pourcentages alignés.

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
