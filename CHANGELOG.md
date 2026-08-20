# Changelog

Tous les changements notables du projet sont consignés ici.

## [Unreleased]

### Added

- Création cartographique MCP complète : catalogue local MOD/HAK/NWN, aperçu et application
  déterministes, inspection avec empreintes, édition de l'environnement, de l'audio, des tuiles,
  instances, triggers, rencontres, transitions et inventaires dans l'overlay réversible.

- Lot 36 : CI Windows étendue au build Tauri/MCP et aux artefacts, audit frontend bloquant,
  contrôles RustSec/cargo-deny et politique explicite de licences et de sources Cargo.
- Lot 37 : cache JSON versionné du catalogue KEY/BIF/override avec signature des sources,
  invalidation automatique, état visible et progression d'analyse par phases ; exemple de
  benchmark reproductible froid/chaud.
- Lot 38 : porte `verify_release.ps1`, manifeste de candidate avec tailles et SHA-256, et diagnostic
  `nwserver` enrichi du code hexadécimal et de l'événement Application Error sans écriture source ;
  le serveur est lancé depuis son dossier `bin/win32`, condition nécessaire à son écoute correcte.
- Lot 39 : Monaco NWScript et graphe React Flow chargés à la demande, module backend de cache
  extrait et budgets bloquants pour bundle, CSS et grands fichiers sources.
- Lot 40 : chaîne locale de distribution avec 17 SBOM CycloneDX, manifeste schéma 2, checksums,
  validation hors workspace, signature Authenticode conditionnelle et workflow GitHub manuel
  protégé. Le serveur NWN réel, l’overlay et la qualification client WOK/PWK/DWK passent, y compris
  la porte fermée puis ouverte ; la signature, le profil Windows propre et la publication restent
  des prérequis externes bloquants.

- Phase 2 : espace d'édition transactionnel lié à l'empreinte source, commandes typées,
  prévisualisation, journal append-only et undo/redo restaurant aussi les octets stagés.
- Writers GFF V3.2 et ERF/MOD/HAK déterministes avec round-trip, suppression réversible et
  réouverture des conteneurs construits.
- Édition contrôlée des champs GFF, NSS Monaco, compilation externe NSS → NCS, déplacement
  d'instances et peinture de tuiles ARE.
- Éditeurs transactionnels des champs DLG, JRL et FAC avec textes localisés, liens de dialogue,
  étapes de journal et réputations, ainsi que profils typés des neuf familles de blueprints.
- Opérations structurelles DLG et JRL : création/suppression de nœuds, liens, départs, catégories et
  étapes, avec réindexation des liens et transformations liées aux SHA-256 avant/après.
- Opérations structurelles FAC : création/suppression de factions et réputations, matrice dirigée
  complétée à l'ajout, protection de la faction PC et réindexation des parents/relations à la suppression.
- Sous-structures de blueprints transactionnelles pour UTC/UTI/UTS/UTE : dons, capacités,
  classes, équipement, propriétés d'objet, sons et profils de rencontre, avec édition imbriquée.
- Inspection générique des ressources hydratée depuis l'overlay actif après modification.
- Éditeur Lot 19 des polygones de triggers/rencontres, points d'apparition, destinations et drapeaux
  de transition, ainsi que des inventaires de placeables et magasins incorporant le blueprint UTI
  résolu complet sans écrire dans le MOD source.
- Atelier Lot 20 WOK/PWK/DWK : lecture des surfaces de faces binaires/ASCII et des anciennes
  ressources `#MAXDOOR`, conservation des variantes et hooks, validation topologique, déplacement,
  découpe, suppression, extrusion et soudure dans l'aperçu SVG. Les writers autonomes WALKMESH,
  PWKMESH et DWKMESH produisent un AABB déterministe et sont relus avant staging. Le remplacement
  d'une ressource existante exige une confirmation explicite.
- Lot 21 : writers déterministes 2DA V2.0 et TLK V3.0, éditions de cellules/lignes et chaînes/sons
  via l'overlay annulable, ainsi qu'un gestionnaire HAK/TLK qui réécrit puis relit `module.ifo`
  sans perdre ses champs inconnus.
- Lot 22 : profils de build persistants avec cohérence des dépendances, vérification par deux MOD
  reconstruits et comparés par SHA-256, exécution avec déploiement `development` facultatif et
  inspection Git bornée en lecture seule via arguments sans shell. Profils `nwmain`/`nwserver`
  persistants, lancement direct borné et journal local séparé.
- Lot 23 : synchronisation bidirectionnelle avec un workspace Aurora Toolset, comparaison à trois
  états, conflits arbitrés explicitement, préconditions SHA-256, imports annulables et sauvegardes
  récupérables avant écriture ou suppression côté Toolset.
- Lot 24 : schéma workspace v3, sauvegarde exacte et historique des migrations, rejet des versions
  futures, moteur de synchronisation isolé, guide utilisateur, guide de migration et ADR dédié.
- Lot 25 : fournisseur IA compatible choisi par l'utilisateur, réseau et partage de données
  désactivés par défaut, clé uniquement en mémoire, import JSON local, opérations GFF/NSS bornées,
  prévisualisation sur les octets courants, confirmation par SHA-256 et application annulable.
- Construction d'un nouveau MOD, déploiement explicite dans `development` et nettoyage sélectif
  par manifeste et SHA-256.
- Fondation Phase 3 : création d'un module vide et de zones ARE/GIT/GIC, palettes typées,
  validation de walkmesh, profils HAK/TLK, export reproductible, synchronisation Aurora contrôlée et
  prévisualisation de propositions IA sous forme de commandes annulables.

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

- Rend les transactions d'édition récupérables après interruption et interdit qu'une commande soit
  validée sans exactement les octets de ressources correspondants.
- Préserve la branche redo lorsqu'une transaction est rejetée et distingue deux overlays ouverts
  sur le même module.
- Invalide un NCS si son NSS ou un include transitif change et lie la compilation au SHA-256 exact
  du compilateur externe.
- Construit les MOD/HAK en streaming, préserve les métadonnées ERF sources et produit un module neuf
  IFO/ARE/GIT/GIC accepté par les oracles `nwn_erf` et `nwn_gff` 2.1.2.
- Rend création/suppression de zone ARE/GIT/GIC atomique, réhydrate les zones créées depuis
  l'overlay et ajoute la suppression réversible d'instances.
- Relit les zones existantes depuis l'overlay après édition et après undo/redo, de sorte que la
  carte 2D, les polygones, transitions et inventaires reflètent exactement le GIT stagé.
- Corrige les offsets des listes GFF imbriquées lors de la sérialisation et refuse les ResRef de
  blueprint invalides avant toute écriture dans l'overlay.
- Refuse les collisions entre workspaces dans `development` et borne le scan des projets Aurora.
- Empêche l'en-tête flottant de l'inventaire de recouvrir le panneau Diagnostics.
- Met à jour le compteur de l'inspecteur lorsque l'inventaire asynchrone arrive.
- Supprime les tableaux GLB optionnels vides, déduplique les joints de skins et neutralise les
  hiérarchies de nœuds cycliques tout en conservant un diagnostic local.
