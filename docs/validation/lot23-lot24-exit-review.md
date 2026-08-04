# Revue de sortie — Lots 23 et 24

Date : 4 août 2026

## Lot 23 — synchronisation temporaire Aurora

- inventaire borné des ressources reconnues, sans suivi des liens symboliques ni des sauvegardes ;
- comparaison à trois états avec baseline persistée par workspace Toolset canonique ;
- conflits visibles et direction explicite par ressource ;
- préconditions SHA-256 revérifiées à l’application ;
- imports enregistrés dans le journal transactionnel OpenNever et compatibles undo/redo ;
- remplacements et suppressions Toolset précédés d’une sauvegarde récupérable ;
- garde NSS/NCS avant envoi et rappel explicite de compiler puis sauvegarder dans Aurora ;
- interface React et commandes Tauri typées pour comparer et appliquer.

## Lot 24 — documentation, migrations et refactoring

- schéma workspace v3 avec migration des versions 1 et 2 ;
- sauvegarde exacte avant migration, historique visible et rejet des versions futures ;
- moteur de comparaison isolé dans `crates/aurora-edit/src/sync.rs` ;
- guide utilisateur, guide de migration et ADR de sécurité ;
- analyse de santé et dette résiduelle explicite dans `lot24-code-health-review.md` ;
- plan, contexte, README, changelog et graphe d’architecture mis à jour.

## Limites externes

La persistance finale des fichiers synchronisés dépend toujours du bouton de sauvegarde d’Aurora.
Le 4 août 2026, cette contrainte a été vérifiée lors d'un cycle réel comparer → synchroniser →
compiler → sauvegarder → rouvrir ; Aurora a recréé le workspace avec le NSS modifié et son NCS
compilé. La preuve est détaillée dans `release-closure-2026-08-04.md`.

La preuve moteur positive `nwserver` demeure à rejouer sur un environnement où le module témoin ne
plante pas avant l’écoute. Cette limite ne retire pas la reproductibilité des writers ni les
garanties de non-modification du MOD source.

## Contrôles de clôture

- `cargo test --workspace` : 110 tests réussis ;
- Vitest : 16 tests d’interface réussis ;
- Pytest : 12 tests d’outillage réussis ;
- TypeScript et build Vite : réussis, avertissement non bloquant sur la taille des chunks ;
- formatage Rust, `git diff --check` et fraîcheur du graphe : réussis ;
- graphe déterministe : 878 nœuds et 978 relations.
