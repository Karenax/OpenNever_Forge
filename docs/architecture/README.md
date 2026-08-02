# Graphe d'architecture

Le graphe est un index local et déterministe du code d'OpenNever Forge. Le code reste la source de
vérité. Le graphe ne contient aucune ressource NWN ni donnée utilisateur.

## Commandes

```powershell
python scripts/architecture_graph.py generate
python scripts/architecture_graph.py check
python scripts/architecture_graph.py stats
python scripts/architecture_graph.py query "start_module_hash"
python scripts/architecture_graph.py query "SQLite" --format paths
```

`graph.json` et `overview.mmd` sont générés et ne doivent jamais être modifiés à la main.

## Couverture initiale

- fichiers Python, Rust, SQL, TypeScript et TSX dans les répertoires source déclarés ;
- imports TypeScript relatifs ;
- modules et imports de crates Rust internes ;
- fonctions, types et composants exportés simples ;
- commandes Tauri et appels `invoke` correspondants ;
- tests Rust annotés et fichiers frontend `*.test.*` ;
- preuves fichier/ligne pour toutes les relations.

## Relations

- `imports` : dépendance statique explicite entre fichiers ou vers une crate ;
- `defined_in` : symbole déclaré dans un fichier ;
- `invokes_command` : appel frontend explicite d'une commande Tauri ;
- `tests` : test Rust explicite rattaché au fichier qui le contient.

## Limites connues

Le générateur utilise des extracteurs lexicaux prudents de la bibliothèque standard Python. Il ne
résout pas encore les réexports TypeScript complexes, les macros Rust arbitraires, les appels de
fonctions inter-crates ou la couverture fonctionnelle implicite. Une syntaxe non reconnue est
ignorée plutôt que transformée en relation supposée.
