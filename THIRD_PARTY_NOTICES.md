# Third-party notices

Inventaire initial des dépendances structurantes. Les versions exactes distribuées sont verrouillées
par `pnpm-lock.yaml` et `Cargo.lock`.

| Composant | Rôle | Licence | Politique |
|---|---|---|---|
| Tauri 2 | Conteneur desktop et IPC | Apache-2.0 OR MIT | Inclus |
| React 19 | Interface | MIT | Inclus |
| Vite | Build frontend | MIT | Développement |
| TypeScript | Typage frontend | Apache-2.0 | Développement |
| TanStack Query | État asynchrone | MIT | Inclus |
| Zustand | État local UI | MIT | Inclus |
| Monaco Editor / `@monaco-editor/react` | Source NWScript en lecture seule | MIT | Inclus localement, sans CDN |
| React Flow / `@xyflow/react` | Graphe complet des dialogues | MIT | Inclus localement |
| Babylon.js / `@babylonjs/core` / `@babylonjs/loaders` | Vues 3D, GLB et textures | Apache-2.0 | Inclus localement, chargé à la demande |
| `png` | Encodage local des aperçus PLT | MIT OR Apache-2.0 | Inclus côté Rust |
| `reqwest` + rustls | Appels HTTPS explicites vers le fournisseur IA choisi | MIT OR Apache-2.0 | Inclus côté Rust, réseau désactivé par défaut |
| Khronos `gltf-validator` | Validation GLB de développement | Apache-2.0 | Développement uniquement |
| xoreos-docs `NWN1MDL.bt` | Description du format MDL NWN1 | CC0-1.0 | Référence de spécification, aucun code embarqué |
| rusqlite / libsqlite3-sys | Index SQLite côté Rust | MIT | Inclus |
| SQLite embarqué | Base locale | Domaine public | Inclus via `rusqlite/bundled` |
| nwn-lib-rs | Oracle ERF/GFF/TLK/2DA éventuel | LGPL-3.0-or-later | Non lié, non copié |
| nwneetools `nwnsc` | Compilation externe NSS → NCS | MIT/BSD permissive | Exécutable choisi par l'utilisateur, non distribué |

Babylon.js est isolé derrière le manifeste de scène Rust : il ne lit pas directement les ressources
NWN et n'introduit aucune seconde logique de résolution. Le lecteur `aurora-mdl` a été écrit
indépendamment à partir de la description CC0 publiée dans xoreos-docs ; aucun code GPL de xoreos,
`nwnrs-types` ou `nwn-lib-d` n'est lié, copié ou distribué.
