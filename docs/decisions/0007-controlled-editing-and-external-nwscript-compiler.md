# ADR 0007 — Édition transactionnelle et compilateur NWScript externe

- Statut : accepté
- Date : 4 août 2026

## Contexte

La Phase 2 doit modifier des ressources sans jamais ouvrir le MOD, les HAK ou l'installation NWN
d'origine en écriture. Un NSS modifié ne peut pas être sauvegardé ou déployé comme terminé sans son
NCS compilé. Le compilateur `nwnsc` est disponible séparément sous des termes permissifs, mais son
binaire ne doit pas être téléchargé silencieusement ni incorporé sans provenance vérifiable.

## Décision

`aurora-edit` conserve un overlay séparé lié au SHA-256 de la source et à son emplacement de travail.
Chaque opération est une
commande typée avec précondition, prévisualisation, journal append-only et curseur undo/redo. Les
révisions binaires sont stockées par contenu afin que l'annulation restaure aussi les octets qui
seront construits ou déployés. Une intention persistée permet de restaurer automatiquement une
transaction interrompue. Les writers GFF et ERF sont internes, déterministes, écrivent les payloads
en streaming et conservent les champs inconnus ainsi que les métadonnées d'archive déjà lues.

La compilation NSS → NCS utilise un exécutable explicitement choisi par l'utilisateur. Son SHA-256,
le NSS principal et chaque include transitif exact sont enregistrés dans la commande. Le processus
est lancé sans shell, avec chemins validés, durée et sorties bornées. Aucun script compilé n'est
exécuté par OpenNever Forge. Un NSS/include modifié après compilation rend le NCS périmé et bloque
le build ou le déploiement.

Les sorties autorisées sont un nouveau `.mod`, un HAK explicitement demandé, un export reproductible
ou le dossier utilisateur `development`. Le nettoyage de `development` ne retire que les fichiers
listés dans le manifeste de l'espace de travail et dont le hash n'a pas changé.
Un fichier déjà revendiqué par un autre overlay est refusé avant toute copie.

## Conséquences

- les archives sources restent immuables, y compris en cas d'échec ;
- une annulation modifie le modèle métier et l'overlay de façon cohérente ;
- le compilateur reste remplaçable et extérieur à la distribution Apache-2.0 ;
- une proposition IA ne peut produire qu'un lot de commandes prévisualisées ; son application reste
  une action utilisateur explicite et réversible ;
- l'ouverture/sauvegarde par Aurora Toolset reste un flux externe, observé en lecture seule par un
  manifeste de synchronisation.

## Validation

- round-trip GFF/ERF, préservation des métadonnées et validation indépendante par
  `neverwinter.nim` 2.1.2 ;
- tests de préconditions, undo/redo des octets, suppression réversible et source modifiée
  extérieurement ;
- validation NSS/NCS/includes bloquante, empreinte du compilateur et arguments sans shell ;
- déploiement, collision inter-workspaces et nettoyage sélectif de `development` ;
- création d'un MOD synthétique avec IFO et zone d'entrée ARE/GIT/GIC canoniques.
