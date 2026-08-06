# CONTEXTE GLOBAL — ÉDITEUR TIERS COMPLET POUR NEVERWINTER NIGHTS

**Nom de travail :** NWN Editor
**Version du contexte :** 0.1
**Date :** 2 août 2026
**Statut :** lancement du projet
**Cible initiale :** Neverwinter Nights: Enhanced Edition sur Windows 10/11
**But à long terme :** remplacer complètement l’Aurora Toolset pour la lecture, la création, la modification, la validation, la compilation et l’export de modules NWN.

---

## 1. Objet de ce document

Ce fichier est la référence principale du projet. Il doit être lu par Codex et par tout nouvel agent avant toute modification du code.

Le projet consiste à créer un éditeur moderne et indépendant capable d’ouvrir un module Neverwinter Nights, d’en comprendre toutes les ressources, de les visualiser de manière cohérente, puis progressivement de les modifier et de reconstruire un fichier `.mod` jouable.

La première phase est volontairement **strictement en lecture seule**. Elle doit permettre de charger un module existant et de visualiser :

- les zones et leurs cartes ;
- la scène 3D complète d’une zone ;
- les tuiles, portes, objets, créatures, déclencheurs, rencontres et points de passage ;
- les dialogues sous forme d’arbre ou de graphe ;
- les scripts NWScript et leurs références ;
- les quêtes et le journal ;
- les factions ;
- les créatures, objets, placeables, portes, marchands, sons et autres blueprints ;
- les fichiers 2DA et TLK ;
- les HAK utilisés par le module ;
- les modèles 3D, textures, animations, sons et autres ressources ;
- les liens entre toutes ces données.

Le logiciel doit ouvrir le module original sans jamais le modifier. Tout traitement doit être effectué dans un cache ou un projet de travail séparé.

---

## 2. Séparation avec les autres projets

Ce projet est exclusivement consacré à l’édition de Neverwinter Nights et au remplacement d’Aurora.

Il ne doit pas être confondu avec OpenRPG_Forge. Les deux projets sont indépendants. Une passerelle d’export vers OpenRPG_Forge pourra être développée ultérieurement, mais elle ne fait pas partie des premières phases et ne doit pas influencer les formats internes au point de ralentir le remplacement d’Aurora.

Le cœur du projet doit néanmoins être suffisamment propre pour permettre plus tard un export vers un format intermédiaire neutre.

---

## 3. Vision finale

L’application finale doit devenir un environnement complet de création de modules NWN comprenant :

1. un explorateur de ressources ;
2. un éditeur de zones 2D et 3D ;
3. un éditeur de dialogues visuel ;
4. un éditeur de quêtes ;
5. un éditeur de scripts NWScript avec compilation et diagnostics ;
6. des éditeurs spécialisés pour chaque type de blueprint ;
7. un éditeur de factions ;
8. un éditeur de journal ;
9. un gestionnaire de HAK, TLK, 2DA et ressources personnalisées ;
10. un système de recherche globale et de références croisées ;
11. un validateur de module ;
12. un système d’historique, d’annulation et de comparaison ;
13. un système de construction du `.mod` ;
14. un lancement direct dans NWN:EE pour test ;
15. une assistance IA contrôlée pour analyser, créer et modifier le contenu.

Le logiciel ne doit pas simplement être une interface autour d’Aurora. Il doit lire et écrire directement les formats de ressources NWN, ou utiliser temporairement des outils externes clairement encapsulés.

---

## 4. Priorité absolue : obtenir une lecture fiable avant toute écriture

Aucune fonction de modification réelle ne doit être développée avant que la chaîne de lecture soit suffisamment fiable.

La phase de lecture doit répondre à quatre questions :

1. Le logiciel trouve-t-il réellement toutes les ressources utilisées par le module ?
2. Le logiciel interprète-t-il correctement les données GFF et les chaînes localisées ?
3. Le logiciel reproduit-il visuellement une zone de manière suffisamment proche du jeu ?
4. Le logiciel sait-il expliquer les relations entre les zones, scripts, dialogues, quêtes et blueprints ?

La lecture générique d’un fichier GFF ne suffit pas. L’application doit construire une représentation métier compréhensible du module.

Exemple : elle ne doit pas seulement afficher un champ `Conversation = dlg_maire`. Elle doit permettre de cliquer sur `dlg_maire`, ouvrir le dialogue, montrer les scripts appelés par ce dialogue, les variables utilisées et les quêtes concernées.

---

## 5. Principes non négociables

### 5.1 Module original immuable

- Ne jamais modifier le `.mod` ouvert.
- Ne jamais écrire dans les HAK d’origine.
- Ne jamais modifier les fichiers de l’installation NWN.
- Calculer un hash du module et des dépendances au moment de l’import.
- Travailler dans un répertoire de cache séparé.
- Les futures écritures devront produire un nouveau fichier, jamais écraser l’original sans action explicite.

### 5.2 Aucun format deviné silencieusement

- Ne pas inventer la signification d’un champ GFF.
- Ne pas ignorer silencieusement un type ou un champ inconnu.
- Conserver les données inconnues dans la représentation brute.
- Afficher les avertissements sans empêcher l’ouverture du reste du module.
- Ajouter des tests à partir de ressources réelles autorisées ou de fixtures synthétiques.

### 5.3 Séparation des couches

La lecture des formats, la résolution des ressources, l’indexation, le modèle métier et l’interface doivent être indépendants.

L’interface ne doit jamais parser directement un fichier `.mod`, `.gff`, `.mdl` ou `.2da`.

### 5.4 Traçabilité

Toute donnée affichée doit pouvoir indiquer :

- son fichier source ;
- son ResRef ;
- son type ;
- le conteneur dont elle provient ;
- la priorité de résolution ayant conduit à cette version ;
- les autres versions masquées par la priorité des ressources ;
- son hash ;
- ses références entrantes et sortantes.

### 5.5 Performances

- Import incrémental fondé sur les hashes.
- Cache persistant.
- Chargement paresseux des modèles, textures et sons.
- Indexation en tâche de fond, sans bloquer l’interface.
- Annulation possible des longues opérations.
- Pas de conversion complète de tous les modèles au démarrage si elle n’est pas nécessaire.

### 5.6 Résilience

Un fichier corrompu ou inconnu ne doit pas empêcher l’ouverture du module entier. Le logiciel doit charger ce qu’il peut, consigner précisément l’erreur et continuer.

---

## 6. Limites juridiques et de distribution

Le logiciel doit fonctionner avec une installation locale légitime de Neverwinter Nights appartenant à l’utilisateur.

Règles à respecter :

- ne pas redistribuer les ressources du jeu ;
- ne pas inclure dans le dépôt les modèles, textures, sons, scripts ou modules propriétaires du jeu ;
- ne pas envoyer les ressources du jeu vers un service distant sans action explicite et information claire ;
- ne pas contourner une protection technique ;
- ne pas modifier l’exécutable de NWN ou celui d’Aurora ;
- ne pas intégrer de code GPL dans un projet sous licence permissive sans décision explicite sur la licence globale ;
- conserver un inventaire de toutes les dépendances et de leurs licences ;
- distinguer le code du logiciel, les fichiers de test synthétiques et les ressources chargées depuis l’installation de l’utilisateur.

Licence recommandée pour le projet tant qu’aucune décision contraire n’est prise : **Apache-2.0 ou MIT**.

Les projets GPL comme xoreos ou Borealis peuvent servir de références fonctionnelles et architecturales, mais leur code ne doit pas être copié dans une base permissive. Toute réutilisation directe devra être précédée d’une décision explicite de licence.

---

## 7. Technologies retenues

### 7.1 Application de bureau

- **Tauri 2** pour le conteneur Windows ;
- **React 19** ;
- **TypeScript strict** ;
- **Vite** ;
- **Rust** pour le cœur natif ;
- **SQLite** pour l’index local et le cache de métadonnées ;
- **Babylon.js** pour la visualisation 3D ;
- **Monaco Editor** pour les scripts et fichiers texte ;
- **React Flow** pour les graphes de dialogues, quêtes et dépendances ;
- **Zustand** pour l’état de l’interface ;
- **TanStack Query** pour les appels asynchrones et le cache de requêtes ;
- **Vitest** et Testing Library pour le frontend ;
- tests Rust unitaires et d’intégration pour le cœur.

### 7.2 Pourquoi cette architecture

Tauri et React permettent une interface moderne, dockable et adaptée à un éditeur complexe. Rust convient aux lectures binaires, à l’indexation, au multithreading, au cache et à la sécurité mémoire. Babylon.js fournit les caméras, la sélection, les gizmos, les matériaux, les animations, le picking et une base adaptée à un futur éditeur 3D.

Le frontend ne doit pas recevoir directement de gros buffers arbitraires via des messages JSON. Le cœur Rust doit produire des artefacts de cache optimisés, par exemple GLB, PNG ou données binaires dédiées, puis les exposer au frontend via un protocole local Tauri sécurisé.

### 7.3 Outils et bibliothèques NWN à évaluer

Priorité aux composants à licence compatible :

- `nwn-lib-rs` comme oracle externe éventuel pour ERF, GFF, TLK et 2DA, sans le lier au binaire
  principal tant que sa licence LGPL-3.0-or-later et son besoin actuel de Rust nightly ne font pas
  l'objet d'une décision explicite ; voir `docs/decisions/0002-nwn-library-and-license-policy.md` ;
