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

## Validation appliquée au Lot 1

`neverwinter.nim` 2.1.2 est le premier oracle effectivement exécuté. Ses outils MIT `nwn_erf`,
`nwn_gff` et `nwn_tlk` sont utilisés depuis un dossier local ignoré, uniquement par un script opt-in.
Ils ne sont pas une dépendance du workspace, ne sont pas téléchargés par la CI et aucun code source
n'est copié. Le rapport du 3 août 2026 ne relève aucune divergence sur la fixture synthétique du
Lot 1.

## Audit appliqué au Lot 8

L'audit du 3 août 2026 confirme que `nwnrs-types` 0.0.1 annonce une couverture MDL binaire et
ASCII utile, mais est publié sous GPL-3.0-only. Il n'est donc ni lié, ni copié, ni distribué avec
OpenNever Forge. `nwn-lib-d` est écarté pour la même raison.

Le template `NWN1MDL.bt` de `xoreos-docs` est publié sous CC0-1.0 et a servi de description de format
au lecteur interne indépendant `aurora-mdl`. L'implémentation, le cache GLB et les tests ont été
écrits dans ce dépôt sous Apache-2.0, sans copier ni lier de code GPL. Le Lot 8 peut ainsi fermer sa
porte tout en conservant une distribution permissive ; voir l'ADR 0006.
