# Changelog

Tous les changements notables du projet sont consignés ici.

## [Unreleased]

### Added

- Socle initial Tauri 2, React 19, TypeScript strict et Rust stable.
- Architecture en crates pour les erreurs, les projets en lecture seule et l'index SQLite.
- Shell sombre de l'éditeur et premier job de hash SHA-256 annulable.
- Graphe d'architecture déterministe et contrôle de fraîcheur.
- Lecteur ERF/MOD borné, inventaire des ressources et accès à la demande limité à 16 Mio.
- Lecteur GFF V3.2 minimal pour le nom du module, sa version, sa zone d'entrée, ses HAK et son TLK.
- Explorateur filtrable et inspecteur synchronisé avec les résultats de l'analyse native.
- Rapport de dépendances HAK/TLK avec provenance, priorité utilisateur, copies masquées et
  diagnostics explicites pour les fichiers absents ou non vérifiés.
- Empreinte SHA-256 en flux des HAK/TLK et détection des changements externes entre deux analyses
  réussies du même module pendant une session.
- Fixture CC0 déterministe avec MOD, HAK et TLK personnalisé, test d'intégration complet et outil de
  comparaison opt-in avec `neverwinter.nim`.
- Resource Manager unifié pour development, override, MOD, HAK, patch et KEY/BIF, avec provenance,
  priorités, collisions, versions masquées, pagination et extraction sûre à la demande.
- Lecteur GFF générique défensif couvrant tous les types V3.2, inspecteur brut paresseux et
  adaptateurs pour ARE, GIT, GIC et blueprints.
- Lecteurs TLK et 2DA avec résolution localisée (langue, genre, origine, état), gestion des versions
  2DA et comparaison cellule par cellule.
- Migrations SQLite du catalogue de ressources, des résumés structurés et des baselines de
  dépendances persistantes.
- Revue de sortie des Lots 2 et 3 validée sur huit modules officiels : 100 % des GFF ouverts et
  aucun diagnostic de résolution de ressource.
- Index NWScript unifiant NSS/NCS, includes, fonctions, constantes, appels et références entrantes
  issues des champs GFF avec chemin de preuve.
- Recherche plein texte paginée, Monaco NWScript strictement en lecture seule et vue technique NCS
  séparée avec empreinte et aperçu hexadécimal.
- Migration SQLite v4 pour les sources, symboles, includes et relations de scripts.
- Fixture synthétique enrichie de deux NSS, d'un NCS et d'un include résolu ; revue du Lot 4 validée
  sur les huit modules officiels.
- Adaptateur `DialogueGraph` conservant le GFF brut, les nœuds Entry/Reply, les racines et tous les
  liens, avec diagnostics des cycles, partages, inaccessibles et cibles cassées.
- Vues dialogue en arbre borné, graphe React Flow complet et inspecteur GFF, avec textes localisés,
  locuteurs, commentaires, animations, sons, quêtes, conditions et actions.
- Navigation créature/ressource → dialogue → script → références entrantes et recherche native
  paginée par texte, locuteur, ResRef ou ressource.
- Migration SQLite v5 pour les dialogues, nœuds, liens et références ; revue du Lot 5 validée sur
  les huit modules officiels.
- Adaptateurs JRL/FAC, textes localisés, étapes finales, matrice de factions et rapprochements
  dialogue/script avec provenance et niveaux de confiance.
- Agrégat ARE/GIT/GIC, carte 2D orientée et inventaire positionné de toutes les instances.
- Inspection bornée des MDL/TGA/DDS/PLT/TXI avec diagnostics dégradés locaux et empreinte de cache.
- Manifeste de scène Rust, vue Babylon.js à chargement différé, caméra orbitale, picking, overlays et
  marqueurs techniques.
- Graphe global indépendant de React, diagnostics transversaux et rapports JSON/HTML anonymisés.
- Migration SQLite v6 pour le rapport Phase 1 et première revue des Lots 6 à 10 sur huit modules.
- Lecteur `aurora-mdl` Apache-2.0 indépendant pour MDL binaire/ASCII, trimesh, skins, supermodèles,
  références, animations et walkmeshes AABB, fondé sur la description CC0 `NWN1MDL.bt`.
- Export GLB 2.0 déterministe, cache atomique versionné par hash composite et commandes IPC binaires
  pour les aperçus de modèles et textures.
- Viewer Babylon.js de modèles avec cadrage, animation, skins et chargeurs DDS/TGA/KTX explicites ;
  aperçu autonome des textures et conversion locale PLT vers PNG par couches.
- Assemblage 3D complet des zones : modèles de tuiles issus des SET, portes/placeables issus des
  blueprints et 2DA, créatures composites UTC, instanciation GLB progressive, budget mémoire,
  picking, surbrillance, vues orbitale/Aurora et modes overlays/walkmesh/filaire.
- Validateur Khronos GLB Apache-2.0 en développement et preuves réelles sur modèle animé, skinné,
  référencé, walkmesh et tuile sans modification du module source.

### Fixed

- Empêche l'en-tête flottant de l'inventaire de recouvrir le panneau Diagnostics.
- Met à jour le compteur de l'inspecteur lorsque l'inventaire asynchrone arrive.
- Supprime les tableaux GLB optionnels vides, déduplique les joints de skins et neutralise les
  hiérarchies de nœuds cycliques tout en conservant un diagnostic local.