- `neverwinter.nim` et ses exécutables comme outils de référence, de conversion ou de validation ;
- `Nasher` pour comparer l’extraction et la reconstruction des projets ;
- `nwn.py` comme oracle de comportement pour ResMan, KEY, GFF, TLK, 2DA et tilesets ;
- `nwnmdlcomp` pour la conversion temporaire des modèles MDL binaires vers ASCII si sa licence et son intégration sont validées ;
- le compilateur officiel open source `nwn_script_comp` pour la future compilation NWScript.

Ne pas multiplier les dépendances faisant la même chose. Définir des interfaces internes afin de pouvoir remplacer une implémentation externe sans modifier l’application.

---

## 8. Architecture générale

```text
┌───────────────────────────────────────────────────────────┐
│                     Interface React                       │
│ Explorateur | Inspecteurs | Graphes | Monaco | Vue 3D    │
└───────────────────────────┬───────────────────────────────┘
                            │ Commandes Tauri typées
┌───────────────────────────▼───────────────────────────────┐
│                    API d’application                       │
│ Projets | Recherche | Navigation | Diagnostics | Cache    │
└───────────────────────────┬───────────────────────────────┘
                            │
┌───────────────────────────▼───────────────────────────────┐
│                    Modèle métier NWN                       │
│ Module | Area | Dialogue | Blueprint | Quest | Script     │
└───────────────┬───────────────────────────────┬───────────┘
                │                               │
┌───────────────▼──────────────┐  ┌────────────▼────────────┐
│ Graphe de références         │  │ Index SQLite            │
│ ResRef, variables, scripts   │  │ recherche et cache      │
└───────────────┬──────────────┘  └────────────┬────────────┘
                │                               │
┌───────────────▼───────────────────────────────▼───────────┐
│                   Resource Manager                         │
│ Priorités, KEY/BIF, MOD, HAK, override, TLK, 2DA          │
└───────────────┬───────────────────────────────┬───────────┘
                │                               │
┌───────────────▼──────────────┐  ┌────────────▼────────────┐
│ Lecteurs de formats          │  │ Pipeline d’assets       │
│ ERF/GFF/KEY/BIF/SET/TLK/2DA  │  │ MDL/TGA/DDS/PLT/TXI     │
└──────────────────────────────┘  └─────────────────────────┘
```

---

## 9. Sous-systèmes du cœur

### 9.1 Project Manager

Responsabilités :

- créer et ouvrir un projet local ;
- enregistrer le chemin du module source ;
- enregistrer les chemins de l’installation et du dossier utilisateur NWN ;
- calculer les hashes ;
- détecter les changements externes ;
- gérer la version du schéma de cache ;
- lancer ou annuler l’import ;
- fournir un rapport d’import.

Un projet de lecture doit être un petit fichier texte, par exemple :

```json
{
  "project_version": 1,
  "name": "Module exemple",
  "module_path": "D:/NWN/modules/exemple.mod",
  "game_install_path": "C:/Program Files/Neverwinter Nights",
  "user_data_path": "C:/Users/User/Documents/Neverwinter Nights",
  "read_only": true
}
```

Ne jamais stocker de chemin spécifique en dur dans le code.

### 9.2 Resource Manager

C’est un composant critique. Il doit reproduire la logique de résolution des ressources de NWN de manière testable.

Il doit pouvoir charger et interroger :

- le module `.mod` ;
- les HAK déclarés par le module ;
- le TLK personnalisé déclaré ;
- les ressources du dossier utilisateur ;
- les ressources de l’installation ;
- les KEY/BIF du jeu ;
- les répertoires de développement ou override lorsqu’ils sont activés ;
- les ressources personnalisées ajoutées au projet dans les phases futures.

Pour chaque ressource, le Resource Manager doit conserver :

```text
ResRef
Type de ressource
Nom complet logique
Source sélectionnée
Sources masquées
Priorité
Offset et taille éventuels dans le conteneur
Hash
État de lecture
```

Il faut pouvoir demander :

```text
resolve("nwscript", NSS)
resolve("maire", UTC)
resolve("dlg_maire", DLG)
resolve("tcn01", SET)
list_all(DLG)
list_versions("appearance", 2DA)
```

La logique de priorité doit être isolée et couverte par des tests. Ne pas la répartir dans les écrans.

### 9.3 Lecteurs de conteneurs

Formats prioritaires :

- ERF ;
- MOD ;
- HAK ;
- KEY ;
- BIF ;
- éventuellement SAV et NWM ultérieurement.

La première livraison doit au minimum savoir lister et extraire virtuellement toutes les ressources du `.mod` et des HAK associés.

### 9.4 Lecteur GFF générique

Le lecteur GFF doit :

- préserver le type exact des champs ;
- préserver l’ordre lorsque cela est utile ;
- supporter les structures et listes imbriquées ;
- représenter correctement les chaînes localisées ;
- exposer un affichage brut pour le diagnostic ;
- permettre plus tard une sérialisation sans perte ;
- signaler les champs non reconnus sans les supprimer.

Le frontend doit disposer d’un inspecteur GFF générique utilisable pour tout fichier non encore couvert par un inspecteur spécialisé.

### 9.5 Adaptateurs métier

Chaque type majeur doit avoir un adaptateur transformant le GFF brut en objet métier typé.

Objets prioritaires :

- `ModuleInfo` pour `module.ifo` ;
- `AreaDefinition` pour `.are` ;
- `AreaInstances` pour `.git` ;
- `AreaToolsetData` pour `.gic` ;
- `DialogueGraph` pour `.dlg` ;
- `Journal` pour `.jrl` ;
- `FactionTable` pour `.fac` ;
- `CreatureBlueprint` pour `.utc` ;
- `ItemBlueprint` pour `.uti` ;
- `PlaceableBlueprint` pour `.utp` ;
- `DoorBlueprint` pour `.utd` ;
- `EncounterBlueprint` pour `.ute` ;
- `TriggerBlueprint` pour `.utt` ;
- `WaypointBlueprint` pour `.utw` ;
- `StoreBlueprint` pour `.utm` ;
- `SoundBlueprint` pour `.uts` ;
- autres formats `UT*` rencontrés.

Chaque adaptateur doit garder un lien vers la structure GFF brute afin de ne perdre aucune information.

### 9.6 Localized String Resolver

Les textes peuvent provenir :

- d’une chaîne directement intégrée ;
- d’un StrRef vers `dialog.tlk` ;
- d’un StrRef vers un TLK personnalisé ;
- de plusieurs langues.

Le resolver doit retourner :

```text
Texte résolu
Langue
Genre éventuel
StrRef
Source TLK
Texte embarqué éventuel
État : résolu / manquant / invalide
```

L’interface doit permettre d’afficher la valeur résolue et les données d’origine.

### 9.7 2DA Manager

Le gestionnaire 2DA doit :

- charger les tables selon la résolution de ressources ;
- conserver les colonnes et lignes ;
- gérer les valeurs `****` ;
- fournir des accès typés ;
- indiquer la source de chaque table ;
- supporter les tables remplacées par un HAK ;
- permettre de comparer plusieurs versions d’une table.

Tables particulièrement importantes pour la visualisation :

- apparences de créatures ;
- placeables ;
- portes ;
- portraits ;
- sons ;
- classes, races, compétences, dons et sorts ;
- matériaux et effets selon les besoins rencontrés.

Ne pas coder les index numériques dans l’interface. Toute conversion d’un identifiant vers un nom ou un modèle doit passer par le 2DA Manager.

---

## 10. Formats et contenus à prendre en charge

### 10.1 Module

Le logiciel doit lire `module.ifo` et afficher au minimum :

- nom et description ;
- version ;
- auteur si disponible ;
- zone de départ ;
- position et orientation de départ ;
- liste des zones ;
- HAK associés et leur ordre ;
- TLK personnalisé ;
- scripts d’événements du module ;
- paramètres de temps, météo et règles ;
- variables locales ;
- ressources manquantes.

### 10.2 Zones

Une zone est principalement représentée par :

- `.are` : propriétés et grille de tuiles ;
- `.git` : instances placées dans la zone ;
- `.gic` : informations propres au toolset.

L’application doit réunir ces trois fichiers dans une seule entité `Area`.

Elle doit afficher :

- nom, tag, ResRef ;
- taille de la grille ;
- tileset ;
- tuiles et orientations ;
- hauteur et paramètres de zone ;
- lumière, météo, brouillard, musique et sons ;
- scripts d’événements ;
- variables locales ;
- toutes les instances placées ;
- les transitions vers d’autres zones ;
- les ressources manquantes ou incohérentes.

### 10.3 Dialogues

Un dialogue doit être transformé en graphe orienté.

Afficher :

- nœuds PNJ ;
- réponses joueur ;
- liens ;
- liens partagés ;
- conditions de départ ;
- scripts d’action ;
- texte localisé ;
- commentaires ;
- animations et sons éventuels ;
- quêtes et variables référencées ;
- références circulaires ;
- nœuds inaccessibles.

Prévoir trois modes :

1. arbre simplifié ;
2. graphe complet ;
3. inspecteur GFF brut.

### 10.4 Scripts

Pour chaque `.nss` :

- afficher le code avec coloration NWScript ;
- indexer fonctions, constantes, includes et variables ;
- détecter les `#include` ;
- afficher les appels vers d’autres scripts lorsque détectables ;
- afficher les objets du module qui utilisent ce script ;
- afficher les diagnostics de syntaxe si un compilateur est disponible ;
- ne pas modifier le fichier en phase 1.

Pour les `.ncs` sans source `.nss` :

