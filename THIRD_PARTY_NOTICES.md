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
| rusqlite / libsqlite3-sys | Index SQLite côté Rust | MIT | Inclus |
| SQLite embarqué | Base locale | Domaine public | Inclus via `rusqlite/bundled` |
| nwn-lib-rs | Oracle ERF/GFF/TLK/2DA éventuel | LGPL-3.0-or-later | Non lié, non copié |

Babylon.js, Monaco Editor et React Flow ont des licences compatibles, mais ne seront ajoutés qu'aux
lots qui les utilisent réellement.
