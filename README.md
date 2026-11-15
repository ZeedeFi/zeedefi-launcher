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

## Contrôles

Manette : stick gauche / D-pad = navigation, A = valider, B = retour,
X = détails, Y = paramètres, stick droit X = défilement rapide,
gâchettes = page précédente/suivante.

Clavier : flèches = navigation, Entrée/Espace = valider, Échap = retour,
X = détails, Y/S = paramètres, PageUp/PageDown = pages.

## Sécurité

- Aucun processus n'est lancé localement à partir des données du démon : la
  demande de lancement est déléguée au démon via `launch_game`.
- Les chemins d'assets sont validés (pas de `..`, symlinks résolus et
  vérifiés contre les racines autorisées).
- Timeouts de 5 s sur les opérations réseau.