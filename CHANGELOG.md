# Journal des versions

## v0.16.0 — Acte IV, fin : le Cosmos vivant

### Nouveautés
- **Tes mégastructures dans le ciel** : chacune devient une étoile de sa
  couleur, reliée à ses voisines, en constellation au-dessus de la fonderie.
- **Éveil du Cosmos** : Cœur stellaire bâti, les six types présents et au
  moins 30 structures, lance-le avec `Entrée` sur le Cœur (écran `W`).
  **72 h en temps réel**, même jeu fermé ; pendant l'Éveil, on ne démolit
  plus. Ensuite : **fin de l'Acte IV**, production ×100, et le jeu continue.
- **Grand final** : un générique qui retrace toute ta partie, acte par acte,
  avec le temps de jeu de chaque grande étape.
- Statistiques : la fin de l'Acte IV rejoint les grandes étapes.
- **1 nouveau succès** (55 au total).

### Animations
- Pendant l'Éveil, une **vague de lumière** balaie la grille et la
  constellation du ciel ; une fois le Cosmos éveillé, elles scintillent aux
  **couleurs du spectre**.
- Le générique final s'ouvre sur une galaxie aux couleurs de l'Acte IV.

### Rythme (simulateur, 3 h de jeu par jour)
- Fin de l'Acte IV vers le **49e jour** (environ 7 semaines).

## v0.15.0 — Acte IV, partie 2 : le Défi du jour

