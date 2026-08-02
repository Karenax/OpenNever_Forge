# Contribuer

Lire `AGENTS.md` et `CONTEXT.md` avant toute modification importante.

## Règles essentielles

- ne jamais ajouter de module, HAK, TLK, modèle, texture, son ou script NWN propriétaire ;
- ne jamais écrire dans un module source pendant la Phase 1 ;
- garder les lecteurs de formats, le Resource Manager, le modèle métier, SQLite et l'UI séparés ;
- ajouter des fixtures synthétiques et des tests pour tout parser ou comportement critique ;
- documenter les limites et les erreurs au lieu de les masquer ;
- interroger puis régénérer le graphe d'architecture pour les modifications indexées.

## Vérifications avant contribution

```powershell
pnpm lint
pnpm test:run
pnpm build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python scripts/architecture_graph.py check
```
