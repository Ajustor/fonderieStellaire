# Navigation desktop — plan

**But :** rendre les écrans de la version terminal faciles à trouver et à
parcourir. Desktop seulement : le mode mobile a déjà ses onglets.

**Décisions (validées) :**

- **Barre d'onglets** dans la bordure basse de l'en-tête : Fonderie │ Prestige
  (Atelier, Défis, Labo, Univers) │ Cosmos (Cosmogonie, Carte, Reliques, Méga,
  Défi du jour) │ Plus (Infos, Options, Classement). Seulement les écrans
  débloqués ; l'écran ouvert en doré ; sa lettre à côté. Trop étroit : noms
  courts, puis l'écran courant seul avec « Tab › ».
- **`Tab` / `Maj+Tab`** : écran suivant / précédent de la barre (la Fonderie
  ferme l'écran ouvert). `←` / `→` gardent le changement de panneau.
- **Menu central** sur `Échap` dans la fonderie : écrans débloqués groupés,
  lettre et une ligne d'explication, puis Aide et Quitter. `↑↓` + `Entrée` ou
  la lettre. `Échap` ferme toujours l'écran ouvert ; `Q` quitte.
- **Panneau d'aide** (`?` ou `F1`, et dans le menu) : toutes les touches
  utiles, groupées ; rien de ce qui n'est pas encore découvert.
- **Barre d'aide contextuelle** : écran ouvert → `Tab` écran suivant,
  `Échap` fermer, `?` aide.
- **Lettres renommées** : Défi du jour `Y` → `J`, Mégastructures `W` → `B`,
  Reliques `H` → `R`, Options `R` → `O` (« Oui » dans les confirmations),
  Big Crunch `U` → `K`. `j`/`k` ne sont plus des flèches, `b` n'achète plus.
  `F3` reste le debug.

**Code :**

1. `input.rs` : `Intent::{NextScreen, PrevScreen, Help}` ; `Tab`/`BackTab`,
   `?`/`F1` ; lettres renommées.
2. `nav.rs` (nouveau) : `ScreenId` (nom, nom court, lettre, groupe, aide,
   `unlocked(snap)`, `intent()`), `available(snap)`, `App::current_screen`,
   `App::switch_screen` (ferme l'écran ouvert, ouvre le suivant).
3. `app.rs` : `NextScreen`/`PrevScreen` avant le routage des popups ;
   `Escape` dans la fonderie → menu ; `Yes` dans la fonderie → Options.
4. `widgets/nav.rs` : barre d'onglets, menu, panneau d'aide.
5. `help_bar.rs` : variante « écran ouvert ».
6. Textes : toutes les mentions `[U]`, `[H]`, `[W]`, `[Y]`, `[R]`… ; README
   (table des touches), CHANGELOG.
7. Tests : routage clavier, Tab dans l'ordre de la barre, menu (lettre et
   Entrée), aide sans écran verrouillé, rendu aux 4 tailles (`readability`).
