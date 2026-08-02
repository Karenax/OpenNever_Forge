# ADR 0002 — Bibliothèques NWN et politique de licence

- Statut : accepté
- Date : 2 août 2026

## Contexte

Le projet vise une licence permissive et doit préserver la possibilité de distribuer un binaire
desktop simple. `nwn-lib-rs` couvre plusieurs formats utiles, mais sa version 0.4.0 est publiée sous
LGPL-3.0-or-later et active actuellement des fonctionnalités Rust nightly.

## Décision

Le cœur utilise Rust stable et des interfaces internes pour les lecteurs. `nwn-lib-rs` n'est pas
lié au binaire principal et aucun de ses fichiers source n'est copié. Il peut servir d'oracle externe
dans des tests locaux opt-in, au même titre que Nasher, neverwinter.nim ou nwn.py.

Chaque dépendance de parsing future doit faire l'objet d'un examen de licence, maintenance,
couverture, comportement sur fichiers invalides et compatibilité avec Rust stable.

## Conséquences

- le Lot 1 peut nécessiter un lecteur ERF/GFF minimal interne ou un adaptateur sidecar ;
- la licence Apache-2.0 du projet reste claire ;
- les résultats des oracles sont comparés, jamais considérés comme une vérité unique ;
- toute évolution vers un lien LGPL ou du code GPL nécessite un nouvel ADR et une décision explicite.
