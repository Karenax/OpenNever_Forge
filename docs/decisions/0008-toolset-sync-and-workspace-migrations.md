# ADR 0008 — Synchronisation Toolset à trois états et migrations sauvegardées

Date : 4 août 2026
Statut : accepté

## Contexte

Aurora Toolset extrait un module dans un dossier temporaire et le recrée depuis la dernière
sauvegarde. OpenNever possède parallèlement un overlay transactionnel. Une simple copie dans les
deux sens écraserait les modifications concurrentes et rendrait les suppressions ambiguës.

## Décision

- Comparer chaque ressource par clé NWN et SHA-256 entre Toolset, OpenNever et une baseline persistée.
- Classer les écarts sans fusion implicite ; tout conflit nécessite une direction choisie.
- Revérifier les empreintes au moment de l’application.
- Importer dans OpenNever uniquement par commandes transactionnelles annulables.
- Sauvegarder toute ancienne version Toolset dans `.opennever-backups` avant remplacement ou retrait.
- Ignorer les liens symboliques, borner profondeur, quantité, taille et ResRef.
- Bloquer l’envoi de NSS stagés si leur compilation NCS n’est plus exacte.
- Migrer les workspaces avec une copie byte-for-byte préalable et refuser les versions futures.

## Conséquences

La synchronisation reste volontaire et traçable. Elle ne remplace pas l’action **Sauvegarder** du
Toolset. Les baselines et sauvegardes consomment de l’espace hors du module, mais permettent de
diagnostiquer et récupérer un échange erroné sans toucher à la source `.mod`.
