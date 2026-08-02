# ADR 0001 — Socle technique de l'application

- Statut : accepté
- Date : 2 août 2026

## Contexte

OpenNever Forge doit lire de grands conteneurs binaires non fiables, indexer leurs métadonnées,
maintenir une interface d'éditeur fluide et préparer un futur rendu 3D, tout en ciblant d'abord
Windows 10/11.

## Options étudiées

1. Tauri 2, React, TypeScript et cœur Rust.
2. Electron avec cœur Node natif.
3. Application Rust native complète.

## Décision

Conserver Tauri 2 + React 19 + TypeScript strict + Rust stable MSVC. SQLite est accessible
exclusivement depuis Rust via `rusqlite` avec SQLite embarqué. Zustand porte l'état local de l'UI et
TanStack Query les appels asynchrones Tauri.

Le workspace commence avec trois crates cohésives :

- `aurora-core` : erreurs, diagnostics et types transversaux ;
- `aurora-project` : projet en lecture seule, validation et hash ;
- `aurora-index` : SQLite et migrations.

Les crates de formats ne seront créées qu'à l'arrivée de leur premier comportement réel. Babylon,
Monaco et React Flow sont différés jusqu'aux lots 8, 4 et 5 respectivement.

## Conséquences

- les parsers et jobs bénéficient de la sûreté mémoire et du multithreading Rust ;
- le frontend reste testable dans un navigateur sans dupliquer les règles métier ;
- l'IPC reste composé de DTO légers ;
- le build Windows nécessite Rust MSVC, les C++ Build Tools et WebView2 ;
- la fragmentation prématurée du workspace est évitée.
