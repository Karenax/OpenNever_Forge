# Plan de construction — NWN Editor

Version : 0.1
Date : 2 août 2026
Source : `C:\Users\karen\Downloads\NWN_EDITOR_CONTEXT.md`
État initial constaté : `E:\OpenNever_Forge` est vide et n'est pas encore un dépôt Git.

## 1. Objectif du plan

Construire progressivement un remplaçant moderne d'Aurora pour Neverwinter Nights: Enhanced Edition, en commençant par un explorateur complet, fidèle et strictement en lecture seule.

La séquence est contraignante :

```text
Fondations
→ ouverture sûre d'un projet local
→ lecture MOD/HAK/KEY/BIF
→ résolution fiable des ressources
→ GFF/TLK/2DA et modèle métier
→ scripts, dialogues, journal et factions
→ navigation 2D
→ modèles, textures et rendu 3D
→ graphe global et validation
→ seulement ensuite édition, compilation et export
```

## 2. Règles qui gouvernent tous les lots

1. Le `.mod`, les HAK et l'installation NWN d'origine sont immuables.
2. Le projet et le cache sont séparés des fichiers source.
3. L'interface ne parse jamais directement un format NWN.
4. Une donnée inconnue est conservée et signalée, jamais supprimée ou interprétée silencieusement.
5. Chaque donnée affichée conserve sa provenance, son hash, sa priorité de résolution et ses références.
6. Les imports sont incrémentaux, annulables et résilients aux ressources invalides.
7. Aucune ressource propriétaire n'entre dans le dépôt, les logs ou la CI.
8. Aucune dépendance n'est ajoutée avant examen de sa licence, de sa maintenance et de son rôle exact.
9. Les outils GPL restent des références ou des processus externes tant qu'une décision de licence n'autorise pas autre chose.
10. Chaque lot livre un incrément installable, testé et démontrable sur Windows 10/11.

## 3. Architecture cible

```text
React 19 / TypeScript strict
  └─ API Tauri typée, paginée et annulable
      └─ Services applicatifs Rust
          ├─ modèle métier NWN
          ├─ graphe de références
          ├─ index SQLite
          ├─ Resource Manager
          │   └─ lecteurs ERF/MOD/HAK/KEY/BIF/GFF/TLK/2DA/SET
          └─ pipeline d'assets MDL/TGA/DDS/PLT/TXI → cache → Babylon.js
```

Le frontend reçoit des DTO légers et des manifestes. Les gros assets sont servis depuis le cache local sécurisé, pas transportés comme de grands tableaux JSON.

## 4. Jalons du programme

| Jalon | Lots | Résultat utilisable | Porte de sortie |
|---|---:|---|---|
| M0 — Socle installable | 0 | Application vide mais robuste, projets locaux, hash annulable | Build, installateur et tests Windows verts |
| M1 — Explorateur de ressources | 1 à 3 | Ouverture d'un module, dépendances, provenance, GFF/TLK/2DA | Un ResRef peut être résolu et expliqué sans perte visible |
| M2 — Compréhension fonctionnelle | 4 à 7 | Scripts, dialogues, quêtes, factions et carte 2D navigables | Parcours bidirectionnel entre objets, scripts, dialogues et zones |
| M3 — Lecture visuelle complète | 8 à 10 | Modèles, textures, zones 3D, graphe global et diagnostic | Phase 1 validée sur le corpus représentatif |
| M4 — Édition contrôlée | Phase 2 | Modifications réversibles et nouveau `.mod` testable | Round-trip et validation sans écraser les sources |
| M5 — Remplacement d'Aurora | Phase 3 | Création et maintenance complète d'un module | Module complexe produit et testé sans Aurora |

## 5. Phase 0/1 — lecture seule complète

### Lot 0 — socle du projet

Objectif : obtenir une application installable avant tout parsing complexe.

Sous-lots :

