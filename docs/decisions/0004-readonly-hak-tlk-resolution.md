# ADR 0004 — Résolution HAK/TLK en lecture seule

- Statut : accepté
- Date : 3 août 2026

## Contexte

`module.ifo` déclare les HAK et, éventuellement, un TLK personnalisé nécessaires à l'ouverture
fidèle d'un module. Le Lot 1 doit rendre les dépendances manquantes et les conflits visibles sans
ouvrir les archives, sans parcourir arbitrairement le disque et sans anticiper le Resource Manager
complet du Lot 2.

## Décision

L'analyse du module reçoit deux racines facultatives et inspecte une liste bornée d'emplacements :

1. données utilisateur : `hak/` ou `tlk/` ;
2. installation du jeu : `data/hk/` ou `data/tlk/` ;
3. dossiers historiques de l'installation : `hak/` ou `tlk/`.

La première copie existante est sélectionnée et les suivantes restent exposées comme copies
masquées. Une dépendance est `resolved`, `missing`, `unchecked` quand aucune racine utile n'est
renseignée, ou `invalid` si son nom pourrait sortir des dossiers autorisés. Les chemins sont
transmis au frontend uniquement comme provenance ; aucun fichier HAK ou TLK n'est ouvert pendant
cette étape de résolution.

Après résolution, chaque fichier sélectionné est ouvert uniquement en lecture et reçoit une
empreinte SHA-256 calculée en flux. Le registre de jobs conserve le dernier rapport réussi pour le
même chemin de module pendant la session. L'analyse suivante classe chaque dépendance comme
inchangée, contenu modifié, source prioritaire modifiée, devenue disponible ou devenue absente.
Une analyse échouée ne remplace jamais la référence de comparaison.

## Conséquences

- la priorité utilisateur sur installation est explicite et testée ;
- les absences, racines manquantes, noms invalides et copies masquées produisent des diagnostics
  visibles ;
- la vérification est déterministe, non récursive et refuse la traversée de chemin ;
- cette décision ne définit pas encore toute la précédence des ressources NWN : modules, HAK,
  override, patch et ressources du jeu seront unifiés par le Resource Manager du Lot 2.
- la référence de comparaison n'est pas encore persistée entre deux lancements de l'application ;
  cette persistance dépendra du projet local et de l'index SQLite, sans jamais écrire dans NWN.
