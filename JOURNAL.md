# Journal de développement — zeedefi-launcher

Tout ce qui a été fait, vu et décidé pendant les sessions de travail.
Dernière mise à jour : 16 novembre 2026.

---

## 1. État des lieux (16 nov. 2026)

| Composant | État | Détail |
|---|---|---|
| Launcher (interface Rust/egui) | **Terminé, commité, poussé** | Dernier commit : `c8d89c2` |
| API / démon ZeedeFi | **À écrire (0 ligne)** | Contrat proposé : voir §6 |
| Lutris | **Pas encore installé** | Sera le lanceur des jeux côté machine |
| Garde-batterie | **En pause volontaire** | Voir §5 |

### Arborescence locale
- Dossier de travail : `~/Documents/Projet par défaut/zeedefi-launhcer`
  (la coquille « launhcer » dans le nom est assumée, elle vient du dossier d'origine).
- Ancien double : `~/Documents/Projet par défaut/zeedefi-launcherrrrrrrrrrrrrrrrr`
  (conservé, plus rien à en tirer).
- Dépôt distant : `https://github.com/ZeedeFi/zeedefi-launcher.git`, branche `main`.
- L'historique `.git` a été copié de l'ancien dossier vers le nouveau :
  toute la refactorisation est donc un commit normal, rien n'est perdu.

### Commits marquants
- `681da7f` — Refonte ergonomie : sélecteur unique par lignes, animations
  d'entrée, reflet et défilement analogique.
- `c8d89c2` — Peaufinage : sélecteur allégé, paramètres en cartes sombres,
  fiche détail agrandie.

---

## 2. Principes de l'interface

- **Aucune souris** : pilotage exclusif clavier/manette. Le curseur système est
  masqué (`ViewportCommand::CursorVisible(false)`), aucun code pointer/click.
- **Aucun `unwrap()` / `expect()` en production** : erreurs explicites +
  `tracing`. Les seuls `expect()` restants sont dans les tests
  (`zeedefi_client.rs`, module `#[cfg(test)]`).
- Chaque modification doit passer : `cargo clippy -- -D warnings` puis
  `cargo build --release`.
- Lancement des jeux **délégué au démon** : le launcher n'exécute jamais un
  processus localement (`launch_game` envoie un identifiant à l'API).

## 3. Navigation (implémentée et testée)

- **Haut / Bas** : change de rangée — profil → actions (Jouer / Détails / ⚙) → tuiles.
- **Gauche / Droite** : parcourt la rangée active.
- **A (Entrée)** : valide ; **B (Échap)** : retour.
- **X** : fiche détail (modale : A/B/Échap referment) ;
  **Y / S** : paramètres ; **Tab / P / Start** : menu de profil.
- **PageUp / PageDown** : saut de pages ; **stick droit** : défilement rapide
  (plus incliné = plus rapide) ; **gâchettes** : pages.
- Un **sélecteur unique** (anneau accent) suit toujours la rangée en cours ;
  il est masqué pendant l'ouverture d'une modale.

## 4. Polissage UI réalisé (session du 16 nov.)

- **Sélecteur unique simplifié** (`src/app.rs`, `draw_focus_ring`) : un liseré
  net 2,5 px + deux halos doux (au lieu de 4 contours). Rayon aligné sur la
  cible : coin de tuile (≥120 px de haut), pilule (≤44 px), rayon de carte
  (16 px) sinon.
- **Paramètres** (`src/components/settings_panel.rs`) :
  - lignes en **cartes opaques** `#151A24`, sélection `#1D2533`, bord 1 px,
    barre d'accent 3×36 px sur la ligne active ;
  - sous-titres éclaircis (`text_muted` → mélange 55 % vers `text_secondary`) ;
  - défilement : marges `FOCUS_MARGIN` (60 px) et `LIST_BOTTOM_PADDING` (60 px)
    pour que l'anneau et la ligne active ne rentrent **jamais** dans les voiles ;
  - voiles haut (40 px) et bas (44 px) entièrement opaques au bord, fondus dans
    la couleur du fond — plus de coupure nette ;
  - à l'ouverture : retour systématique à la première rubrique
    (`selected = 0`, `scroll = 0`) ;
  - pendant les fondus d'ouverture/fermeture, l'accueil reste peint
    (`is_fading()`) pour éviter tout saut visuel ;
  - rappel de touches en pied : `A Choisir` / `B Retour`.
- **Fiche détail** (`src/app.rs`, `draw_detail_overlay`) : agrandie
  640×330 → **760×400**, vignette 140 → **180 px**, titre 30 px, valeurs
  14,5 px, interligne 56 px.
- **Pied de page accueil** (`src/components/carousel.rs`) : ajout de
  `B Retour`, capsules et libellés plus contrastés.
- **README** : navigation par rangées documentée.
- **`zeedefi_client.rs`** : dernier `unwrap()` de production supprimé
  (calcul du délai d'attente via `map`).

### Vérifications faites
- `cargo clippy -- -D warnings` : OK après chaque étape.
- `cargo build --release` : OK (dernière : 4 min 10 s).
- Captures d'écran de contrôle (`~/.cache/zeedefi-launcher/`) : accueil,
  rangée actions, rangée profil, menu profil, paramètres (haut, moitié, fin),
  fiche détail. Tout est conforme.
- Tests de navigation clavier via `xdotool` : montée/descente de rangées,
  ouverture menu → paramètres, défilement jusqu'à « Quitter le launcher »
  (double-A de confirmation vérifié, sans effet destructeur hors ligne),
  ouverture/fermeture de la fiche détail.

---

## 5. Garde-batterie

- **Script** : `scripts/battery-guard.sh` — à 8 % de batterie : commit+poussée
  de sauvegarde puis extinction de la machine.
- Chemins mis à jour vers le nouveau dossier (script + unité systemd).
- Copie installée : `~/.local/bin/zeedefi-battery-guard` ;
  unité : `~/.config/systemd/user/zeedefi-battery-guard.service`.
- **État actuel : EN PAUSE** sur demande explicite de l'utilisateur
  (« n'éteins pas le pc ») :
  - service `inactive` (`systemctl --user is-active zeedefi-battery-guard.service`),
  - marqueur `~/.cache/zeedefi/battery-guard.done` présent → le script ne
    réagira pas même s'il est relancé.
- Réactivation (quand l'utilisateur le demandera) :
  `rm ~/.cache/zeedefi/battery-guard.done && systemctl --user start zeedefi-battery-guard.service`
- Preuve du dernier déclenchement : `~/.cache/zeedefi/battery-guard.log`
  (sauvegarde + extinction à 8 % le 15 nov. 22:56).

---

## 6. Prochaine étape : Lutris + API (à écrire)

L'utilisateur va installer **Lutris**, qui lancera les jeux. Des raccourcis
seront créés, et **c'est l'API (démon) qui interrogera Lutris** pour fournir :
le titre, et les chemins vers la bannière, le fond, le logo et la tuile.

### Contrat existant (déjà consommé par le launcher)
- Transport : socket Unix `/run/zeedefi/zeedefi.sock`, une ligne JSON/message.
- Requêtes :
  - `list_games` `{}` → `data` = tableau de jeux ;
  - `launch_game` `{"id": "…"}` → lancement délégué au démon.
- Enveloppe de réponse : `{"success": bool, "code": u32, "id": u64?, "data": …}`.
- Jeu (struct `DaemonGame`, `src/zeedefi_client.rs:80`) :
  `{"id", "title"?, "cover_path"?, "background_path"?, "store_icon"?}`.
  Le launcher n'exploite aujourd'hui que tuile (`cover_path`) + fond
  (`background_path`) ; `store_icon` est reçu mais inutilisé.

### Contrat proposé pour les artworks Lutris (à confirmer)
```json
{
  "id": "lutris:11834",
  "title": "Starfield",
  "tile_path":       "~/.cache/zeedefi/assets/starfield/tile.png",
  "banner_path":     "~/.cache/zeedefi/assets/starfield/banner.jpg",
  "background_path": "~/.cache/zeedefi/assets/starfield/background.jpg",
  "logo_path":       "~/.cache/zeedefi/assets/starfield/logo.png"
}
```
Règles de repli (dégradé = aucune image) :
- `logo_path` → remplace le titre texte du héros, sinon texte ;
- `banner_path` → bandeau large de la fiche détail, sinon vignette carrée ;
- `tile_path` → tuile du carrousel, repli `cover_path`, sinon dégradé+initiales ;
- `background_path` → fond, repli bannière floutée, sinon aplat.

**Sécurité** : le launcher ne charge qu'un chemin **absolu** situé sous une
racine autorisée (`/usr/share/zeedefi`, `/var/lib/zeedefi`, `/var/cache/zeedefi`,
`~/.local/share/zeedefi`, `~/.cache/zeedefi`, `~/.cache/zeedefi-launcher`, ou
`ZEEDEFI_ASSET_ROOTS`). Hors racines → chemin **ignoré silencieusement**.
Si Lutris garde ses images ailleurs (`~/.cache/lutris`, `~/.steam/...`),
l'API doit les copier sous `~/.cache/zeedefi/assets/<jeu>/`, ou on ajoute les
dossiers Lutris à `ZEEDEFI_ASSET_ROOTS`.

### Travail launcher restant (indépendant de l'API)
1. `DaemonGame` / `GameEntry` : ajouter `banner_path`, `logo_path`, `tile_path`
   (+ alias tolérants : `cover_path`, `banner`, `logo`, `tile`).
2. `cache.rs` : `AssetKind::Banner` / `AssetKind::Logo`, **transparence
   préservée** pour le logo (PNG alpha).
3. Héros (`hero_tile.rs`) : dessiner le logo à la place du titre s'il existe.
4. Fiche détail : bannière large en en-tête si présente.
5. Préchargement : logo/bannière du jeu focus + voisins.

### Questions ouvertes (bloquent l'API, pas le launcher)
1. Nommage JSON : `tile_path`/`banner_path`/`background_path`/`logo_path`, ou
   un objet `assets: {tile, banner, background, logo}` ?
2. Les images sont-elles copiées par l'API vers `~/.cache/zeedefi/assets/`,
   ou ajoute-t-on les dossiers Lutris à `ZEEDEFI_ASSET_ROOTS` ?
3. Format de `id` : id Lutris brut (`"11834"`) ou préfixé (`"lutris:11834"`) ?

### Décision de design reportée
Le déséquilibre visuel de l'accueil (deux tiers supérieurs vides) vient de
l'absence d'artworks, pas du layout. À réexaminer **une fois les vraies
images branchées** ; améliorer d'abord la typo, puis éventuellement un bloc
d'infos à droite.

---

## 7. Mémo technique (tests visuels)

```bash
# Lancer l'app (déjà compilée en release)
cd "/home/zdfi/Documents/Projet par défaut/zeedefi-launhcer" && setsid nohup \
  env WAYLAND_DISPLAY= DISPLAY=:0 ./target/release/zeedefi-launcher \
  > "$HOME/.cache/zeedefi/launcher.log" 2>&1 < /dev/null &

# Donner le focus (sinon les touches partent dans OpenCode !)
DISPLAY=:0 wmctrl -l                       # retrouver l'id : 0x00e00002
DISPLAY=:0 wmctrl -i -a 0x00e00002 && sleep 1
DISPLAY=:0 xdotool getactivewindow getwindowname   # vérifier AVANT d'envoyer des touches

# Touches (avec --clearmodifiers), puis capture plein écran
DISPLAY=:0 xdotool key --clearmodifiers Up
cd ~/.cache/zeedefi-launcher && spectacle -f -b -n -o capture.png
```

- Tuer l'UI : `pkill -x zeedefi-launche` (nom tronqué à 15 caractères ;
  **jamais** `pkill -f`, qui tuerait le build cargo en cours).
- `spectacle -f` (plein écran) est plus fiable que `-a` (fenêtre active) :
  le focus revient parfois sur OpenCode juste après une capture.
- Si la capture est noire : `xdotool mousemove 500 500` pour réveiller l'écran.
- Vérifier `xdotool getactivewindow` **avant chaque touche** : une touche perdue
  dans OpenCode défile la conversation au lieu de naviguer dans le launcher.
