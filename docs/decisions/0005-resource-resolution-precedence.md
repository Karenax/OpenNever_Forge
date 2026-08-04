# ADR 0005 — Priorité et provenance du Resource Manager

- Statut : accepté pour les Lots 2–3
- Date : 3 août 2026

## Contexte

Une même `ResourceKey` peut exister dans plusieurs répertoires, le module, plusieurs HAK et les
KEY/BIF du jeu. L’interface doit montrer la version choisie et toutes les versions masquées. Une
priorité implicite ou dépendante de l’ordre du système de fichiers rendrait l’analyse non
reproductible.

L’implémentation de référence `neverwinter.nim` décrit un ResMan où le dernier conteneur ajouté est
interrogé en premier et conserve l’origine de chaque ressource. Son constructeur NWN:EE charge les
KEY de base, puis les ERF additionnels, les couches de synchronisation et enfin les répertoires de
surcharge. Ces références servent d’oracle de comportement sans intégrer leur code :

- <https://github.com/niv/neverwinter.nim/blob/master/neverwinter/resman.nim>
- <https://github.com/niv/neverwinter.nim/blob/master/neverwinter/game.nim>

## Décision

`ResourceKey` est le couple normalisé `ResRef ASCII insensible à la casse + ResourceType u16`.
Chaque version conserve son type de source, son libellé, son chemin, son rang, son offset, sa taille,
son emplacement de lecture et son hash lorsqu’il a été calculé.

L’ordre de résolution, du plus fort au plus faible, est :

1. `development` utilisateur, activé comme couche de test explicite ;
2. `override` utilisateur ;
3. `lang/<lang>/data/ovr`, puis `ovr` de l’installation ;
4. le module MOD ;
5. les HAK déclarés par `module.ifo`, dans l’ordre de la liste, le premier étant prioritaire ;
6. les KEY de correctifs identifiées explicitement ;
7. `nwn_retail_loc`, `nwn_retail`, `nwn_base_loc`, `nwn_base`, puis les autres KEY dans un ordre
   lexical stable.

Un rang numérique croissant matérialise cet ordre. En cas d’égalité, le chemin normalisé puis
l’offset départagent les versions afin de garantir un résultat déterministe. La version de rang le
plus faible est `selected`; toutes les autres restent dans `shadowed`.

Les répertoires sont lus comme des conteneurs plats. Les chemins KEY/BIF sont refusés s’ils sont
absolus ou contiennent `..`. Les lectures de payload restent à la demande, bornées et en lecture
seule. Une extraction éventuelle cible exclusivement un cache séparé avec un nom canonique dérivé
de la `ResourceKey`.

## Conséquences

- chaque octet demandé possède une provenance explicable ;
- les collisions ne détruisent aucune information ;
- l’index SQLite peut être remplacé atomiquement à partir du hash de la source ;
- une source KEY/BIF invalide produit un diagnostic sans rendre invisibles les autres conteneurs ;
- NWSync et les contenus de projet éditables restent hors de la Phase 1 et devront obtenir un rang
  explicite avant leur activation.

Le choix de langue est actuellement `en` lorsqu’il existe, sinon le premier répertoire de langue
trié. Un sélecteur de langue explicite devra remplacer cette détection avant la distribution
multilingue.
