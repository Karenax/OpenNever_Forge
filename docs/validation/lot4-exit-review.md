# Revue de sortie — Lot 4

- Date : 3 août 2026
- Statut : accepté
- Portée : inventaire NSS/NCS, analyse NWScript, recherche et navigation en lecture seule

## Exigences

| Exigence | État | Preuve |
| --- | --- | --- |
| Inventaire NSS/NCS | Réussie | index logique groupant source et bytecode par ResRef |
| Monaco en lecture seule | Réussie | langage NWScript déclaré, thème du produit, `readOnly` et `domReadOnly` |
| Includes, symboles et constantes | Réussie | lexer borné ignorant commentaires et chaînes, lignes et déclarations conservées |
| Appels détectables | Réussie | index des appels de fonctions avec ligne source |
| Recherche plein texte | Réussie | recherche native paginée sur ResRef, source, symboles et ressources entrantes |
| Vue NCS séparée | Réussie | en-tête, taille, SHA-256 et aperçu hexadécimal, sans prétendre décompiler |
| Sources absentes | Réussie | diagnostic stable `NSS_SOURCE_MISSING` et vue NCS toujours disponible |
| Références entrantes | Réussie | parcours des champs GFF de module, zones, dialogues et blueprints avec chemin de preuve |
| Navigation objet ↔ script | Réussie | action depuis l'inspecteur de ressource et liste des références dans la vue script |
| Persistance | Réussie | migration SQLite v4 : scripts, source, symboles, includes et références |
| Fixture redistribuable | Réussie | deux NSS, un NCS et un include résolu dans la fixture CC0 |
| Build release Windows | Réussie | application et installateur NSIS x64 reconstruits |
| Sources NWN immuables | Réussie | lecture à la demande uniquement, aucune compilation ni écriture |

## Validation sur le corpus local

| Module | Scripts | NSS | NCS | NSS absents | Symboles | Références entrantes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Contest of Champions | 4 272 | 4 219 | 4 168 | 53 | 9 550 | 30 149 |
| Kingmaker | 5 582 | 5 529 | 5 474 | 53 | 11 059 | 40 359 |
| Neverwinter Chess | 4 371 | 4 317 | 4 266 | 54 | 9 555 | 30 211 |
| ShadowGuard | 5 396 | 5 343 | 5 280 | 53 | 10 865 | 38 332 |
| The Dark Ranger's Treasure | 4 233 | 4 180 | 4 130 | 53 | 9 372 | 30 482 |
| The Winds of Eremor | 4 234 | 4 181 | 4 131 | 53 | 9 373 | 31 370 |
| To Heir Is Human | 4 250 | 4 197 | 4 147 | 53 | 9 391 | 30 611 |
| Witch's Wake | 5 052 | 4 999 | 4 946 | 53 | 10 235 | 39 265 |

Les huit analyses se terminent sans diagnostic du Resource Manager et sans échec GFF. Les sources
absentes viennent principalement des ressources de base NCS distribuées sans NSS ; elles restent
visibles et consultables techniquement.

## Limites explicites

- L'analyse NSS est lexicale et structurelle ; elle ne prétend pas remplacer le compilateur.
- Aucun compilateur NWScript n'est embarqué. Le mode de vérification reste désactivé et expliqué
  dans l'interface, conformément à la clause « si l'outil retenu le permet ».
- La vue NCS est technique et ne décompile pas le bytecode.
- Les références entrantes sont limitées aux champs GFF observables dont le rôle de script est
  explicite. Les usages calculés dynamiquement par un script ne sont pas inventés.

Le Lot 5 peut s'appuyer sur cet index pour relier chaque condition ou action de dialogue à sa source
NSS, à son NCS et à toutes ses références prouvées.