- **0.1 Gouvernance et dépôt** : Git, licence à décider entre MIT et Apache-2.0, conventions, `CONTEXT.md`, `README.md`, `CONTRIBUTING.md`, `CHANGELOG.md`, `THIRD_PARTY_NOTICES.md`, modèle d'ADR et politique sur les fixtures.
- **0.2 Workspace** : workspace Cargo, application Tauri 2, React 19, Vite, TypeScript strict, formatage et lint.
- **0.3 Coquille d'éditeur** : écran d'accueil, menu, Explorateur, Zone de travail, Inspecteur, Diagnostics, thème sombre accessible et disposition persistée.
- **0.4 Noyau transversal** : modules Rust `project`, `jobs`, `errors`, `logging`, `database`; erreurs structurées; corrélation des imports; logs rotatifs.
- **0.5 Projet local** : schéma versionné du fichier projet, sélection du `.mod`, installation et dossier utilisateur, validation des chemins, lecture seule garantie.
- **0.6 Jobs** : calcul SHA-256 en flux, progression, annulation coopérative, fermeture propre et récupération après erreur.
- **0.7 SQLite** : migrations, tables minimales `projects`, `import_jobs`, `diagnostics`, `source_containers` et métadonnées de cache.
- **0.8 Qualité et livraison** : tests Rust et frontend, CI Windows, build release, installateur et contrôle des licences.
- **0.9 Graphe d'architecture** : générateur local déterministe, preuves fichier/ligne, requêtes bornées, artefacts JSON/Mermaid, tests de reproductibilité, règles `AGENTS.md` et contrôle de fraîcheur en CI.

Critères d'acceptation :

- l'application s'installe et démarre sans environnement de développement ;
- un projet en lecture seule est créé et rouvert ;
- le hash d'un gros fichier peut être annulé et ne laisse pas d'état incohérent ;
- aucun chemin n'est codé en dur ;
- les erreurs sont visibles, compréhensibles et corrélées ;
- aucune extraction de ressource NWN n'est encore effectuée ;
- l'architecture permet d'ajouter le Resource Manager sans réécrire l'UI.
- le graphe d'architecture est généré, déterministe, interrogeable et déclaré frais par la CI.

Charge indicative : **3 à 5 semaines-personne**.

### Lot 1 — détection de NWN et ouverture d'un module

Objectif : lire juste assez du conteneur et de `module.ifo` pour identifier le module et ses dépendances.

Travaux :

- interface interne `ContainerReader` ;
- lecture minimale et défensive ERF/MOD ;
- localisation de `module.ifo` ;
- lecteur GFF minimal nécessaire à l'IFO, sans perdre les champs non modélisés ;
- adaptateur typé `ModuleInfo` ;
- détection ordonnée des HAK et du TLK personnalisé ;
- rapport présent/absent/invalide ;
- hash des dépendances trouvées et détection des changements externes ;
- comparaison opt-in avec au moins un oracle externe.

Porte de sortie : plusieurs modules, dont un avec HAK et un avec TLK personnalisé, affichent correctement leurs métadonnées et dépendances sans modification des sources.

Charge indicative : **3 à 5 semaines-personne**.

### Lot 2 — Resource Manager et explorateur brut

Objectif : établir la source de vérité de toutes les résolutions de ressources.

Travaux :

- lecteurs ERF/MOD/HAK puis KEY/BIF derrière des interfaces remplaçables ;
- type stable `ResourceKey = ResRef + ResourceType` ;
- table de priorité explicite, documentée par ADR ;
- versions sélectionnées et masquées ;
- index incrémental par hash ;
- extraction à la demande dans un cache protégé contre la traversée de chemins ;
- pagination, filtres, recherche par ResRef/type/source ;
- inspecteur binaire minimal pour les formats inconnus ;
- diagnostic des collisions, ombres et absences.

Porte de sortie : pour toute ressource recherchée, l'application explique quelle version est utilisée, pourquoi, quelles versions sont masquées et d'où provient chaque octet.

Charge indicative : **6 à 10 semaines-personne**. Risque élevé.

### Lot 3 — GFF, TLK, 2DA et premiers objets métier

Objectif : passer d'une liste de ressources à des données NWN compréhensibles.

Travaux :

- lecteur GFF générique complet, borné et défensif ;
- conservation des types, listes, structures, ordre utile et champs inconnus ;
- inspecteur GFF brut paresseux ;
- `LocalizedStringResolver` avec textes embarqués, `dialog.tlk`, TLK personnalisé, langue, genre et état de résolution ;
- `2DAManager` avec `****`, sources, versions et accès typés ;
- comparaison de versions 2DA ;
- adaptateurs `ModuleInfo`, `AreaDefinition`, `AreaInstances`, `AreaToolsetData` et blueprints prioritaires ;
- migration SQLite pour métadonnées, chaînes et objets métier.

Porte de sortie : tout GFF du corpus s'ouvre sans perte silencieuse; les noms localisés et les lignes 2DA indiquent toujours leur origine.

Charge indicative : **7 à 11 semaines-personne**. Risque élevé.

### Lot 4 — scripts en lecture

Objectif : naviguer entre un objet et ses scripts sans supposer que les sources existent.

Travaux :

