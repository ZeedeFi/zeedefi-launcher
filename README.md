# zeedefi-launcher

Interface « Big Picture » (plein écran, pilotée à la manette) pour le démon
système **ZeedeFi OS**. Rendu natif via `egui`/`wgpu`, aucune dépendance web
ni moteur de jeu.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                       eframe (wgpu)                         │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Background layer : fond d'écran + overlay sombre     │  │
│  │  Middle layer     : header / hero / carrousel         │  │
│  │  Foreground layer : settings panel / toast            │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                             │
│  InputManager (thread gilrs) ──► AppState                   │
│  AssetLoader  (tokio spawn_blocking) ──► TextureCache (LRU) │
│  ZeedeFiClient (tokio task) ──► socket Unix /run/zeedefi/  │
└─────────────────────────────────────────────────────────────┘
```

## Build

```bash
cargo build --release
cargo clippy -- -D warnings
```

## Lancement

```bash
ZEEDEFI_SOCKET=/run/zeedefi/zeedefi.sock ./target/release/zeedefi-launcher
```

Variables d'environnement :

- `ZEEDEFI_SOCKET` : chemin du socket Unix du démon (défaut `/run/zeedefi/zeedefi.sock`).
- `ZEEDEFI_ASSET_ROOTS` : racines autorisées pour les assets (séparateur `:`).
- `RUST_LOG` : niveau de log (défaut `info,zeedefi_launcher=debug`).
- `ZEEDEFI_FPS=1` : affiche le compteur de frames en bas à gauche.
- `ZEEDEFI_NO_VSYNC=1` : désactive la synchronisation verticale (diagnostic de performance).
- `ZEEDEFI_PROFILE` : nom de profil affiché (sinon champ GECOS de `/etc/passwd`).

## Contrôles

Manette : stick gauche / D-pad = navigation, A = valider, B = retour,
X = détails, Y = paramètres, Start = menu de profil, stick droit X =
défilement rapide, gâchettes = page précédente/suivante.

Clavier : flèches = navigation, Entrée/Espace = valider, Échap = retour,
X = détails, Y/S = paramètres, Tab/P = menu de profil, PageUp/PageDown = pages.

Souris : aucune. L'interface est pilotée exclusivement au clavier ou à la
manette ; le pointeur système est masqué en permanence et aucun élément n'est
cliquable. Le focus est unique, toujours visible, et survit aux surcouches.

Navigation : les listes bouclent dans les deux sens, un appui maintenu répète
le déplacement (400 ms puis toutes les 120 ms), et Gauche/Droite bascule
directement les interrupteurs du panneau de paramètres.

## Sécurité

- Aucun processus n'est lancé localement à partir des données du démon : la
  demande de lancement est déléguée au démon via `launch_game`.
- Les chemins d'assets sont validés (pas de `..`, symlinks résolus et
  vérifiés contre les racines autorisées).
- Timeouts de 5 s sur les opérations réseau.