- afficher leur présence et leurs métadonnées ;
- permettre ultérieurement une vue désassemblée ;
- ne jamais prétendre avoir retrouvé le source original ;
- indiquer clairement que les noms locaux et commentaires peuvent être perdus.

### 10.5 Journal et quêtes

Le fichier `.jrl` doit être affiché par catégories et étapes.

Chaque quête doit montrer :

- nom ;
- tag ;
- priorité ;
- étapes et textes ;
- état final éventuel ;
- scripts et dialogues qui semblent mettre à jour la quête ;
- références de journal trouvées dans les scripts lorsque l’analyse le permet.

La relation entre JRL et scripts est souvent implicite. Les liens déduits par analyse statique doivent être marqués comme **déduits**, jamais comme certitudes.

### 10.6 Blueprints

Chaque blueprint doit avoir :

- une fiche lisible ;
- un affichage brut ;
- une prévisualisation visuelle lorsque possible ;
- la liste de ses scripts ;
- ses variables locales ;
- ses inventaires ou contenus ;
- ses références entrantes et sortantes ;
- sa provenance et les versions masquées.

### 10.7 Factions

Afficher :

- factions ;
- relations ;
- réputation ;
- créatures associées ;
- matrice visuelle des relations ;
- incohérences.

### 10.8 Ressources textuelles

Afficher avec syntaxe adaptée :

- NSS ;
- 2DA ;
- SET ;
- TXI ;
- MTR ;
- INI ou fichiers de configuration rencontrés ;
- fichiers texte personnalisés.

### 10.9 Ressources graphiques et audio

Prévoir des viewers spécialisés :

- MDL binaire et ASCII ;
- TGA ;
- DDS ;
- PLT ;
- portraits ;
- icônes ;
- sons et musiques lisibles ;
- animations de modèles.

Une ressource non visualisable doit au minimum être listée, identifiable et exportable vers le cache de diagnostic.

---

## 11. Pipeline de visualisation 3D

La visualisation 3D est le plus grand risque technique du projet. Elle doit être développée progressivement.

### 11.1 Objectif

Afficher une zone de module avec :

- toutes les tuiles correctement positionnées et orientées ;
- les portes ;
- les placeables ;
- les créatures ;
- les effets visuels simples lorsque raisonnable ;
- les lumières principales ;
- les déclencheurs et rencontres sous forme de volumes techniques ;
- les waypoints et points de son ;
- les limites walkmesh ;
- une caméra libre et une caméra proche de celle d’Aurora.

### 11.2 Étapes du rendu

1. Lire l’ARE et construire la grille de tuiles.
2. Résoudre le fichier `.set` du tileset.
3. Résoudre pour chaque tuile le modèle MDL correspondant.
4. Convertir ou décoder le MDL vers une représentation interne.
5. Résoudre les textures et matériaux.
6. Générer un GLB mis en cache ou transmettre une scène optimisée à Babylon.js.
7. Appliquer la position, l’orientation et les variantes de tuiles.
8. Lire le GIT.
9. Résoudre chaque blueprint placé.
10. Résoudre son apparence et son modèle via les 2DA.
11. Placer les entités dans la scène.
12. Ajouter les overlays techniques.

### 11.3 Modèles MDL

Le système doit progressivement supporter :

- MDL ASCII ;
- MDL binaire ;
- hiérarchie de nœuds ;
- trimesh ;
- skin ;
- dummy ;
- reference ;
- light ;
- emitter ;
- AABB et données walkmesh ;
- supermodels ;
- animations et keyframes ;
- matériaux, transparence et paramètres de textures.

Première stratégie acceptable :

- utiliser un convertisseur MDL binaire vers ASCII validé ;
- parser l’ASCII ;
- produire un GLB de cache ;
- comparer le résultat avec NWN Explorer, Aurora et le jeu.

Stratégie finale : lecteur natif des modèles binaires afin de réduire les dépendances et accélérer le chargement.

### 11.4 Textures

Formats à prendre en charge :

- TGA ;
- DDS ;
- PLT ;
- informations TXI ;
- matériaux MTR selon les ressources EE.

Le cache peut convertir les formats vers PNG, KTX2 ou textures directement utilisables par Babylon.js. Le logiciel doit conserver la référence vers le fichier original.

### 11.5 Walkmesh

Afficher au minimum :

- surfaces marchables ;
- surfaces bloquées ;
- portes et connexions ;
- volumes AABB ;
- superposition activable dans la scène.

La phase 1 ne doit pas tenter de recalculer un walkmesh. Elle doit seulement lire et visualiser les données existantes.

### 11.6 Sélection dans la scène

Un clic sur un objet doit :

- le sélectionner ;
- surligner son modèle ou son volume ;
- ouvrir son inspecteur ;
- afficher son ResRef, tag, position, orientation et blueprint ;
- proposer la navigation vers le fichier source ;
- afficher les scripts et références associés.

### 11.7 Mode dégradé

Si un modèle est absent ou non supporté :

- afficher un marqueur technique clair ;
- conserver la position et le type d’objet ;
- afficher l’erreur dans l’inspecteur ;
- ne pas faire échouer toute la zone.

---

## 12. Carte 2D des zones

Avant la fidélité 3D complète, fournir une vue 2D fiable.

La carte 2D doit afficher :

- grille des tuiles ;
- orientation des tuiles ;
- miniature lorsqu’elle peut être générée ;
- portes ;
- placeables ;
- créatures ;
- rencontres ;
- triggers ;
- waypoints ;
- sons ;
- transitions ;
- filtres par catégorie ;
- recherche et centrage sur un objet.

La vue 2D constitue le premier outil de navigation spatiale et ne doit pas dépendre du fonctionnement complet du renderer 3D.

---

## 13. Graphe global du module

Construire un graphe de références indépendant de l’interface.

Types de nœuds :

- ressource ;
- zone ;
- instance ;
- blueprint ;
- dialogue ;
- nœud de dialogue ;
- script ;
- quête ;
- étape de quête ;
- variable ;
- faction ;
- entrée 2DA ;
- chaîne TLK ;
- modèle ;
- texture ;
- son.

Types de liens :

- utilise ;
- contient ;
- instancie ;
- déclenche ;
- appelle ;
- inclut ;
- pointe vers ;
- dépend de ;
- résout vers ;
- masque ;
- définit ;
- lit une variable ;
- écrit une variable ;
- met à jour une quête ;
- transition vers une zone.

Chaque lien doit porter un niveau de confiance :

- certain : référence explicite dans une structure ;
- probable : détecté statiquement dans un script ;
- possible : correspondance heuristique ou IA.

L’utilisateur doit pouvoir répondre à des questions comme :

- Où ce dialogue est-il utilisé ?
- Quels objets utilisent ce script ?
- Quels scripts modifient cette variable ?
- Quelles zones pointent vers cette zone ?
- Cette créature est-elle réellement placée dans le module ?
- Cette ressource est-elle orpheline ?
- Quelle version de ce 2DA est réellement utilisée ?

---

## 14. Index SQLite

Le SQLite ne remplace pas les fichiers NWN. Il sert uniquement d’index et de cache.

Tables conceptuelles :

```text
projects
source_containers
resources
resource_versions
localized_strings
areas
area_tiles
area_instances
blueprints
scripts
script_symbols
dialogues
dialogue_nodes
journal_categories
journal_entries
references
diagnostics
asset_cache
import_jobs
```

Conserver dans l’index :

- IDs internes stables ;
- ResRef et type ;
- hashes ;
- chemins et conteneurs ;
- métadonnées recherchables ;
- références ;
- diagnostics ;
- état du cache.

Ne pas stocker inutilement les gros blobs du jeu dans SQLite.

---

## 15. Interface utilisateur cible

### 15.1 Disposition générale

```text
┌────────────────────────────────────────────────────────────────────┐
│ Menu | Projet | Recherche | Diagnostics | Paramètres              │
├───────────────┬───────────────────────────────────┬────────────────┤
│ Explorateur   │ Zone de travail                   │ Inspecteur     │
│ du module     │ 3D / 2D / Dialogue / Script      │ propriétés     │
│               │                                   │ références     │
├───────────────┴───────────────────────────────────┴────────────────┤
│ Sortie | Import | Compilation future | Erreurs | Journal          │
└────────────────────────────────────────────────────────────────────┘
```

### 15.2 Explorateur

Arborescence principale :

```text
Module
├── Informations
├── Zones
├── Dialogues
├── Scripts sources
├── Scripts compilés
├── Quêtes et journal
├── Factions
├── Créatures
├── Objets
├── Placeables
├── Portes
├── Rencontres
├── Triggers
├── Waypoints
├── Marchands
├── Sons
├── Modèles
├── Textures
├── 2DA
├── TLK
├── HAK
├── Ressources inconnues
└── Diagnostics
```

Fonctions :

- filtre instantané ;
- recherche par ResRef, tag, nom ou contenu ;
- regroupement par type, dossier logique ou origine ;
- badges d’erreur et d’avertissement ;
- menu permettant de copier le ResRef et le chemin logique ;
- navigation arrière/avant.

### 15.3 Onglets et docking

- Plusieurs ressources ouvertes simultanément.
- Onglets épinglables.
- Split horizontal et vertical.
- Sauvegarde de la disposition de travail.
- Historique de navigation.
- Ne pas développer un moteur de docking maison si une bibliothèque stable et compatible existe.

### 15.4 Inspecteur

L’inspecteur doit comporter :

- vue lisible ;
- propriétés techniques ;
- variables locales ;
- scripts ;
- références entrantes ;
- références sortantes ;
- source et priorité ;
- GFF brut ;
- diagnostics.

---

