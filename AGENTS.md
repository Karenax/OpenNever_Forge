# Consignes du dépôt

Lire entièrement `CONTEXT.md` avant toute modification importante. Le projet reste strictement en
lecture seule vis-à-vis des fichiers NWN pendant toute la Phase 1.

## Graphe d'architecture

Le graphe déterministe décrit les relations entre l'interface React, les commandes Tauri, les
services Rust, le modèle métier, le Resource Manager, les lecteurs de formats, SQLite, le cache et
les tests. Le code reste la source de vérité.

Le générateur du Lot 0 est présent. `graph.json` et `overview.mmd` restent des artefacts générés :

- avant une modification transversale, interroger le sous-graphe concerné avec
  `python scripts/architecture_graph.py query "<module ou symbole>"` ;
- utiliser `--format paths` pour obtenir uniquement les fichiers à examiner et n'augmenter
  `--depth` ou `--max-nodes` que si le résultat initial est insuffisant ;
- ne jamais charger intégralement `docs/architecture/graph.json` dans le contexte d'un agent ;
- après toute modification d'un fichier source indexé ou des règles d'architecture, exécuter
  `python scripts/architecture_graph.py generate` ;
- avant de terminer une intervention, exécuter
  `python scripts/architecture_graph.py check` ;
- ne jamais corriger manuellement `docs/architecture/graph.json` ou
  `docs/architecture/overview.mmd`, car ces fichiers sont générés.

Le mode d'emploi et la signification des relations sont documentés dans
`docs/architecture/README.md`.
