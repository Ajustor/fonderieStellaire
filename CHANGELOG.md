# Journal des versions

## v0.7.0 — Acte II, partie 1 : le Laboratoire

### Nouveautés
- **Laboratoire de matière noire** (`L`), ouvert au 10e Big Crunch. La matière
  noire devient dépensable (son bonus de production est conservé) dans
  12 **recherches en temps réel** : une à la fois, et elles **avancent même
  jeu fermé**. Durée totale : 68 h, coût total : 21 matières noires.

  | Section | Recherches |
  |---|---|
  | Automatismes (activables / en pause) | Filet à comètes, Constellations automatiques, Supernova automatique |
  | Conservation au Big Crunch | Mémoire de l'Atelier, Défis gravés, Départ lancé (3 Supernovas) |
  | Puissance | Toucher du vide, Densité I (×2), Densité II (×3), Écho stellaire (+50 % fragments), Horizon des événements (hors-ligne 100 %, 48 h), Aimant d'ombre (comètes rares ×2) |

- La recherche en cours s'affiche sur la bordure de l'en-tête, avec son temps
  restant ; la matière noire libre à côté du bonus.
- **2 nouveaux succès** (37 au total) ; les recherches comptent dans la
  progression.

### Équilibrage
- Le rythme du Laboratoire est fixé par la durée des recherches, et non par la
  vitesse des Big Crunch (qui s'accélèrent trop après le 10e pour servir de
  repère) : environ 3 jours, jeu fermé compris.

## v0.6.0

### Nouveautés
- **Big Crunch** (`U`) : le second niveau de prestige. Une fois 15 Supernovas
  atteintes (Multivers débloqué), on peut replier l'univers sur lui-même :
  - **remis à zéro** : Supernovas, fragments, Atelier, Constellations, Défis ;
  - **conservé** : succès, statistiques, comètes capturées, Récolte continue ;
  - **gagné** : 1 *matière noire* par Big Crunch, qui donne **production
    ×(1 + matière noire)** (×2, ×3, ×4…) et **+25 % de fragments** par
    matière noire.
- Confirmation détaillée (ce qui est perdu, gardé, gagné) et séquence plein
  écran en **implosion** : le soleil s'effondre, l'onde converge vers le centre,
  puis « BIG CRUNCH ».
- La matière noire s'affiche dans l'en-tête (`✶ 3 (×4)`) et sur l'écran titre.
- **3 nouveaux succès** (35 au total) : 1, 5 et 10 Big Crunch.

### Équilibrage
- Réglé par simulation pour atteindre **10 Big Crunch en environ 22 h de jeu
  actif** (quelques jours en comptant le hors-ligne) : le 1er arrive vers
  8 h 30, puis environ 4 h, 2 h 30, 1 h 45… jusqu'à environ 35 min.

## v0.5.0

### Nouveautés
- **Défis** (`D`) : un cycle joué avec une contrainte. Réussir l'objectif
  rapporte une récompense **permanente**. Lancer un Défi relance le cycle (une
  Supernova a lieu d'abord si elle rapporte des fragments) ; on peut
  l'abandonner à tout moment. Lancer et abandonner demandent une confirmation.

  | Défi | Contrainte | Objectif | Récompense | Débloqué après |
  |---|---|---|---|---|
  | Doigts liés | aucune récolte manuelle | 100 M | production ×1,25 | 1 Supernova |
  | Ascèse | aucune amélioration | 10 M | améliorations −25 % | 1 Supernova |
  | Minimaliste | Sondes, Drones, Moissonneurs seulement | 100 M | ces 3 générateurs ×3 | 1 Supernova |
  | Nuit noire | aucune comète | 1 Md | comètes 25 % plus fréquentes | 3 Supernovas |
  | Course contre la montre | objectif en moins de 15 min | 100 M | +1 fragment par Supernova | 5 Supernovas |

- Pendant un Défi, la bordure de l'en-tête montre son avancement (et le temps
  restant pour la Course).
- **Comètes rares** :
  - *Comète dorée* (10 %) : capturée, production ×77 pendant 7 s.
  - *Comète noire* (5 %) : capturée, +1 fragment permanent ; ratée, elle vole
    10 % de ta poussière.
  - Elles ont leurs couleurs, leur bannière et leur alerte.
- **4 nouveaux succès** (32 au total) ; les Défis comptent dans la progression.

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