## 16. Phase 1 — lecture seule complète

La phase 1 est divisée en lots. Chaque lot doit être utilisable avant de passer au suivant.

### Lot 0 — socle du projet

Livrables :

- monorepo ou workspace propre ;
- application Tauri qui démarre ;
- React et TypeScript strict ;
- commandes Tauri typées ;
- journalisation ;
- gestion des erreurs ;
- SQLite avec migrations ;
- système de jobs annulables ;
- tests et CI ;
- documentation de développement ;
- écran d’accueil.

Critère de sortie : l’application s’installe et démarre sur Windows sans environnement de développement.

### Lot 1 — détection de NWN et ouverture d’un module

Livrables :

- sélection de l’installation NWN ;
- sélection du dossier utilisateur ;
- sélection d’un `.mod` ;
- détection et lecture de `module.ifo` ;
- liste des HAK et du TLK requis ;
- rapport des dépendances présentes ou absentes ;
- création du projet local en lecture seule ;
- hash et copie éventuelle de sécurité des métadonnées, pas des ressources propriétaires.

Critère de sortie : ouvrir plusieurs modules et afficher correctement leurs informations générales.

### Lot 2 — Resource Manager et explorateur brut

Livrables :

- lecture ERF/MOD/HAK ;
- lecture KEY/BIF ;
- résolution de ressources ;
- liste de toutes les ressources disponibles ;
- affichage de la source sélectionnée et des versions masquées ;
- extraction à la demande dans le cache ;
- inspecteur hexadécimal ou binaire minimal pour les formats inconnus ;
- diagnostic des ressources manquantes.

Critère de sortie : rechercher un ResRef et expliquer précisément d’où vient la ressource réellement utilisée.

### Lot 3 — GFF, TLK et 2DA

Livrables :

- inspecteur GFF générique ;
- résolution des LocalizedStrings ;
- viewer TLK ;
- viewer 2DA ;
- comparaison de versions 2DA ;
- adaptateurs métier initiaux pour IFO, ARE, GIT et blueprints principaux.

Critère de sortie : ouvrir n’importe quel GFF du module sans perte visible et afficher les noms localisés.

### Lot 4 — scripts en lecture

Livrables :

- liste NSS et NCS ;
- Monaco avec coloration NWScript ;
- index des includes et symboles ;
- recherche plein texte ;
- liste des objets utilisant chaque script ;
- diagnostics du compilateur en mode vérification lorsque possible ;
- vue technique des NCS sans promesse de source reconstitué.

Critère de sortie : partir d’un objet dans une zone, ouvrir son script, puis revenir à tous les objets utilisant ce script.

### Lot 5 — dialogues

Livrables :

- parser métier DLG ;
- arbre de conversation ;
- graphe ;
- affichage des textes localisés ;
- conditions et actions ;
- liens vers les scripts ;
- détection des nœuds inaccessibles et liens cassés ;
- navigation depuis une créature vers son dialogue.

Critère de sortie : visualiser sans erreur des dialogues simples, ramifiés, cycliques et utilisant des liens partagés.

### Lot 6 — journal, quêtes et factions

Livrables :

- viewer JRL ;
- étapes de quête ;
- liens vers dialogues et scripts lorsque certains ;
- matrice des factions ;
- recherche globale par nom, tag, variable et StrRef.

Critère de sortie : reconstituer la structure narrative d’un module sans ouvrir Aurora.

### Lot 7 — carte 2D

Livrables :

- lecture de la grille ARE ;
- lecture du SET ;
- représentation des tuiles et orientations ;
- affichage des instances GIT ;
- filtres ;
- sélection et inspecteur ;
- transitions entre zones ;
- minimap existante ou rendu top-down simplifié.

Critère de sortie : retrouver dans la carte 2D tous les objets listés dans le GIT avec leurs coordonnées.

### Lot 8 — viewer de modèles et textures

Livrables :

- viewer MDL ;
- hiérarchie de nœuds ;
- textures TGA/DDS/PLT ;
- matériaux de base ;
- animations principales ;
- supermodels ;
- cache GLB ;
- aperçu des blueprints.

Critère de sortie : visualiser des tuiles, créatures, portes et placeables représentatifs, avec rapport clair pour les fonctions non supportées.

### Lot 9 — vue 3D des zones

Livrables :

- assemblage des tuiles ;
- placement des instances ;
- caméra libre ;
- sélection ;
- overlays techniques ;
- chargement progressif ;
- statistiques de rendu ;
- mode dégradé ;
- comparaison par captures avec Aurora ou le jeu.

Critère de sortie : charger plusieurs zones de tailles différentes et reconnaître visuellement leur structure, leurs objets et leurs transitions.

### Lot 10 — graphe global et diagnostic de module

Livrables :

- graphe de références ;
- ressources orphelines ;
- références manquantes ;
- scripts manquants ;
- transitions invalides ;
- blueprints absents ;
- TLK/StrRef invalides ;
- rapport exportable en JSON et HTML.

Critère de sortie : fournir un rapport cohérent sur l’état d’un module sans le modifier.

---

## 17. Phase 2 — édition contrôlée

Cette phase ne commence qu’après validation de la phase 1.

**État au 4 août 2026 : phase engagée explicitement.** L'espace transactionnel, les writers
GFF/ERF, l'édition structurée/NSS, la compilation NCS, les transformations de zone et les sorties
MOD/development sont intégrés. Les sources NWN restent immuables ; seuls l'overlay et les nouvelles
sorties sont écrits. Les structures DLG, JRL et FAC peuvent être créées/supprimées par commandes
réversibles ; les relations indexées sont nettoyées et réindexées automatiquement. Les listes
prioritaires UTC/UTI/UTS/UTE disposent du même contrat transactionnel, sans masquer le GFF brut.

Fonctions prévues :

- création d’un espace de travail modifiable ;
- copie logique des ressources modifiées seulement ;
- système de commandes avec undo/redo ;
- journal des modifications ;
- édition des propriétés générales ;
- éditeurs de blueprints ;
- édition des dialogues ;
- édition du journal et des factions ;
- édition NWScript ;
- compilation NSS vers NCS ;
- déplacement et ajout d’instances dans les zones ;
- modification de tuiles ;
- validation avant export ;
- construction d’un nouveau `.mod` ;
- test direct dans NWN:EE.

Toute modification doit être représentée comme une commande réversible, par exemple :

```json
{
  "command": "set_field",
  "resource": "maire.utc",
  "path": "Conversation",
  "before": "dlg_maire_old",
  "after": "dlg_maire"
}
```

L’édition ne doit pas manipuler directement le fichier original sur disque.

---

## 18. Phase 3 — remplacement complet d’Aurora

**État au 4 août 2026 : fondations en cours.** La création d'un MOD synthétique, des zones
ARE/GIT/GIC, les palettes, la validation géométrique, les HAK, les exports reproductibles, le scan
Aurora en lecture seule et les propositions IA prévisualisées disposent de contrats testés. Cette
phase n'est pas encore acceptée comme remplacement complet.

Fonctions finales :

- création d’un module vide ;
- création et suppression de zones ;
- création de tilesets personnalisés si techniquement réaliste ;
- palettes et bibliothèques de blueprints ;
- placement 3D avec gizmos ;
- peinture de tuiles ;
- édition de portes et transitions ;
- édition de triggers et encounters ;
- gestion avancée des walkmeshes ;
- édition complète des créatures et inventaires ;
- assistants de quêtes ;
- gestion des HAK, TLK, 2DA et contenus personnalisés ;
- gestion de projet Git ;
- builds reproductibles ;
- profils de test ;
- import depuis Aurora et synchronisation temporaire ;
- génération de documentation du module ;
- outils d’analyse et de refactoring ;
- assistance IA.

Le logiciel pourra être considéré comme un remplacement complet lorsque l’utilisateur peut créer, maintenir, compiler et tester un module complexe sans ouvrir Aurora.

---

## 19. IA dans le projet

L’IA n’est pas prioritaire dans les premiers lots. Elle ne doit pas masquer une lecture incomplète des formats.

Une fois le graphe de ressources fiable, l’IA pourra :

- résumer un module ;
- expliquer le fonctionnement d’une quête ;
- retrouver où une variable est utilisée ;
- proposer un dialogue ;
- créer un plan de quête ;
- écrire ou corriger du NWScript ;
- proposer des opérations structurées ;
- analyser les erreurs de validation ;
- traduire des textes ;
- documenter le module.

L’IA ne doit jamais écrire directement dans un GFF. Elle doit produire des opérations validées par le moteur de commandes.

Exemple futur :

```json
{
  "operations": [
    {
      "type": "create_quest",
      "id": "druide_captif",
      "title": "Le druide captif"
    },
    {
      "type": "create_dialogue",
      "resref": "dlg_druide"
    },
    {
      "type": "place_creature",
      "area": "foret01",
      "blueprint": "druide.utc",
      "position": [32.5, 18.2, 0.0]
    }
  ]
}
```

Chaque opération devra être contrôlée, prévisualisée, annulable et ajoutée à l’historique.

---

## 20. Structure recommandée du dépôt

