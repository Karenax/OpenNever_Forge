# Migrations des projets OpenNever Forge

## Politique

- Le format persistant du workspace est versionné indépendamment du MOD NWN.
- Une migration ne modifie jamais le module source.
- Avant toute migration, les octets exacts de `workspace.json` sont copiés dans
  `workspace.json.v<ancienne-version>.bak`.
- Les migrations connues sont ascendantes et séquentielles. Les schémas futurs sont refusés.
- Les ressources stagées, blobs d’historique, suppressions, curseur undo/redo et transactions en
  attente sont conservés ou récupérés avant la reprise du travail.

## Versions

### Schéma 1

État transactionnel initial : source immuable, valeurs, commandes et ressources modifiées.

### Schéma 2

Ajout des révisions de ressources, des suppressions atomiques et de la récupération fiable d’une
transaction interrompue.

### Schéma 3

Ajout de l’historique de migration et des baselines de synchronisation Toolset stockées séparément
dans `aurora-sync/`. Une baseline est identifiée par l’empreinte du chemin canonique du workspace
Toolset et ne contient que des noms de ressources et des SHA-256.

## Procédure de retour arrière

1. Fermer OpenNever Forge.
2. Copier le workspace complet avant intervention.
3. Restaurer le fichier `.bak` en tant que `workspace.json`.
4. Utiliser une version OpenNever compatible avec l’ancien schéma.

Ne jamais remplacer un `workspace.json` récent par sa sauvegarde pendant que l’application est
ouverte. Les commandes ajoutées depuis la migration ne seraient pas présentes dans l’ancienne copie.
