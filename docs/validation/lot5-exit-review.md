# Revue de sortie — Lot 5

- Date : 3 août 2026
- Statut : accepté
- Portée : adaptation DLG, arbre borné, graphe complet, scripts et provenance

## Exigences

| Exigence | État | Preuve |
| --- | --- | --- |
| `DialogueGraph` fidèle | Réussie | listes Entry/Reply, StartingList et liens conservés séparément du GFF brut |
| Arbre simplifié | Réussie | expansion bornée avec marqueurs explicites de cycle et de lien partagé |
| Graphe complet | Réussie | React Flow affiche tous les nœuds et liens sans transformer le DLG en faux arbre |
| Inspecteur brut | Réussie | GFF générique original disponible à la demande |
| Textes localisés | Réussie | texte embarqué, `dialog.tlk` et TLK personnalisé résolus par le service du Lot 3 |
| Métadonnées | Réussie | locuteur, commentaire, animation, boucle, son et quête exposés |
| Conditions et actions | Réussie | scripts des nœuds et liens navigables via l'index du Lot 4 |
| Structures difficiles | Réussie | cycles, partages, inaccessibles et cibles cassées diagnostiqués séparément |
| Navigation créature ↔ dialogue | Réussie | champs Conversation prouvés par ressource et chemin GFF |
| Recherche | Réussie | ResRef, texte, locuteur et ressource entrante, avec pagination native |
| Persistance | Réussie | migration SQLite v5 pour dialogues, nœuds, liens et références |
| Build release Windows | Réussie | application et installateur NSIS x64 reconstruits |
| Sources NWN immuables | Réussie | aucune écriture DLG/GFF et aucun writer lié au parcours |

## Validation sur le corpus local

| Module | DLG | Nœuds | Liens | Partagés | Cycles | Références |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Contest of Champions | 53 | 18 970 | 28 059 | 3 852 | 1 106 | 976 |
| Kingmaker | 162 | 29 139 | 43 072 | 5 769 | 1 967 | 1 254 |
| Neverwinter Chess | 42 | 18 866 | 27 807 | 3 845 | 1 054 | 931 |
| ShadowGuard | 119 | 24 780 | 35 537 | 4 559 | 1 376 | 1 192 |
| The Dark Ranger's Treasure | 42 | 18 916 | 27 964 | 3 911 | 1 082 | 930 |
| The Winds of Eremor | 48 | 18 863 | 27 835 | 3 870 | 1 056 | 953 |
| To Heir Is Human | 48 | 19 155 | 28 244 | 3 948 | 1 056 | 937 |
| Witch's Wake | 106 | 21 425 | 33 582 | 4 916 | 1 527 | 1 066 |

Les huit analyses terminent avec zéro lien cassé et zéro nœud inaccessible. Les cas cassés et
inaccessibles restent couverts par la fixture unitaire synthétique ; les cycles, ramifications et
liens partagés sont présents dans le corpus réel et dans le test UI borné.

## Limites explicites

- La vue arbre est volontairement une projection : un nœud déjà vu devient un marqueur « lien
  partagé » et un retour dans la branche devient un marqueur « cycle ».
- Le graphe complet utilise un placement déterministe en deux colonnes. Un futur moteur de layout
  pourra améliorer la lisibilité sans changer `DialogueGraph`.
- Les références calculées dynamiquement par NWScript ne sont pas inventées ; seules les preuves
  GFF et les scripts explicitement déclarés sont affichés.
- Le Lot 5 reste un lecteur : aucune édition de texte, de lien ou de script n'est activée.

Le Lot 6 peut réutiliser les textes localisés et les liens dialogue-script pour rapprocher journal,
quêtes et factions tout en conservant des niveaux de confiance explicites.