```text
nwn-editor/
├── README.md
├── CONTEXT.md
├── LICENSE
├── THIRD_PARTY_NOTICES.md
├── docs/
│   ├── architecture/
│   ├── formats/
│   ├── decisions/
│   ├── testing/
│   └── screenshots/
├── apps/
│   └── desktop/
│       ├── src/                 # React/TypeScript
│       ├── src-tauri/           # entrée Tauri
│       └── tests/
├── crates/
│   ├── aurora-core/             # types communs et erreurs
│   ├── aurora-project/          # projets et cache
│   ├── aurora-resman/           # résolution ressources
│   ├── aurora-erf/              # adaptateur ERF/MOD/HAK
│   ├── aurora-keybif/           # KEY/BIF
│   ├── aurora-gff/              # GFF brut et adaptateurs
│   ├── aurora-tlk/
│   ├── aurora-2da/
│   ├── aurora-tileset/          # SET et assemblage tuiles
│   ├── aurora-dialogue/
│   ├── aurora-nwscript/
│   ├── aurora-model/            # MDL et animations
│   ├── aurora-texture/          # TGA/DDS/PLT/TXI
│   ├── aurora-area/             # ARE/GIT/GIC
│   ├── aurora-index/            # SQLite et recherches
│   ├── aurora-graph/            # références croisées
│   ├── aurora-validation/
│   └── aurora-cache/
├── fixtures/
│   ├── synthetic/               # fixtures créées par le projet
│   └── manifests/               # références de tests externes non distribuées
├── tools/
│   ├── format-dump/
│   ├── compare-oracles/
│   └── screenshot-regression/
└── .github/
    └── workflows/
```

Le terme `aurora` dans les noms internes désigne les formats Aurora, pas une dépendance à l’éditeur officiel.

---

## 21. Contrats d’API internes

Toutes les commandes Tauri doivent être typées et retourner des erreurs structurées.

Exemples :

```text
open_project(project_file)
create_readonly_project(module_path, install_path, user_path)
scan_project(project_id)
get_import_status(job_id)
cancel_job(job_id)
list_resources(project_id, filter, page)
get_resource(project_id, resource_id)
get_resource_versions(project_id, resref, type)
get_gff_tree(resource_id)
get_area(area_id)
get_area_scene_manifest(area_id)
get_dialogue_graph(dialogue_id)
get_script(script_id)
find_references(resource_id)
search_project(query, filters)
get_diagnostics(scope)
```

Ne pas renvoyer des structures GFF géantes si seule une liste est demandée. Prévoir pagination et chargement paresseux.

---

## 22. Modèle d’erreur

Toutes les erreurs doivent contenir :

```text
code stable
message utilisateur
message technique
source éventuelle
ressource concernée
étape d’import
cause imbriquée
niveau : info / warning / error / fatal
suggestion éventuelle
```

Exemples de codes :

```text
NWN_INSTALL_NOT_FOUND
MODULE_ERF_INVALID
MODULE_IFO_MISSING
HAK_NOT_FOUND
CUSTOM_TLK_NOT_FOUND
RESOURCE_SHADOWED
GFF_FIELD_UNSUPPORTED
LOCALIZED_STRING_UNRESOLVED
MODEL_NODE_UNSUPPORTED
TEXTURE_FORMAT_UNSUPPORTED
SCRIPT_INCLUDE_MISSING
AREA_TILESET_MISSING
AREA_MODEL_MISSING
REFERENCE_BROKEN
```

---

## 23. Journalisation et diagnostics

- Utiliser une journalisation structurée côté Rust.
- Un identifiant de corrélation par import.
- Logs rotatifs.
- Éviter d’enregistrer le contenu complet des scripts ou dialogues par défaut.
- Écran de diagnostics avec filtres.
- Bouton pour créer un paquet de diagnostic excluant les ressources propriétaires.

Le paquet de diagnostic peut contenir :

- versions du logiciel ;
- système ;
- chemins anonymisés ;
- hashes ;
- logs ;
- liste des formats rencontrés ;
- erreurs ;
- statistiques ;
- aucune ressource du jeu sans consentement explicite.

---

## 24. Tests

### 24.1 Tests unitaires

Chaque parser doit avoir des tests pour :

- valeurs normales ;
- champs absents ;
- valeurs limites ;
- données tronquées ;
- encodage ;
- ordre des champs ;
- types inconnus ;
- round-trip futur lorsque l’écriture sera développée.

### 24.2 Fixtures

Créer autant que possible des fixtures synthétiques minimales, sans ressources propriétaires.

Pour les tests nécessitant une installation NWN :

- utiliser des tests locaux opt-in ;
- référencer les fichiers par hash ;
- ne pas les inclure dans le dépôt ;
- permettre à l’utilisateur de sélectionner son installation.

### 24.3 Oracles

Comparer les résultats avec plusieurs outils :

- Aurora ;
- le jeu NWN:EE ;
- Nasher ;
- neverwinter.nim ;
- nwn.py ;
- NWN Explorer ;
- éventuellement xoreos pour le rendu, sans copier son code.

Un outil externe ne doit pas être considéré comme infaillible. Les différences doivent être consignées.

### 24.4 Tests visuels

Pour le rendu :

- captures de modèles sous plusieurs angles ;
- captures top-down de zones ;
- comparaison d’images avec tolérance ;
- scènes synthétiques ;
- tests sur rotation et position ;
- tests des textures transparentes ;
- tests de supermodels et animations.

### 24.5 Modules de validation

Prévoir un ensemble local de modules de test couvrant :

- petit module sans HAK ;
- module avec plusieurs HAK ;
- module avec TLK personnalisé ;
- grand module ;
- dialogues cycliques ;
- scripts source absents ;
- ressources masquées ;
- tileset personnalisé ;
- modèles et textures personnalisés ;
- ressources manquantes ;
- module partiellement corrompu.

---

## 25. Performance cible

Objectifs initiaux sur un PC Windows moderne :

- ouverture de l’application en moins de quelques secondes ;
- affichage des métadonnées du module avant la fin de l’indexation complète ;
- recherche dans l’index quasi instantanée ;
- ouverture d’un dialogue standard sans délai perceptible après indexation ;
- affichage progressif d’une zone ;
- interface fluide pendant l’import ;
- mémoire maîtrisée, sans charger tous les modèles simultanément.

Ne pas promettre de chiffre strict avant d’avoir constitué un corpus de modules de référence. Mesurer systématiquement.

---

## 26. Sécurité

- Valider tous les chemins.
- Empêcher la traversée de répertoires lors de l’extraction.
- Limiter les tailles et profondeurs de structures avant allocation.
- Considérer les modules comme des fichiers non fiables.
- Ne jamais exécuter de script NWScript sur le poste.
- Le compilateur peut analyser ou compiler, mais pas exécuter arbitrairement le contenu.
- Isoler les outils externes et limiter leurs arguments.
- Vérifier les hashes des binaires sidecar distribués.
- Désactiver les accès réseau par défaut pour l’analyse locale.

---

## 27. Décisions d’architecture à consigner

Créer un ADR dans `docs/decisions/` pour chaque décision majeure :

- choix de Tauri ;
- choix de Babylon.js ;
- choix de SQLite ;
- bibliothèque GFF/ERF ;
- stratégie MDL ;
- stratégie de licence ;
- logique de résolution des ressources ;
- format du cache ;
- modèle de commandes futur ;
- politique sur les outils GPL ;
- stratégie d’IA.

Un ADR doit contenir : contexte, options, décision, conséquences et date.

---

## 28. Règles de travail pour Codex

Codex doit respecter les règles suivantes :

1. Lire entièrement ce fichier avant une modification importante.
2. Examiner le code existant avant de créer une nouvelle architecture.
3. Ne pas ajouter une dépendance sans vérifier sa licence, sa maintenance et son utilité.
4. Ne jamais copier du code GPL dans le projet sans décision explicite.
5. Ne jamais intégrer de ressource propriétaire dans le dépôt.
6. Ne jamais activer l’écriture dans un module pendant la phase 1.
7. Écrire des tests pour chaque parser ou comportement critique.
8. Préférer les types explicites aux dictionnaires génériques dans le modèle métier.
9. Conserver l’accès au GFF brut pour les données non encore modélisées.
10. Ne pas cacher les erreurs de parsing.
11. Ne pas supposer qu’un module contient les sources `.nss`.
12. Ne pas supposer qu’une ressource provient du `.mod` ; elle peut venir d’un HAK ou du jeu de base.
13. Ne pas supposer qu’un index 2DA est stable entre deux installations ou deux HAK.
14. Ne pas coder les chemins Windows en dur.
15. Ne pas charger tous les blobs en mémoire.
16. Ne pas utiliser l’IA pour compenser un parser incomplet.
17. Documenter les limites connues dans l’interface et dans le code.
18. Maintenir `CHANGELOG.md`, les migrations SQLite et les ADR.
19. Garder le projet compilable après chaque lot.
20. Produire des commits petits, cohérents et testés.

---

## 29. Première tâche à donner à Codex

Créer le socle du projet sans commencer le parsing complexe.

### Résultat attendu

- initialiser le dépôt et le workspace ;
- créer l’application Tauri 2 + React 19 + TypeScript strict ;
- ajouter une interface sombre moderne d’éditeur ;
- créer les panneaux Explorateur, Zone de travail, Inspecteur et Diagnostics ;
- créer le cœur Rust avec modules `project`, `jobs`, `errors`, `logging` et `database` ;
- intégrer SQLite avec migrations ;
- créer le modèle d’erreur structuré ;
- permettre de sélectionner un fichier `.mod` et des dossiers NWN ;
- enregistrer un projet local en lecture seule ;
- calculer SHA-256 du module dans un job annulable ;
- afficher la progression ;
- ne pas encore extraire ni modifier le module ;
- ajouter tests frontend et Rust ;
- ajouter CI Windows ;
- créer `README.md`, `CONTRIBUTING.md`, `THIRD_PARTY_NOTICES.md` et le premier ADR.

### Critères d’acceptation

