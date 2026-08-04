# Revue de sortie — Lots 21 et 22

Date : 4 août 2026

## Lot 21 — contenus personnalisés

- `aurora-2da` écrit un format `2DA V2.0` canonique, relisible et stable, avec modification de
  cellule, ajout/suppression de ligne et valeur par défaut.
- `aurora-tlk` écrit un format `TLK V3.0` canonique, conserve les bits inconnus, valide les ResRef
  sonores et permet la modification ou l'ajout d'entrées sans décaler implicitement les StrRef.
- Les commandes Tauri relisent la ressource issue du Resource Manager ou de l'overlay, appliquent
  une opération typée, sérialisent, rouvrent, puis stagèrent une transformation liée aux SHA-256.
- Le gestionnaire graphique HAK/TLK transforme uniquement `Mod_HakList` et `Mod_CustomTlk` dans
  `module.ifo`. Les autres champs sont conservés et la commande participe à l'undo/redo.
- La construction HAK existante reste séparée des archives sources et ne contient que les ressources
  explicitement modifiées.

## Lot 22 — reproductibilité, profils et Git

- Les profils de build sont validés, triés et persistés dans le workspace, hors du module source.
- Le préflight compare les HAK et le TLK attendus au `module.ifo` réellement relu. La politique
  `blockOnWarnings` bloque un profil incohérent.
- La vérification construit deux MOD temporaires indépendants et exige des SHA-256 identiques.
- Un profil produit son MOD dans un dossier choisi et peut demander le déploiement `development`.
- Les profils de test ciblent exclusivement `nwmain.exe` ou `nwserver.exe`, passent des arguments
  bornés sans shell et redirigent stdout/stderr vers un journal du workspace.
- L'intégration Git est en lecture seule : racine, branche, HEAD et statut borné. Elle ne stage,
  committe, change de branche ou pousse rien.

## Preuves automatisées

- round-trip déterministe des writers TLK et 2DA ;
- édition de `module.ifo` avec préservation d'un champ non concerné ;
- persistance des profils de build et de lancement ;
- deux builds MOD identiques par SHA-256 ;
- dépôt Git temporaire inspecté sans mutation ;
- tests d'interface du gestionnaire de dépendances et du profil de build ;
- graphe d'architecture régénéré et vérifié après les modifications.

## Limite externe

Le lancement d'un processus est maintenant intégré, mais la réussite du chargement en jeu reste une
preuve environnementale séparée. Le profil local `nwserver.exe` connu s'arrête encore avant écoute
avec `0xC0000005` sur le témoin comme sur l'overlay ; aucune validation moteur positive n'est donc
inventée dans cette revue.
