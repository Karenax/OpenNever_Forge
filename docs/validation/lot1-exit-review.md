# Revue de sortie — Lot 1

- Date : 3 août 2026
- Statut : accepté, avec limites reportées explicites
- Portée : détection NWN, ouverture MOD, `module.ifo`, HAK/TLK et provenance en lecture seule

## Exigences du lot

| Exigence | État | Preuve |
| --- | --- | --- |
| Interface interne `ContainerReader` | Réussie | `aurora-erf` sépare inventaire et lecture bornée à la demande |
| Lecture ERF/MOD défensive | Réussie | contrôles de bornes, limites, version, annulation et tests de corruption |
| Localisation unique de `module.ifo` | Réussie | diagnostics absent et ambigu, lecture de la seule ressource IFO |
| Adaptateur typé `ModuleInfo` | Réussie | nom, description, tag, version facultative, zone, HAK et TLK |
| Détection ordonnée HAK/TLK | Réussie | priorité utilisateur puis installation, copies masquées visibles |
| Rapport présent/absent/invalide | Réussie | états typés, diagnostics UI et refus de traversée de chemin |
| Empreinte et changements externes | Réussie | SHA-256 en flux et comparaison avec la dernière analyse réussie |
| Fixture à TLK personnalisé | Réussie | fixture CC0 déterministe MOD + HAK + TLK et test d'intégration Rust |
| Oracle externe opt-in | Réussie | huit contrôles concordants avec `neverwinter.nim` 2.1.2 |
| Build release Windows | Réussie | binaire optimisé et installateur NSIS x64 produits |
| Sources NWN immuables | Réussie | ouvertures en lecture seule, aucune extraction hors test synthétique |

## Corpus et mesure indicative

Le parcours en ligne de commande a analysé les huit modules officiels présents dans l'installation
locale : huit réussites, zéro échec. Trois modules déclarent un HAK, tous résolus et empreints ; les
cinq autres n'ont aucune dépendance personnalisée. Aucun de ces modules ne déclare de TLK
personnalisé, ce cas est donc couvert par la fixture synthétique redistribuable.

Mesure locale du binaire de développement, incluant le hash du MOD et des dépendances :

- moyenne : 324,9 ms par module ;
- maximum : 1 154 ms ;
- minimum : 18 ms.

Ces valeurs servent de point de comparaison initial et ne constituent pas encore un budget de
performance contractuel.

## Limites reportées

- la référence servant à détecter un changement externe est conservée pendant la session mais pas
  encore persistée dans SQLite ;
- le lecteur GFF du Lot 1 produit uniquement `ModuleInfo` : les octets inconnus restent intacts dans
  la source, mais ne sont pas encore navigables dans un inspecteur brut ;
- le TLK est résolu et empreint, pas interprété par le cœur ; le lecteur TLK et la résolution des
  `StrRef` appartiennent au Lot 3 ;
- la précédence complète MOD/HAK/override/patch/KEY-BIF et l'ouverture du contenu des HAK
  appartiennent au Resource Manager du Lot 2.

Ces limites ne contredisent pas la porte de sortie du Lot 1 et restent visibles dans le plan. Le Lot
2 peut commencer sans activer d'écriture NWN.