- build propre ;
- application installable ;
- aucune ressource NWN dans le dépôt ;
- aucun chemin en dur ;
- erreurs visibles et compréhensibles ;
- annulation fonctionnelle du hash d’un gros fichier ;
- réouverture du dernier projet ;
- tests verts ;
- architecture prête pour le Resource Manager.

---

## 30. Deuxième tâche prévue

Implémenter une première lecture du `.mod` :

- lister les entrées de l’archive ERF ;
- trouver `module.ifo` ;
- parser son GFF ;
- afficher les propriétés principales ;
- détecter les HAK et le TLK ;
- afficher toutes les ressources du module dans l’explorateur ;
- conserver la lecture strictement en mémoire/cache ;
- comparer les résultats avec Nasher ou `nwn_erf` dans des tests d’intégration locaux.

Ne pas développer le renderer 3D avant que ce lot soit stable.

---

## 31. Définition de « lecture complète »

La phase 1 est terminée lorsque l’application peut, sur un ensemble représentatif de modules :

- ouvrir le module et toutes ses dépendances ;
- lister toutes les ressources ;
- expliquer leur résolution ;
- afficher tous les GFF ;
- afficher les textes TLK ;
- afficher les 2DA ;
- visualiser les scripts ;
- visualiser les dialogues ;
- visualiser journal et factions ;
- afficher les blueprints ;
- afficher une carte 2D de toutes les zones ;
- afficher une vue 3D exploitable des zones ;
- visualiser les principaux modèles, textures et animations ;
- naviguer entre les références ;
- produire un rapport des erreurs et éléments non supportés ;
- ne modifier aucun fichier source.

Une lecture partielle silencieuse n’est pas acceptable. Les limites doivent être mesurées et affichées.

---

## 32. Risques principaux

### Risque 1 — fidélité du Resource Manager

Une mauvaise priorité de ressources peut afficher le mauvais modèle, le mauvais 2DA ou le mauvais script.

Réponse : tests dédiés, affichage des versions masquées, comparaison avec le jeu.

### Risque 2 — complexité MDL

Les modèles NWN contiennent plusieurs types de nœuds, animations, supermodels, emitters et particularités historiques.

Réponse : progression par catégories, mode dégradé, fixtures, conversion temporaire, tests visuels.

### Risque 3 — tilesets personnalisés

Les HAK peuvent remplacer SET, modèles, textures et 2DA.

Réponse : aucune hypothèse sur les ressources de base, tout passe par le Resource Manager.

### Risque 4 — portée trop grande

Remplacer Aurora est un projet long.

Réponse : livraisons verticales et utilisables, lecture avant écriture, pas d’IA prématurée.

### Risque 5 — licences

Une réutilisation incontrôlée de code GPL peut imposer une licence globale non souhaitée.

Réponse : inventaire, ADR, vérification avant ajout et séparation claire des outils externes.

### Risque 6 — modules corrompus ou atypiques

Les modules communautaires peuvent contenir des données inhabituelles.

Réponse : parser défensif, diagnostics et récupération partielle.

---

## 33. Références techniques initiales

Ces références sont des points de départ. Vérifier leurs versions et licences avant intégration.

- Nasher : https://github.com/squattingmonk/nasher
- neverwinter.nim : https://github.com/niv/neverwinter.nim
- nwn.py : https://niv.github.io/nwn.py/
- nwn-lib-rs : https://docs.rs/nwn-lib-rs/latest/nwn_lib_rs/
- NWScript compiler : https://github.com/nwneetools/nwnsc
- NWN Explorer : https://github.com/virusman/nwnexplorer
- nwnmdlcomp : https://github.com/niv/nwn-tools
- xoreos : https://github.com/xoreos/xoreos
- Borealis NWN Model Viewer : https://github.com/varenx/borealis_nwn_model_viewer
- NeverBlender : https://github.com/Supermanu/NeverBlender
- Documentation historique des formats incluse dans neverwinter.nim : dossier `docs/`
- Documentation communautaire des formats et ressources : https://nwn.wiki/

---

## 34. Résumé opérationnel

Le projet doit commencer comme un **explorateur intelligent et visuel de modules NWN**, totalement en lecture seule.

Ordre obligatoire :

```text
Projet local
→ ouverture MOD
→ Resource Manager
→ GFF/TLK/2DA
→ modèle métier
→ scripts et dialogues
→ carte 2D
→ modèles et textures
→ vue 3D
→ graphe de références
→ validation
→ seulement ensuite édition et export
```

Le premier objectif n’est pas de créer rapidement une nouvelle quête. Le premier objectif est de prouver que le logiciel comprend réellement un module existant dans son ensemble.

À terme, ce socle permettra de remplacer complètement Aurora, avec une interface moderne, une meilleure navigation entre les ressources, une validation avancée et une assistance IA contrôlée.

---

## 35. Graphe d’architecture du code obligatoire

### 35.1 Mission

Le dépôt doit disposer d’un graphe d’architecture déterministe reliant le code, les interfaces,
les commandes Tauri, les services applicatifs, le modèle métier, les lecteurs de formats, les
accès SQLite, le cache et les tests.

Ce graphe sert à :

- retrouver rapidement les fichiers concernés par une évolution ;
- comprendre les dépendances sans charger tout le dépôt dans le contexte ;
- vérifier qu’une modification transversale couvre toutes les couches nécessaires ;
- détecter automatiquement un graphe périmé dans l’intégration continue ;
- fournir des preuves vérifiables issues du code.

Le **code est toujours la source de vérité**. Le graphe est un index généré : il ne doit jamais
inventer une relation, contenir de logique métier ni devenir une seconde architecture maintenue
manuellement.

### 35.2 Séparation avec le graphe de ressources NWN

Deux graphes différents existeront dans le projet et ne doivent pas être confondus :

1. le **graphe d’architecture du code**, décrit dans cette section, est un outil de développement
   statique, local et généré depuis le dépôt ;
2. le **graphe global d’un module NWN**, décrit à la section 13 et livré au Lot 10, est une fonction
   du produit construite à partir des ressources du module ouvert.

Ils peuvent employer des principes communs de provenance et de requêtes bornées, mais ils ont des
schémas, des données, des cycles de vie et des responsabilités séparés. Le graphe d’architecture ne
doit jamais indexer les ressources propriétaires chargées par l’utilisateur.

### 35.3 Livrables du Lot 0

Le Lot 0 doit créer :

```text
scripts/architecture_graph.py           # générateur et CLI locale
tests/test_architecture_graph.py         # tests unitaires du générateur
docs/architecture/graph.json             # index machine généré
docs/architecture/overview.mmd           # synthèse Mermaid générée
docs/architecture/README.md              # contrat, taxonomie et limites
AGENTS.md                                # règles de maintenance pour les agents
```

La CI Windows doit exécuter les tests du générateur puis le contrôle de fraîcheur.

### 35.4 Contraintes fondamentales

1. Le générateur est local, déterministe et reproductible.
2. Il n’utilise ni LLM, ni réseau, ni base de données, ni service externe.
3. Il privilégie la bibliothèque standard. Toute dépendance supplémentaire doit être justifiée,
   verrouillée et consignée.
4. Deux générations exécutées sur les mêmes sources produisent exactement les mêmes octets.
5. Chaque relation porte une preuve : chemin relatif et, lorsque possible, numéro de ligne.
6. Les nœuds, relations, preuves et clés JSON sont triés de façon stable.
7. Les chemins générés utilisent `/` et ne dépendent ni de la machine, ni de l’heure, ni du chemin
   absolu du dépôt.
8. Le graphe complet n’est jamais chargé dans le contexte d’un agent. Les agents interrogent un
   sous-graphe borné ou demandent seulement les chemins pertinents.
9. `graph.json` et `overview.mmd` ne sont jamais corrigés manuellement.
10. Une syntaxe inconnue ou ambiguë est ignorée proprement ou signalée, jamais transformée en
    relation supposée.
11. Aucun secret, fichier utilisateur, ressource NWN, cache, base SQLite ou artefact lourd ne doit
    être indexé.

### 35.5 Contrat de ligne de commande

Depuis la racine du dépôt :

```bash
python scripts/architecture_graph.py generate
python scripts/architecture_graph.py check
python scripts/architecture_graph.py stats
python scripts/architecture_graph.py query "<crate, composant, commande ou concept>"
python scripts/architecture_graph.py query "<recherche>" --format paths
python scripts/architecture_graph.py query "<recherche>" --format json
python scripts/architecture_graph.py query "<recherche>" --format mermaid
```

Options minimales :

- `--root` : racine analysée ;
- `--output` : emplacement du graphe ;
- `--depth` : profondeur maximale du sous-graphe ;
- `--max-nodes` : nombre maximal de nœuds retournés ;
- `--format text|paths|json|mermaid` pour `query`.

Comportement requis :

- `generate` écrit atomiquement les artefacts ;
- `check` reconstruit en mémoire et échoue si un artefact est absent ou périmé, sans modifier le
  dépôt ;
- `stats` affiche des comptes synthétiques par type de nœud et relation ;
- `query` recherche dans les identifiants, symboles, chemins et métadonnées, puis étend uniquement
  les voisins pertinents dans les limites demandées ;
- `--format paths` ne retourne que les fichiers à examiner, dédupliqués et triés ;
- une requête sans résultat retourne un résultat vide valide et un message clair.

### 35.6 Modèle de données

Le JSON possède un schéma versionné et stable :