- inventaire NSS/NCS ;
- Monaco en lecture seule avec grammaire NWScript ;
- index des includes, symboles, constantes et références détectables ;
- recherche plein texte ;
- diagnostics de compilation en mode vérification si l'outil retenu le permet ;
- vue technique séparée pour NCS ;
- liens entrants depuis les événements de module, zones, dialogues et blueprints.

Porte de sortie : depuis une instance, ouvrir son script puis retrouver toutes les ressources qui l'utilisent; une absence de NSS est explicitement signalée.

Charge indicative : **4 à 6 semaines-personne**.

### Lot 5 — dialogues

Objectif : représenter fidèlement les DLG, y compris leurs structures non arborescentes.

Travaux :

- adaptateur `DialogueGraph` conservant le GFF brut ;
- arbre simplifié, graphe complet React Flow et inspecteur brut ;
- textes localisés, conditions, actions, commentaires, animations et sons ;
- liens partagés, cycles, nœuds inaccessibles et liens cassés ;
- navigation créature → dialogue → script → références entrantes.

Porte de sortie : dialogues simples, ramifiés, cycliques et partagés affichés sans boucle UI, perte de nœud ou fausse structure d'arbre.

Charge indicative : **5 à 8 semaines-personne**.

### Lot 6 — journal, quêtes et factions

Objectif : reconstituer la structure narrative observable du module.

Travaux :

- viewers JRL et FAC ;
- catégories, étapes, états finaux et textes localisés ;
- matrice des relations de factions ;
- rapprochements avec scripts et dialogues ;
- niveaux de confiance `certain`, `probable`, `possible` ;
- recherche par nom, tag, variable et StrRef.

Porte de sortie : la structure narrative principale est consultable sans Aurora et toute relation déduite est clairement distinguée d'une référence explicite.

Charge indicative : **3 à 5 semaines-personne**.

### Lot 7 — carte 2D des zones

Objectif : offrir une navigation spatiale fiable, indépendante du renderer 3D.

Travaux :

- agrégation ARE/GIT/GIC dans l'entité `Area` ;
- lecture SET via le Resource Manager ;
- grille, tuiles, orientations, instances et transitions ;
- filtres, recherche, centrage, sélection et inspecteur ;
- minimap existante ou rendu top-down simplifié ;
- diagnostics de coordonnées, tileset ou blueprint manquant.

Porte de sortie : toutes les instances GIT du corpus sont retrouvables sur la carte avec leurs coordonnées, leur type et leur provenance.

Charge indicative : **5 à 8 semaines-personne**.

### Lot 8 — modèles, textures et animations

Objectif : valider le pipeline d'assets avant l'assemblage de zones 3D.

Travaux :

- spike comparatif sur la stratégie MDL et ADR ;
- première chaîne acceptable : conversion binaire → ASCII encapsulée, parsing ASCII, génération GLB de cache ;
- progression par nœuds : dummy, trimesh, reference, light, skin, emitter, AABB ;
- supermodels, animations et keyframes ;
- TGA, DDS, PLT, TXI puis MTR selon le corpus ;
- matériaux, transparence et cache versionné ;
- viewer isolé et aperçus de blueprints ;
- captures de référence et mode dégradé documenté.

Porte de sortie : un corpus représentatif de tuiles, créatures, portes et placeables est reconnaissable; chaque fonction non supportée produit un diagnostic local et n'empêche pas le reste du modèle de s'afficher.

Charge indicative : **10 à 18 semaines-personne**. Risque très élevé.

### Lot 9 — vue 3D des zones

Objectif : assembler les données déjà validées sans introduire une seconde logique de résolution.

Travaux :

- manifeste de scène produit par Rust ;
- assemblage des tuiles et placement des instances dans Babylon.js ;
- caméra libre et caméra proche d'Aurora ;
- picking, surbrillance, inspecteur et navigation vers la source ;
- overlays pour triggers, encounters, waypoints, sons et walkmeshes lus ;
- chargement progressif, annulation, budget mémoire et statistiques ;
- marqueurs techniques pour ressources manquantes/non supportées ;
- comparaison visuelle avec Aurora ou NWN:EE.

Porte de sortie : plusieurs zones de tailles et de tilesets différents sont visuellement reconnaissables, navigables et résilientes aux assets manquants.

Charge indicative : **8 à 14 semaines-personne**. Risque très élevé.

### Lot 10 — graphe global et validation

Objectif : prouver que l'éditeur comprend les relations du module.

Travaux :