### Nouveautés
- **Défi du jour** (touche `Y`, à l'Acte IV) : dix minutes dans un **univers
  de poche**, une partie vierge avec **deux règles tirées de la date**, les
  mêmes pour tous les joueurs ce jour-là (production ×3 mais générateurs plus
  chers, comètes ×5, aucune amélioration, entropie…). Le score est la
  poussière récoltée.
- Ta partie principale attend de côté et **continue d'avancer en temps réel**
  pendant le défi (recherches, chantiers, flottes, matière exotique) ; la
  sauvegarde la protège.
- Le **premier défi du jour** rapporte jusqu'à **12 ◈ de matière exotique**
  selon le score ; ensuite, on rejoue pour battre son **record du jour**.
- Un **code de score** (`FS<jour>-<score>-<contrôle>`) à partager.
- La touche `y` n'est plus un raccourci pour « oui » (`o` le reste).

### Animations
- Entrée et sortie de l'univers de poche par un **iris** lumineux qui s'ouvre
  depuis le centre de l'écran.
- Compte à rebours et score dans l'en-tête pendant le défi.
- **Carte du résultat** : le score défile, record du jour, récompense et
  code à partager, feux d'artifice.

## v0.14.0 — Acte IV, partie 1 : les Mégastructures

### Nouveautés
- **Acte IV** : après la Convergence, la **matière exotique** ◈ afflue, en
  temps réel et même jeu fermé.
- **Mégastructures** (touche `W`) : une grille de 9×5 où bâtir six
  structures. **Les voisines comptent** :

  | Structure | Effet | Voisinage |
  |---|---|---|
  | ◇ Collecteur | +1 matière exotique par heure | renforcé par les Relais |
  | ◆ Relais | — | +25 % pour chaque structure voisine |
  | ▲ Forge-monde | +25 % de production | ×1,5 à côté d'un Collecteur |
  | ★ Archive | +20 % de fragments | ×1,5 à côté d'une Forge-monde |
  | ◎ Phare | expéditions plus rapides et plus sûres | ×2 au bord de la grille |
  | ✶ Cœur stellaire (unique) | production ×1,5 par type voisin différent | — |

  Chaque exemplaire coûte plus cher que le précédent ; `D` démolit une
  structure (sans remboursement) pour réorganiser la grille.
- **3 nouveaux succès** (54 au total).

### Animations
- Grille animée : les liaisons entre structures voisines brillent (celles des
  Relais pulsent), la case visée palpite, une nouvelle structure apparaît
  avec un balayage lumineux et une onde ; feux d'artifice pour le Cœur.

### Rythme (simulateur, 3 h de jeu par jour)
- Cœur stellaire vers le 43e jour, 20 structures vers le 44e, grille complète
  vers le 53e.

## v0.13.0 — Bon retour, forgeron

### Nouveautés
- **Rapport d'absence** sur l'écran titre : la production de ton absence
  défile jusqu'à son total, puis chaque nouvelle s'écrit à son tour :
  expéditions rentrées, reliques trouvées, recherches et chantiers terminés,
  Convergence, événements manqués, succès débloqués.
- **Pilote automatique des flottes** (après 10 systèmes explorés, `A` sur la
  Carte stellaire) : les flottes repartent toutes seules, explorer d'abord puis
  prospecter. **Jeu fermé aussi** : elles enchaînent leurs voyages pendant
  ton absence.
- **Statistiques** en haut de l'écran Infos (`I`) : courbe de ta production
  sur toute la partie (échelle logarithmique), record de production, et temps
  de jeu de tes grandes étapes (1re Supernova, 1er Big Crunch, 1er Big Bang,
  fins d'actes).
- **Couleurs** dans les Options (`R`) : normales, **daltonien** (les verts
  deviennent bleus, les rouges orangés : plus de paires rouge/vert à
  distinguer) ou **contraste élevé** (texte plus clair, fonds plus sombres).
  Appliqué à tout l'écran, réglage sauvegardé.

## v0.12.2 — Barre d'aide contextuelle

> La v0.12.1 n'a pas été publiée (échec de la CI) : son contenu est ici.

- La barre du bas **n'affiche plus que les actions possibles maintenant** :
  `[C] Comète !` quand une comète passe, `[Z] Événement !` quand un dilemme
  attend, `[P] Nova` quand une Supernova rapporte, `[U] Crunch` quand le Big
  Crunch est possible, `[A] Tout` quand une amélioration est abordable, et
  l'Atelier, les Défis, le Labo, les univers, la carte ou les reliques une fois
  débloqués. Les autres touches marchent toujours.
- Une action qui vient d'apparaître **brille un instant** ; les urgences
  (comète, événement) restent mises en évidence.
- `[S] Sauver` et `[F] Effets` quittent la barre (sauvegarde automatique,
  effets dans les Options `[R]`) : les touches restent actives.

## v0.12.0 — Acte III, fin : Événements cosmiques et Convergence

### Nouveautés
- **Événements cosmiques** : à l'Acte III, toutes les 2 à 4 h de jeu, un
  dilemme se présente (signal d'une civilisation, tempête de plasma, comète
  géante, épave ancienne, pulsar emballé, trou de ver, marchand d'étoiles,
  supernova voisine). Chaque choix a ses conséquences : production ou
  fragments multipliés pendant un temps, gains immédiats, reliques, flottes
  accélérées… certains sont **risqués** (chance affichée, et ce qui arrive
  sinon). On a 1 h pour répondre ; `Z` pour revenir à l'événement.
- **Convergence** : carte complète et 16 reliques réunies, on la lance depuis
  la Fonderie sur la Carte stellaire. **48 h en temps réel** (même jeu fermé),
  puis **fin de l'Acte III**, avec son générique. Ensuite, la production est
  multipliée par 10, et les reliques légendaires restent à chasser.
- **3 nouveaux succès** (51 au total), toujours triés par difficulté.
- **Options** : nouveau réglage **60 images par seconde** (60, 30, 20 ou 15).
  La touche `R` (Options) est maintenant toujours affichée dans la barre
  d'aide.
- **Sauvegarde chiffrée et signée** : elle n'est plus lisible en clair, et une
  sauvegarde modifiée à la main est refusée (mise de côté en `.bak`, nouvelle
  partie). Les anciennes sauvegardes en clair sont reprises puis réécrites
  chiffrées.

### Animations
- **Événement** : la fenêtre descend du haut de l'écran avec un rebond, son
  titre se décrypte comme un signal capté, sa bordure palpite.
- **Fin de l'Acte III** : convergence en spirale aux couleurs de la carte,
  galaxie qui s'ouvre, générique avec les chiffres de ta partie.

### Corrections
- Le renvoi à la ligne garde le retrait des lignes indentées.
- Le symbole des événements occupait deux colonnes dans certains terminaux :
  remplacé par `✹`.

### Rythme (simulateur, 3 h de jeu par jour)
- **Fin de l'Acte III vers le 40e jour.**

## v0.11.0 — Acte III, partie 2 : les Reliques

### Nouveautés
- **Reliques** (touche `H`) : 16 reliques réparties en **4 ensembles**
  (Forgeron, Voyageur, Éclipse, Origines), chacune en **4 raretés**
  (commune, rare, épique, légendaire, de ×1 à ×4 sur son effet). Effets :
  production, fragments, clics, hors-ligne, comètes, vitesse et réussite des
  expéditions.
- Les expéditions réussies rapportent une relique **une fois sur deux** ; les
  systèmes dangereux donnent plus souvent des reliques rares.
- **Prospection** : renvoyer une flotte vers un système déjà exploré, plus
  vite et plus sûrement, pour une relique à coup sûr.
- **3 à 5 emplacements** (+1 à 10 systèmes explorés, +1 à la carte
  complète). **Bonus d'ensemble** avec 3 reliques du même ensemble :
  production ×1,5 (Forgeron), expéditions plus rapides et plus sûres
  (Voyageur), hors-ligne +25 % (Éclipse), production ×2 (Origines).
- Un **doublon** devient un **Éclat** : +1 % de production chacun, pour
  toujours. Une relique trouvée à une rareté supérieure remplace l'ancienne.
- **3 nouveaux succès** (48 au total).

### Animations
- **Relique trouvée** : une carte se retourne au centre de l'écran (même
  par-dessus la Carte stellaire), bordée de la couleur de sa rareté, traversée
  par un reflet. Une relique épique s'entoure d'étincelles ; une légendaire a
  une bordure arc-en-ciel qui tourne et des feux d'artifice. Plusieurs
  reliques s'affichent l'une après l'autre.

### Rythme (simulateur, 3 h de jeu par jour)
- Collection complète vers le **41e jour** (les raretés supérieures restent
  à chasser ensuite).

## v0.10.0 — Acte III, partie 1 : la Carte stellaire

### Nouveautés
- **Carte stellaire** (touche `X`, à l'éveil du Grand Attracteur) : une carte
  de 24 systèmes, **générée pour chaque partie**, reliés par des routes de
  plus en plus lointaines. Un brouillard cache ce qui n'est pas encore à
  portée.
- **Expéditions** : envoie des flottes explorer les systèmes voisins, en
  **temps réel** (de 2 h à une dizaine d'heures, même jeu fermé). Le danger
  (de ◆ à ◆◆◆◆◆) rallonge le voyage et baisse la chance de succès, mais
  augmente la récompense. Une expédition ratée revient les cales vides.
- **Récompenses permanentes** selon le type de système :

  | Système | Récompense |
  |---|---|
  | ✦ Nébuleuse | production +10 % (×danger) |
  | ★ Amas d'étoiles | fragments +10 % (×danger) |
  | ● Trou noir | +2 h de plafond hors-ligne (×danger) |
  | ◎ Pulsar | comètes plus fréquentes |
  | ▲ Civilisation | une flotte de plus |
  | ◈ Anomalie | des heures de production d'un coup |

- 2 flottes au départ, 4 avec les deux Civilisations.
- **3 nouveaux succès** (45 au total).
- **Succès triés par difficulté** dans l'écran Infos, du plus facile au plus
  difficile (ordre mesuré avec le simulateur).

### Animations
- Carte qui s'ouvre en fondu sur un ciel étoilé, **brume** qui ondule au-delà
  des systèmes à portée, **éclats dorés qui circulent** sur les routes
  explorées, réticule qui palpite autour du système visé.
- **Flottes** visibles en route, avec leur traînée, du système de départ à
  leur cible.
- Exploration réussie : **onde lumineuse** aux couleurs du système, feux
  d'artifice et notification.

### Corrections
- La CI de la v0.9.1 échouait (règle d'architecture) : l'export et l'import
  de sauvegarde passent dans la couche infrastructure.

### Rythme (simulateur, 3 h de jeu par jour)
- Fin de l'Acte II vers le 26e jour, **Carte stellaire complète vers le
  35e jour**.

## v0.9.1 — Options, simulateur et macOS

### Nouveautés
- **Options** (touche `R`) : images par seconde (30, 20 ou 15, pour les
  terminaux lents), effets réduits, emplacement de la sauvegarde, et
  **Recommencer une nouvelle partie**. L'effacement demande une double
  confirmation, puis l'écran se consume de haut en bas, avec un front de
  braises, avant de revenir à l'écran titre d'une partie neuve. Les réglages
  sont gardés.
- **Exporter / importer sa sauvegarde** : `fonderie-stellaire --export fichier`
  puis `--import fichier` (sur un autre ordinateur, par exemple). La
  sauvegarde remplacée est mise de côté en `.bak`.
- **Binaires macOS** (Intel et Apple Silicon), installables avec le script
  `install.sh` et mis à jour automatiquement comme les autres.
- **Simulateur d'équilibrage** : `fonderie-stellaire --simulate` rejoue une
  partie entière avec le vrai moteur du jeu et affiche quand chaque étape est
  atteinte ; `--simulate 3` simule un joueur qui joue 3 h par jour.

### Changements
- Les **mises à jour automatiques ne peuvent plus être désactivées**
  (`--no-update` et `FONDERIE_NO_UPDATE` retirés). Dans l'image Docker, le jeu
  signale la nouvelle version et on met l'image à jour avec `docker pull`.

### Équilibrage (mesuré avec le simulateur)
- Les **Clés d'univers** tombaient toutes en une journée : leurs objectifs
  passent à 25 Supernovas (Gravité forte, Nuée, Désert), 23 (Temps dilaté) et
  28 (Entropie). Elles s'étalent maintenant sur une bonne semaine.
- Rythme mesuré pour un joueur qui joue **3 h par jour** : 1re Supernova en
  1 h, 10e Big Crunch au 5e jour, les 5 Clés entre le 5e et le 14e jour,
  1er Big Bang au 14e jour, **fin de l'Acte II vers le 26e jour**.

## v0.9.0 — Acte II, fin : le Big Bang et le Grand Attracteur

> **Remise à zéro des sauvegardes.** L'équilibrage a trop changé pour garder
> les parties en cours : au premier lancement, un message prévient le joueur
> et l'ancienne partie est effacée (les réglages, comme les effets réduits,
> sont conservés).

### Nouveautés
- **Cosmogonie** (touche `G`, dès le 10e Big Crunch) : un écran qui réunit le
  Big Bang, la forge primordiale et le Grand Attracteur.
- **Big Bang** (30 Big Crunch) : 3e niveau de prestige. Les Big Crunch et tout
  le niveau Supernova repartent de zéro contre des **particules
  primordiales ✷** (une par tranche de 10 Big Crunch). Chaque particule donne
  **+100 % de production** et **+25 % de fragments**, pour toujours, même une
  fois dépensée. Le Laboratoire, les Clés d'univers, les forges et le Grand
  Attracteur sont conservés.
- **Forge primordiale** : 3 générateurs de fin de jeu à forger avec des
  particules (1, 2 puis 3) : Forge à neutrons, Tisseur de réalités, Conscience
  cosmique. Une fois forgés, ils sont présents dans tous les cycles.
- **Grand Attracteur** : 7 étapes payées en particules (23 au total) et
  construites **en temps réel**, même jeu fermé (108 h au total). Elles
  multiplient la production, les fragments, le hors-ligne (100 %) et la
  matière noire. La dernière étape, **l'Éveil**, clôt l'Acte II.
- **3 nouveaux succès** (42 au total).

### Équilibrage
- **Bonus des Supernovas revu à la baisse** : toujours +10 % par fragment
  jusqu'à 5, puis une croissance en racine cubique au lieu de la racine
  carrée. Exemples : 40 fragments donnent +100 % (au lieu de +141 %),
  625 fragments +250 % (au lieu de +559 %), 120 000 fragments +1 450 %
  (au lieu de +7 900 %).

### Animations
- **Big Bang** : l'écran entier est aspiré en spirale vers un point, un
  silence où ce point palpite de plus en plus vite, puis un éclair blanc. Des
  ondes de choc partent ensuite aux couleurs du spectre, des débris filent dans
  toutes les directions, et une nébuleuse et des étoiles naissent. Enfin,
  « BIG BANG » se décrypte en arc-en-ciel avant l'annonce des particules.
- **Fin de l'Acte II** : les étoiles convergent en spirale, un éclair doré
  annonce une galaxie immense qui s'ouvre et tourne, puis le générique défile
  avec les chiffres de ta partie.
- **Le Grand Attracteur dans le ciel** : une galaxie à deux bras autour d'un
  cœur doré, qui grandit à chaque étape. L'étape en chantier se dessine en
  pointillés qui se remplissent, et une onde lumineuse salue chaque étape
  achevée.
- **Forge** : gerbe d'étincelles aux couleurs du générateur forgé.

### Lisibilité
- **Plus aucun texte coupé** : les descriptions, détails, notifications, le
  journal et toutes les fenêtres passent à la ligne au lieu d'être tronqués.
  Les fenêtres s'adaptent à leur contenu et gardent une taille fixe quand on
  change de sélection. Un test parcourt chaque écran à 4 tailles de terminal
  (de 80×24 à 160×48) et échoue au moindre texte coupé.
- Le panneau des améliorations **s'agrandit en douceur** quand il a le focus,
  pour afficher le détail complet.
- L'en-tête passe à une version plus compacte quand la place manque, au lieu
  de couper.
- Journal : les heures au-delà d'une heure de jeu s'affichent `111h06`.

### Confort
- **Redémarrage automatique après une mise à jour** : un bandeau compte à
  rebours (5 s), le jeu attend la fin d'une Supernova ou d'un Big Bang en
  cours, sauvegarde, puis se relance tout seul sur la nouvelle version.
- **Moins de saccades** : chaque image part en une seule écriture vers le
  terminal (au lieu de plusieurs), avec la mise à jour synchronisée quand le
  terminal la gère. Les mesures montrent moins de 2 ms par image : les
  ralentissements restants viennent du terminal, pas du jeu (`F3` affiche les
  temps d'image).

## v0.8.0 — Acte II, partie 2 : les Univers parallèles

### Nouveautés
- **Univers parallèles** (à partir du 10e Big Crunch) : chaque Big Crunch
  ouvre l'**écran des portails**, où l'on choisit les règles du cycle suivant.

  | Univers | Règles | Clé (objectif) | Bonus permanent de la Clé |
  |---|---|---|---|
  | Gravité forte | générateurs ×2 plus chers, production ×4 | 20 Supernovas | générateurs −25 % |
  | Temps dilaté | production ×0,5 en jeu, hors-ligne ×4 | 18 Supernovas | hors-ligne ×2, +24 h |
  | Nuée | comètes ×5 plus fréquentes, tempêtes 2× plus courtes | 20 Supernovas | pluie d'étoiles +50 %, comètes rares ×1,5 |
  | Désert | aucune amélioration, production ×10 | 20 Supernovas | production ×2 |
  | Entropie | production −1 %/min dans chaque cycle, fragments ×3 | 22 Supernovas | fragments ×1,5 |

- `V` pour consulter les univers et ses Clés à tout moment.
- **2 nouveaux succès** (39 au total) ; les Clés comptent dans la progression.

### Animations
- **Écran des portails** : carrousel de cartes avec un **vortex animé** par
  univers (bras en spirale, densité `· : ░ ▒ ▓ █`, couleurs propres). Les
  cartes arrivent en vol, glissent avec rebond, la carte choisie brille et
  tourne plus vite ; à la validation, le portail **s'ouvre jusqu'à engloutir
  l'écran** avant l'implosion du Big Crunch.
- **Big Crunch aux couleurs de la destination** : onde d'implosion et flash
  teintés, puis le vortex de l'univers s'ouvre derrière « BIG CRUNCH » et
  « Bienvenue dans l'univers : … ».
- **Cérémonie de la Clé** : deux anneaux en sens inverse s'ouvrent avec
  rebond autour d'une clé dessinée en blocs, traversée par un reflet ; nom et
  bonus écrits en machine à écrire, gerbe d'étincelles et feux d'artifice.
- **Le ciel reflète l'univers** : nébuleuse teintée, ciel **ralenti** dans le
  Temps dilaté et **accéléré** (soleil plus gros) en Gravité forte, étoiles qui
  **s'éteignent** et soleil qui se ternit avec l'Entropie, pluie d'étoiles
  filantes dans la Nuée. Chaque Clé ajoute un **anneau en orbite** autour du
  soleil, parcouru par une perle brillante.
- Badge de l'univers en cours sur la bordure de l'en-tête.

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