```json
{
  "metadata": {
    "schema_version": 1,
    "source_digest": "sha256:..."
  },
  "nodes": [
    {
      "id": "tauri_command:open_project",
      "kind": "tauri_command",
      "name": "open_project",
      "path": "apps/desktop/src-tauri/src/commands/project.rs",
      "line": 18
    }
  ],
  "edges": [
    {
      "source": "ui:ProjectOpenDialog",
      "target": "tauri_command:open_project",
      "kind": "invokes_command",
      "evidence": {
        "path": "apps/desktop/src/features/project/ProjectOpenDialog.tsx",
        "line": 42
      }
    }
  ]
}
```

Les identifiants sont uniques, lisibles et stables. Le document ne contient pas le contenu des
fichiers, de variables d’environnement ou de chaînes pouvant porter des données utilisateur.

### 35.7 Portée initiale adaptée à NWN Editor

Répertoires à analyser lorsqu’ils existent :

```text
apps/desktop/src/
apps/desktop/src-tauri/src/
crates/
scripts/
tests/
fixtures/synthetic/
```

Les migrations SQLite sont indexées uniquement si elles représentent une relation architecturale
utile. Les manifestes du corpus local et toutes les ressources NWN sont exclus.

Exclusions minimales :

```text
.git/ node_modules/ vendor/ dist/ build/ target/ coverage/
.venv/ venv/ __pycache__/ .pytest_cache/ .mypy_cache/ .ruff_cache/
.cache/ .tmp/ tmp/ generated/
*.db *.sqlite* *.pem *.key *.p12 *.onnx *.bin *.model
.env .env.* ressources NWN caches d’assets jeux de données utilisateur
```

### 35.8 Taxonomie initiale

Nœuds possibles, seulement lorsqu’ils sont prouvables statiquement :

- fichier, module, package et crate ;
- écran, composant React et commande cliente ;
- commande Tauri, événement et point d’entrée ;
- service applicatif, job et cas d’usage ;
- modèle métier et DTO ;
- Resource Manager, lecteur de conteneur, parser et adaptateur métier ;
- repository SQLite, migration et cache ;
- diagnostic structuré ;
- test et fixture synthétique significative.

Relations initiales :

- `imports` et `defined_in` ;
- `invokes_command` et `handled_by` ;
- `uses_service`, `uses_reader` et `uses_repository` ;
- `parses_format`, `adapts_to` et `resolves_resource` ;
- `accesses_model`, `indexes`, `caches` et `emits_diagnostic` ;
- `tests` lorsqu’un import, un appel ou une fixture constitue une preuve réelle.

Ne pas déduire une couverture fonctionnelle à partir du seul nom d’un test. Les extracteurs doivent
commencer par un parcours vertical représentatif, par exemple : composant React → commande Tauri →
service de projet → repository SQLite, avec les tests correspondants.

### 35.9 Vue Mermaid

`overview.mmd` reste une synthèse lisible des couches principales, pas une copie de tous les nœuds.
Elle regroupe au minimum interface, API Tauri, services, modèle métier, index/cache, Resource Manager,
lecteurs de formats, pipeline d’assets et tests. Son ordre et ses identifiants sont déterministes.

### 35.10 Tests obligatoires

Le générateur est testé sur de petits dépôts factices temporaires, jamais en dépendant de la taille
du dépôt réel. Couvrir au minimum :

1. un parcours React → Tauri → service → repository/modèle → test ;
2. les imports TypeScript et Rust internes ;
3. les relations entre tests et code réellement référencé ;
4. l’absence de rapprochement entre deux commandes de noms voisins ;
5. l’exclusion des caches, builds, secrets, bases, ressources NWN et dépendances externes ;
6. la stabilité des identifiants, preuves et tris ;
7. deux générations strictement identiques ;
8. `check` réussi après génération puis en échec après modification d’une source ;
9. une requête pertinente et bornée dans les quatre formats ;
10. les chemins Windows contenant des espaces et leur normalisation ;
11. un fichier invalide ou une construction non prise en charge ;
12. une évolution explicite de `schema_version` lorsque le format change.

Ajouter un test de régression pour chaque faux positif ou erreur de résolution découvert.

### 35.11 Cycle de maintenance obligatoire

Pour toute modification transversale :

1. interroger d’abord le graphe avec `query`, de préférence en `--format paths` ;
2. examiner les preuves et les fichiers retournés ;
3. modifier le code sans adapter l’architecture au générateur ;
4. exécuter `generate` après toute modification d’une source indexée ou d’une règle du graphe ;
5. exécuter `check` avant de terminer ;
6. documenter honnêtement les relations non détectées et les heuristiques limitées.

La CI exécute :

```bash
python -m unittest tests/test_architecture_graph.py
python scripts/architecture_graph.py check
```

Elle échoue avec une instruction demandant `generate` si le graphe est absent ou périmé. Elle ne
régénère ni ne committe automatiquement les artefacts.

### 35.12 Critères d’acceptation

Le graphe d’architecture initial est accepté lorsque :

- sa génération ne nécessite aucun réseau ou service externe ;
- deux générations consécutives sont identiques octet pour octet ;
- `check` valide l’arbre courant et détecte une source modifiée ;
- les relations principales et les risques de faux positifs sont testés ;
- trois requêtes transversales représentatives retournent des sous-graphes utiles ;
- `--format paths` retourne une courte liste de fichiers exploitable ;
- la CI refuse un graphe périmé ;
- `AGENTS.md` explique quand interroger, générer et vérifier ;
- aucun secret, cache, environnement, base, modèle binaire ou contenu NWN n’est indexé ;
- les limites des extracteurs sont documentées.

---

## 36. Cycle de travail avec Aurora Toolset et le jeu

Ces règles décrivent le comportement opérationnel observé et devront être respectées lorsque les
phases d'écriture et de test en jeu seront autorisées. Elles ne lèvent pas la contrainte de lecture
seule de la Phase 1.

### 36.1 Modules de référence

Les archives de modules installées se trouvent normalement dans :

```text
E:\Jeux\Steam\steamapps\common\Neverwinter Nights\data\mod
```

Un module utilisé pour le développement est d'abord copié dans un espace de travail ignoré par Git.
L'archive installée reste immuable et aucun contenu propriétaire NWN n'est ajouté au dépôt.

### 36.2 Modification via le Toolset

Lorsqu'un module est ouvert, Aurora Toolset travaille sur des fichiers temporaires extraits. Une
modification directe de ces fichiers n'est persistée dans l'archive du module qu'après sauvegarde du
module dans le Toolset. À la prochaine ouverture, le Toolset recrée son espace temporaire depuis la
dernière archive sauvegardée.

Pour les scripts, le fichier source `.nss` ne suffit pas : les scripts modifiés doivent être
recompilés en `.ncs` avant la sauvegarde et la fermeture du module. Toute future automatisation
d'écriture devra donc modéliser explicitement la séquence modifier → compiler → vérifier →
sauvegarder, avec diagnostics et possibilité d'annulation.

### 36.3 Surcharge de développement en direct

Pour tester avec un module déjà chargé par le jeu ou `nwserver`, les fichiers modifiés peuvent être
placés dans le dossier NWN `development`. Cette couche prend le pas sur les ressources chargées par
le module et permet une itération en direct sans redémarrage. Elle doit rester une sortie de test
explicite et séparée de l'archive source, avec une liste précise des fichiers déployés et une action
de nettoyage sûre.

### 36.4 Édition structurée des objets de zone

Le Lot 19 conserve les structures observées dans les ressources GIT réelles : `TriggerList`
encode `Geometry` avec `PointX/PointY/PointZ`, `Encounter List` utilise `X/Y/Z` et sépare ses
`SpawnPointList`, tandis que portes et triggers portent `LinkedTo`, `LinkedToFlags` et
`LoadScreenID`. Les inventaires ne sont pas réécrits comme de simples références : les instances
de placeables et magasins incorporent une copie du blueprint UTI résolu, puis ajoutent uniquement
les champs de position, quantité, catégorie et stock infini nécessaires. Les champs inconnus restent
préservés. Après chaque commande et après undo/redo, la zone est relue depuis l'overlay afin que la
carte reflète les octets réellement stagés.

### 36.5 Walkmeshes WOK, PWK et DWK

Le Lot 20 dispose désormais d'un document géométrique typé (sommets, triangles, identifiants de
surface, variantes et hooks), d'une validation bornée incluant l'aire réelle, les faces dupliquées,
les arêtes non-manifold et la cohérence d'orientation. Les transformations Rust couvrent le
déplacement d'un sommet, l'affectation d'une surface, la découpe au centroïde, la suppression avec
compactage, l'extrusion selon la normale et la soudure spatiale à tolérance contrôlée. L'interface
appelle ces opérations via une commande Tauri typée et affiche immédiatement leurs diagnostics.

Le writer ne produit plus un MDL générique : il sérialise les grammaires autonomes observées dans
les ressources NWN. Un WOK porte `#NWmax WALKMESH ASCII`, `beginwalkmeshgeom`, les vingt matériaux,
des coordonnées de texture déterministes et un arbre AABB équilibré. Un PWK porte sa géométrie de
collision et ses points `use`; un DWK conserve les états `closed`, `open1`, `open2` ainsi que les
points de porte. Les anciens PWK `#MAXDOOR ASCII` ne contenant que des hooks sont également lus et
réécrits sans ajout artificiel de géométrie. Chaque résultat est relu par le parser interne avant
staging dans l'overlay ou déploiement dans `development`.