- modèle de nœuds et d'arêtes indépendant de React Flow ;
- provenance et niveau de confiance pour chaque relation ;
- références entrantes/sortantes ;
- détection des orphelins, scripts/blueprints manquants, StrRef invalides, transitions cassées et ressources masquées ;
- vues ciblées plutôt qu'un graphe géant inutilisable ;
- rapport JSON stable et rapport HTML autonome sans ressource propriétaire ;
- paquet de diagnostic avec chemins anonymisés.

Porte de sortie : rapport reproductible sur l'état d'un module, navigation jusqu'à la preuve source et aucune modification des fichiers d'origine.

Charge indicative : **5 à 8 semaines-personne**.

### Gate Phase 1 — autorisation d'éditer

La Phase 2 reste bloquée tant que les points suivants ne sont pas démontrés sur le corpus local :

- toutes les dépendances sont ouvertes ou diagnostiquées ;
- les versions de ressources et leur ordre de priorité sont explicables ;
- tous les GFF s'affichent sans perte silencieuse ;
- TLK, 2DA, scripts, dialogues, journal, factions et blueprints sont navigables ;
- toutes les zones ont une carte 2D et une vue 3D exploitable ;
- les principaux modèles, textures et animations sont visualisables ;
- les références croisées et les éléments non supportés sont rapportés ;
- aucun test n'a écrit dans une source NWN.

Charge totale indicative de la Phase 1, stabilisation comprise : **environ 67 à 110 semaines-personne**. Cette fourchette doit être réestimée après les spikes Resource Manager, GFF et MDL.

## 6. Phase 2 — édition contrôlée

Cette phase se construit en six incréments, toujours dans un espace de travail séparé :

1. **Socle d'édition** : modèle de commandes typées, validation, prévisualisation, undo/redo, journal append-only et copie logique des seules ressources modifiées.
2. **Sérialisation sans perte** : writers GFF/ERF et tests de round-trip structurel et sémantique; les champs inconnus sont préservés.
3. **Éditeurs métier** : propriétés de module, blueprints, dialogues, journal et factions, chaque action produisant une commande réversible.
4. **NWScript** : édition NSS, diagnostics, compilation NSS → NCS et gestion sûre des includes, sans exécution du script.
5. **Zones** : déplacement/ajout d'instances, modification de tuiles, transitions, sélection et gizmos; aucune écriture directe du MOD.
6. **Build et test** : validation bloquante configurable, construction déterministe d'un nouveau `.mod`, comparaison avec la source et lancement explicite dans NWN:EE.

Porte de sortie : un module de test synthétique puis un module utilisateur autorisé peuvent être modifiés, reconstruits, rouverts dans NWN Editor et lancés dans le jeu, tout en conservant l'original intact.

## 7. Phase 3 — remplacement complet d'Aurora

Ordre proposé :

1. création d'un module vide et gestion des palettes ;
2. création/suppression de zones, peinture de tuiles et placement 3D ;
3. portes, transitions, triggers, encounters et inventaires ;
4. gestion avancée des walkmeshes ;
5. HAK, TLK, 2DA et contenus personnalisés ;
6. builds reproductibles, profils de test et intégration Git ;
7. synchronisation temporaire avec les projets Aurora existants ;
8. documentation, analyse et refactoring ;
9. assistance IA contrôlée, uniquement sous forme d'opérations validées, prévisualisées et annulables.

Le critère final est fonctionnel : créer, maintenir, compiler et tester un module complexe sans ouvrir Aurora.

## 8. Chantiers transversaux obligatoires

### Tests et corpus

- Fixtures synthétiques minimales pour chaque parser.
- Tests locaux opt-in pour les ressources d'une installation NWN légitime.
- Manifeste de corpus par hash, sans contenu propriétaire.
- Cas minimaux : sans HAK, plusieurs HAK, TLK personnalisé, grand module, DLG cyclique, NSS absent, ressources masquées, tileset personnalisé, ressources manquantes et module partiellement corrompu.
- Comparaisons avec plusieurs oracles; toute divergence est documentée au lieu d'être automatiquement attribuée à NWN Editor.
- Régressions visuelles pour modèles et zones avec tolérances versionnées.

### Sécurité et confidentialité

- Modules traités comme données non fiables.
- Bornes de tailles, profondeurs, offsets et allocations dans chaque parser.
- Canonicalisation des chemins et interdiction des sorties hors cache.
- Réseau désactivé par défaut pour l'analyse locale.
- Outils externes exécutés avec arguments validés et binaires vérifiés.
- Aucun contenu complet de script, dialogue ou asset dans les logs par défaut.

### Performance

