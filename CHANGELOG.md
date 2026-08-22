# Changelog

Tous les changements notables du projet sont consignés ici.

## [Unreleased]

### Added

- Plan d'amélioration transversal `docs/IMPROVEMENT_PLAN_2026-08-21.md` : CI/couverture,
  remboursement des monolithes (`App.tsx`, `commands.rs`, `aurora-edit/lib.rs`, `tauri.ts`),
  structure frontend alignée sur la refondation UX, qualité code et fixtures à volume réel.
- Lot V — fixtures à volume réel : générateur déterministe `scripts/generate_volume_fixture.py`
  (dialogue de 1 001 nœuds avec cycles, partage et nœuds isolés ; zone 16x15 de 444 instances ;
  manifeste SHA-256, mode `--check`) et test d'intégration
  `crates/aurora-dialogue/tests/volume_fixture.rs` qui reconstruit le DLG, vérifie cycles,
  inaccessibles, partage, absence de liens cassés et bornes d'arbre, avec budget d'adaptation < 5 s.

### Changed

- Monolithes (fin) : `aurora-edit/lib.rs` descend à 6 583 lignes (9 876 au départ) après
  extraction du cœur transactionnel `EditWorkspace` vers `workspace.rs` ; plafond abaissé à 7 000.
- Structure frontend : composants regroupés par atelier sous `features/` (dialogues, exports,
  map-creator, agent-studio, help, shared) avec barrels ; état UI extraits vers les stores Zustand
  `store/uiStore.ts` (volets explorateur/inspecteur/diagnostics) et `store/workbenchStore.ts`
  (élément courant, dernière vue, bref objectif agent). Le barrel `features/shared` n'expose plus
  le composant global `UxEnhancements` afin d'éviter le chargement de son CSS dans l'entrée.
- Monolithes (suite) : `aurora-edit/lib.rs` est réduit de 9 876 à 8 360 lignes par extraction de
  `types.rs`, `walkmesh.rs` et `workspace_io.rs` (API publique inchangée via réexportations) ; les
  trois ateliers d'export partagent désormais le socle `features/exports/ExportWorkshopShell.tsx`
  (état verrouillé, en-tête, destination, consentement, métriques, avertissements) sans changement
  de comportement ni de classes CSS.
- Plafonds de budget sources abaissés pour verrouiller les gains : `App.tsx` 3 050 → 2 950,
  `commands.rs` 7 700 → 7 200, `aurora-edit/lib.rs` 9 950 → 9 000.
- Refactorisation : `lib/tauri.ts` (1 323 lignes) est découpé en modules par domaine derrière le
  barrel `lib/tauri/index.ts` (erreurs, types, analyse, workspace, walkmesh, agent, cartographie,
  exports, inspection, dialogues de fichiers) sans changer l'API importée par les composants.
- Monolithes : les DTOs purs de `commands.rs` sont extraits vers `commands/dto.rs` (7 820 → 6 841
  lignes) et l'atelier dialogue d'`App.tsx` vers `features/dialogues/DialogueWorkshop.tsx`
  (3 090 → 2 825 lignes), ramenant les deux fichiers sous leurs plafonds de budget.
- CI : `ci.yml` se déclenche désormais sur push (main) et pull_request avec groupe de concurrence ;
  il ne restait plus limité aux exécutions manuelles.
- Couverture frontend mesurée et bloquante : Vitest provider v8, rapport text-summary + LCOV,
  seuils initiaux 45 % statements / 50 % branches / 38 % fonctions / 50 % lignes ; la CI Rust
  produit un rapport LCOV via `cargo-llvm-cov`.
- Descriptions ajoutées dans les 20 `Cargo.toml` du workspace.
- Vue 3D : le plan technique est maintenant centré sous la grille de tuiles et la préparation des
  modèles GLB s'effectue en une file parallèle adaptative en arrière-plan avant leur intégration
  dans Babylon.js, afin d'éviter les attentes croisées entre modèles et textures ; les occurrences
  d'un même modèle partagent désormais leurs matériaux au lieu de les cloner individuellement ;
  les orientations de tuiles NWN compensent le demi-tour ajouté par la conversion automatique du
  GLB droitier vers le repère gauche de Babylon.js avant d'appliquer les quarts de tour antihoraires.

### Added — fonctionnalités produit

- Nouvel atelier « Exporter des dialogues », troisième entrée du menu supérieur Export : sélection
  de la révision analysée ou modifiée dans le workspace, prise en charge des DLG nouvellement créés,
  copie exacte du DLG, JSON portable, transcript Markdown et manifeste SHA-256, avec conservation
  explicite des cycles, liens cassés, scripts, références et diagnostics sans écriture dans les
  sources NWN.

- Nouvel atelier « Exporter des assets », deuxième entrée du menu supérieur Export : sélection d’un
  modèle Aurora analysé, qualification statique ou animée fondée sur le GLB produit, résolution des
  animations de supermodel et des textures, export local GLB + PNG + manifeste SHA-256 par
  publication atomique dans une destination neuve, sans modification des sources NWN.

- Réintégration de l’exportateur de cartes sous l’atelier « Exporter une carte » : audit d’une
  zone analysée, bundle local `area-migration-bundle@1.0.0`, conversion bornée des modèles et
  textures, préservation explicite des navigations WOK/PWK/DWK, diagnostics, annulation et
  vérification atomique sans modification des sources NWN.

- Ouverture directe d’une carte `.are` en lecture seule, avec chargement automatique des ressources
  voisines `.git`/`.gic`, résolution des ressources NWN et accès immédiat aux vues de zone sans
  exiger de conteneur `.mod`.

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

- Empêche l’atelier Dialogues de charger automatiquement le premier DLG, borne le navigateur de
  lignes, rend les cibles de liens paresseuses et limite la resynchronisation UX aux hôtes utiles.
- Recharge l'analyse complète du dernier module depuis le cache local au démarrage, y compris pour
  les dialogues profonds qui dépassaient auparavant la limite d'enveloppe JSON de Serde.
- Oriente les faces MDL→GLB du côté de leurs normales après la conversion Z-up→Y-up, invalide le
  cache concerné et abaisse le plan technique Babylon afin qu'il ne masque plus les sols texturés.
- Rétablit la porte de publication Windows en recalant ses budgets de confinement et en corrigeant
  l'analyse PowerShell des scripts SBOM et de vérification de distribution.
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