L'import d'un walkmesh existant conserve l'identifiant de surface de chaque face, les géométries
alternatives PWK/DWK et les nœuds dummy d'usage ou de porte. Son enregistrement reste présenté
comme un **remplacement complet** et exige une confirmation explicite. Une ressource absente est
créée par une commande annulable ; une ressource existante ou déjà stagée est transformée avec les
SHA-256 avant/après. Le MOD source n'est jamais ouvert en écriture.

Le harnais `scripts/validate_nwn_runtime.ps1` compare un lancement témoin et un lancement avec trois
overrides WOK/PWK/DWK réellement référencés par une copie de module. Dans l'environnement local du
4 août 2026, les deux lancements `nwserver.exe` s'arrêtent avec le même code `0xC0000005` avant
écoute ; le résultat est classé `INCONCLUSIVE_ENVIRONMENT`, jamais comme un échec du writer. Le
client Windows démarre avec les overrides et permet de poursuivre le contrôle manuel, tandis que
les empreintes confirment l'immuabilité du module source.

### 36.6 Contenus personnalisés HAK, TLK et 2DA

Le Lot 21 fournit des writers internes déterministes pour les tables `2DA V2.0` et `TLK V3.0`.
Les cellules, lignes, chaînes et références sonores sont modifiées par des opérations typées ; le
résultat est sérialisé, relu par le parser interne, puis stagé dans l'overlay avec les SHA-256 avant
et après. La suppression implicite d'un index TLK n'est pas proposée, car elle invaliderait les
StrRef existants ; les entrées peuvent être modifiées ou ajoutées en fin de table.

Le gestionnaire HAK/TLK valide les noms et l'ordre, refuse les doublons, transforme `Mod_HakList`
et `Mod_CustomTlk` dans `module.ifo`, puis relit le GFF écrit. Cette transformation est annulable et
préserve tous les champs IFO non concernés. Le writer HAK existant continue à empaqueter uniquement
les ressources explicitement modifiées, sans toucher aux HAK sources.

### 36.7 Builds reproductibles, profils et Git

Le Lot 22 persiste les profils de build dans le workspace séparé. Avant construction, la liste HAK
et le TLK attendus sont comparés aux déclarations réellement relues dans `module.ifo`; un profil
configuré pour bloquer les avertissements refuse tout écart. La vérification reproductible construit
deux MOD temporaires indépendants et compare leurs SHA-256. Un profil peut ensuite produire son MOD
dans un dossier choisi et déployer facultativement les ressources dans `development` selon les
règles de propriété et de nettoyage existantes.

L'intégration Git est volontairement locale et en lecture seule : elle résout la racine, la branche,
le commit courant et au plus 10 000 entrées de statut avec des arguments directs à l'exécutable
`git`, sans shell et sans effectuer automatiquement d'ajout, commit, checkout ou push.

Les profils de test `client` et `server` acceptent uniquement un `nwmain.exe` ou `nwserver.exe`
existant, un dossier de travail valide et une liste bornée d'arguments directs. Le processus est
lancé sans shell et ses sorties standard et d'erreur sont ajoutées à un journal du workspace. Le
logiciel ne prétend pas que le chargement moteur a réussi tant que ce processus n'a pas été contrôlé.

### 36.8 Synchronisation contrôlée avec Aurora Toolset

Le Lot 23 compare les fichiers reconnus du dossier temporaire Aurora, l’état résolu ou stagé du
workspace OpenNever et une baseline persistée. Chaque ressource est classée comme identique,
présente d’un seul côté, modifiée d’un seul côté ou conflictuelle. Une différence ne déclenche aucune
écriture tant que l’utilisateur n’a pas choisi importer depuis le Toolset ou envoyer vers le
Toolset. Les SHA-256 de la prévisualisation sont revérifiés immédiatement avant l’opération.

Un import devient une commande OpenNever transactionnelle et annulable. Un envoi crée d’abord une
sauvegarde récupérable sous `.opennever-backups`, y compris avant une suppression. Les liens
symboliques, chemins sortants, doublons de ResRef, profondeurs, quantités et tailles hors limites
sont refusés. L’envoi d’un NSS stagé est bloqué si son NCS n’est plus lié à sa compilation exacte.
La synchronisation ne remplace jamais la compilation puis la sauvegarde explicite dans Aurora.

### 36.9 Migrations et documentation utilisateur

Le Lot 24 porte le workspace au schéma 3. Les schémas 1 et 2 sont migrés uniquement après copie
byte-for-byte de `workspace.json` vers `workspace.json.v<version>.bak`; le chemin, les étapes et les
versions sont conservés dans l’historique visible. Un schéma futur est refusé sans réécriture. Les
baselines Toolset restent séparées des ressources NWN dans `aurora-sync/`.

Le moteur de comparaison est isolé dans `crates/aurora-edit/src/sync.rs`. Le guide utilisateur
décrit l’ouverture, l’édition, la compilation NSS→NCS, `development`, les builds, la synchronisation
Toolset et la récupération. Le guide des migrations et l’ADR 0008 fixent les garanties de
compatibilité et de sauvegarde.

### 36.10 Assistance IA contrôlée

Le Lot 25 contacte le réseau uniquement après une action explicite de l’utilisateur : tester,
lancer, poursuivre ou générer une proposition. Aucune case d’autorisation réseau supplémentaire
n’est nécessaire. L’utilisateur choisit explicitement un endpoint compatible et un modèle ; HTTP
est limité aux hôtes locaux, tandis que le réseau distant exige HTTPS. La clé éventuelle vit
uniquement en mémoire pendant l’appel. Les choix de métadonnées de module et de contenu de ressource
restent indépendants. Au plus huit GFF/NSS sont
sélectionnables, avec des limites strictes sur le prompt, chaque contexte, la réponse et le nombre
d’opérations. Aucun contenu transmis ou secret n’est journalisé.

Le modèle renvoie uniquement un `AiChangeSet`. Le cœur accepte les opérations `set_field` sur GFF
et `replace_text` sur NSS, refuse les autres commandes, rejoue les transformations sur les octets
courants et valide le parse NSS. Une prévisualisation sans mutation porte une empreinte SHA-256 ;
l’application exige une confirmation explicite de cette même empreinte, stage les octets dans
l’overlay et conserve chaque commande dans l’historique undo/redo. Une proposition JSON locale
emprunte le même pipeline sans réseau. L’IA n’écrit jamais directement dans un GFF ou un MOD.

### 36.11 Orchestrateur agentique et MCP

Les Lots 26 à 35 étendent l’assistant ponctuel sans relâcher sa frontière. `aurora-agent` porte les
politiques versionnées, six niveaux de sécurité, les budgets, la matrice par capacité, le
`ModuleBlueprint`, les runs, checkpoints et audits. Agent Studio rend réglables le contexte partagé,
les hôtes, les prix par token, les limites d’exécution, les chemins du compilateur/jeu/Toolset et
chaque fonction. Une clé fournisseur reste éphémère.

La boucle locale normalise Responses API, Chat Completions compatible et Ollama. Le modèle ne reçoit
que des outils enregistrés, implémentés et autorisés. Les écritures passent par `EditWorkspace`, un
checkpoint est persisté avant chaque mutation réversible et un appel interrompu n’est jamais rejoué
automatiquement. Le plan de module crée ARE/GIT/GIC, NSS, DLG, dépendances et manifeste IFO, puis
planifie NSS→NCS et la validation. Build, `development`, Toolset et lancement NWN conservent leurs
validations et autorisations supplémentaires.

Responses offre un stockage fournisseur explicite, désactivé par défaut. S’il est actif, chaque tour
poursuit le `previous_response_id` fourni par le serveur ; sinon, le run rejoue localement ses
entrées et tous les éléments de sortie. Les résultats typés restent des `function_call_output` liés
au `call_id` d’origine. Cette continuité est persistée, bornée par les budgets et expurgée à l’état
terminal selon la politique de rétention.

`opennever-mcp` est un adaptateur stdio : il partage politique et registre avec Tauri, expose l’état
en ressources MCP et ne fournit jamais un accès supérieur à l’interface. MCP n’est ni le cœur métier
ni un serveur réseau implicite. Les détails opérationnels sont dans `docs/AGENT_STUDIO.md` et la
décision dans l’ADR 0010. Il négocie les révisions `2025-11-25` et `2025-06-18`, exige la fin de
l’initialisation JSON-RPC et applique les mêmes limites de contexte, de portée, d’appels et de durée
que la politique locale.

### 36.12 Qualification de la construction agentique

Les six presets de sécurité sont maintenant fournis par le cœur Rust à Agent Studio ; changer de
niveau charge réellement ses limites, consentements et règles, tout en conservant la configuration
locale des chemins, portées et hôtes. Un test d’exhaustivité impose qu’une capacité du registre ait
un exécuteur Tauri. La politique sans rétention garde uniquement l’état actif nécessaire à la
reprise, masque les échanges fournisseur dans l’audit et expurge objectif, blueprint, arguments et
résultats quand le run devient terminal.

La qualification du 4 août 2026 couvre le workspace Rust complet, l’interface, le MCP, le graphe,
le build release Windows et une compilation réelle `ai_smoke.nss` → `ai_smoke.ncs` avec
`nwn_script_comp`. La copie de travail et le module installé `The Dark Ranger's Treasure.mod`
restent identiques (594 030 octets, SHA-256
`172C06CD5A2178AF46CC5C2828985EAB65FB5DD68898241333B391AB4FC26019`). Ollama local est
détecté mais son seul modèle dépasse encore le délai synthétique de 45 secondes ; ce contrôle
externe reste inconclusif et n’est pas transformé en réussite. Voir
`docs/validation/lot35-exit-review.md`.