- Mesures dès le Lot 0, objectifs chiffrés seulement après constitution du corpus.
- Métadonnées visibles avant la fin de l'indexation.
- Imports incrémentaux, pages de résultats, chargement paresseux et cache persistant.
- Budgets suivis : temps d'ouverture, temps d'indexation, latence de recherche, mémoire, temps de première image 2D/3D et taille du cache.

### Documentation et décisions

ADR requis avant engagement sur : Tauri, Babylon.js, SQLite, GFF/ERF, licences, résolution des ressources, format de cache, pipeline MDL, moteur de commandes, outils GPL et IA.

Chaque format possède une fiche : sources consultées, hypothèses, limites, fixtures, oracles, comportements observés et diagnostics associés.

### Graphe d'architecture du code

Le graphe de développement est distinct du graphe de ressources NWN livré au Lot 10. Il est généré
uniquement depuis le code, sans LLM ni réseau, et porte des preuves fichier/ligne. Avant une
modification transversale, utiliser `query --format paths`; après modification des sources indexées,
exécuter `generate`; avant livraison, exécuter `check`. Les artefacts JSON et Mermaid ne sont jamais
modifiés manuellement.

## 9. Definition of Done commune à chaque lot

Un lot est terminé seulement si :

- le scénario utilisateur du lot fonctionne dans le build release Windows ;
- les tests unitaires, intégration et UI pertinents sont verts ;
- les erreurs et cas dégradés sont visibles dans l'application ;
- aucune donnée inconnue n'est perdue silencieusement ;
- les migrations SQLite sont versionnées et testées ;
- les API sont typées, paginées si nécessaire et annulables pour les opérations longues ;
- les performances du corpus sont enregistrées ;
- la documentation, l'ADR éventuel, le changelog et les notices de licences sont à jour ;
- aucune ressource NWN propriétaire n'est présente dans le commit ou les artefacts CI ;
- le lot précédent continue de fonctionner.
- le graphe d'architecture a été régénéré après les changements indexés et son contrôle de fraîcheur réussit.

## 10. Backlog de démarrage — ordre des premières unités livrables

1. Initialiser Git, les documents de gouvernance et la structure minimale du workspace.
2. Décider et consigner la licence du projet.
3. Établir la matrice initiale des dépendances et licences, sans intégrer encore de bibliothèque NWN.
4. Générer le shell Tauri 2 + React 19 + TypeScript strict.
5. Mettre en place formatage, lint, tests, CI Windows et build release.
6. Créer la coquille sombre à quatre panneaux et l'écran d'accueil.
7. Définir le modèle d'erreur stable partagé Rust/TypeScript.
8. Ajouter la journalisation structurée, les identifiants de corrélation et l'écran Diagnostics.
9. Ajouter SQLite et la première migration.
10. Implémenter le registre de jobs, la progression et l'annulation.
11. Définir et tester le schéma versionné du fichier projet en lecture seule.
12. Ajouter les sélecteurs de chemins et leurs validations.
13. Implémenter le hash SHA-256 en flux comme premier job réel.
14. Persister et rouvrir le dernier projet.
15. Produire puis tester l'installateur Windows.
16. Implémenter le graphe d'architecture déterministe sur le premier parcours vertical réel, ajouter ses tests et son contrôle CI.
17. Effectuer la revue de sortie du Lot 0 avant toute lecture ERF/GFF.

Chaque unité doit rester assez petite pour être revue indépendamment et laisser le dépôt compilable.

## 11. Décisions à prendre avant l'implémentation

Ces choix ne bloquent pas la rédaction du plan mais bloquent certains commits :

1. licence du projet : MIT ou Apache-2.0 ;
2. gestionnaire du workspace JavaScript et version minimale de Node ;
3. versions minimales de Rust et Windows prises en charge ;
4. stratégie de génération/partage des types Tauri ;
5. bibliothèque SQLite et politique de migrations ;
6. emplacement et politique d'expiration du cache ;
7. bibliothèque de docking UI après prototype et audit de licence ;
8. bibliothèque NWN retenue après matrice licence/maintenance/couverture ;
9. corpus local autorisé et procédure de comparaison avec Aurora/NWN:EE ;
10. stratégie MDL temporaire puis native.

## 12. Prochaine action recommandée

Exécuter uniquement le **Lot 0**, en commençant par les décisions 1 à 6 et les unités 1 à 5 du backlog. Ne pas intégrer de parser NWN, de Babylon.js, de Monaco ou de React Flow avant que leur premier usage fonctionnel soit planifié dans le lot correspondant.
