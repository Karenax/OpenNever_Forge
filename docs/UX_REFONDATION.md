# Refondation d’utilisabilité d’OpenNever Forge

## Décision produit

La qualification technique des Lots 0 à 40 a prouvé une base solide de lecture, d’édition
transactionnelle, de construction et de test NWN. Elle n’a pas prouvé que l’application pouvait être
utilisée quotidiennement à la place d’Aurora Toolset. Les écrans historiques exposaient trop souvent
la structure interne : cartes réduites, graphes illisibles, formulaires numériques, éditeurs entassés
et documentation centrée sur les garanties techniques.

À compter du 10 août 2026, la priorité est la tâche humaine complète. Une capacité backend n’est
considérée livrée que lorsque son atelier permet de trouver, comprendre, modifier, annuler, valider et
tester le résultat sur un volume représentatif.

## Contrat commun des ateliers

Chaque atelier spécialisé respecte le même modèle :

1. **Trouver** — recherche locale, filtres métier, compte et liste virtualisée ou paginée.
2. **Sélectionner** — sélection persistante, contexte visible et centrage automatique.
3. **Comprendre** — nom lisible, provenance, diagnostics et relations avant le brut.
4. **Modifier** — contrôles spécialisés, valeurs résolues et aperçu direct.
5. **Sécuriser** — overlay, état modifié, annuler/rétablir et confirmation des suppressions.
6. **Valider** — diagnostics ciblés et action suivante explicite.
7. **Tester** — build ou `development`, puis preuve moteur quand la ressource le nécessite.

Le panneau de droite reste un inspecteur contextuel. Il ne peut plus contenir l’éditeur principal
d’une ressource complexe. Le GFF brut est une vue avancée de diagnostic, jamais le parcours normal.

## Ateliers et critères d’acceptation

### Dialogues

- liste paginée avec recherche et résumé ;
- graphe centré sur une racine ou un nœud, profondeur réglable, recherche et limite explicite ;
- nœuds et liens lisibles sans zoom microscopique sur un DLG de plus de 1 000 nœuds ;
- arbre chargé à la demande ;
- création, suppression et liaison depuis le graphe, propriétés du nœud dans un panneau dédié ;
- cycles, liens partagés, nœuds inaccessibles et scripts visibles sans ouvrir le GFF brut.

Scénario bloquant : ouvrir un dialogue de 1 000 nœuds, retrouver une phrase, modifier le nœud,
ajouter une réponse, annuler, rétablir et valider les liens sans perte de contexte.

### Zones 2D

- liste des zones, barre d’outils, canevas et inspecteur forment quatre régions explicites ;
- zoom, recentrage, filtres de catégories et sélection d’instance ;
- déplacement direct d’une instance avec coordonnées reflétées dans l’overlay ;
- palette issue du manifeste Rust et du Resource Manager, avec recherche de blueprints ;
- édition des tuiles, transitions, rencontres, polygones et inventaires depuis la sélection ;
- suppression de zone cantonnée à la barre d’outils, jamais rendue comme une colonne du canevas.

Scénario bloquant : sur une zone 16×15 contenant plus de 400 instances, trouver un blueprint,
placer et déplacer une porte, régler sa transition, annuler et vérifier le passage dans NWN.

### Journal et quêtes

- atelier distinct des factions ;
- liste des catégories à gauche, quête sélectionnée au centre, étapes lisibles et éditables ;
- ajout/suppression de catégorie et d’étape sans afficher tous les formulaires simultanément ;
- état final, priorité, XP et texte localisé compréhensibles.

Scénario bloquant : créer une quête à cinq étapes, modifier l’état final, annuler et retrouver chaque
texte sans défilement horizontal ni colonne de mots isolés.

### Factions

- atelier distinct du journal ;
- liste, matrice de réputation et fiche de faction séparées ;
- noms visibles dans les lignes et colonnes, identifiants secondaires ;
- ajout/suppression de faction et édition d’une relation sans afficher cent formulaires à la suite.

### Blueprints

- catalogue par catégorie et type lisible ;
- éditeur principal au centre avec sections métier et sous-structures ;
- valeurs symboliques résolues par les 2DA/TLK actifs ; aucun nom numérique codé dans React ;
- provenance et GFF brut disponibles dans des onglets avancés ;
- recherche et sélection réutilisables depuis la palette de zone.

Scénario bloquant : retrouver une créature parmi plus de 6 000 blueprints, modifier son nom, sa
faction et son dialogue, ajouter un don, annuler et vérifier les références.

### Agent Studio

- parcours principal : fournisseur/modèle, test, contexte actif, objectif, création puis lancement ;
- indication exacte du moment où un appel réseau commence ;
- sélection courante importable comme portée sans saisir `resref:type` ;
- réglages de sécurité, budgets, chemins et matrice accessibles sous un mode expert ;
- journal orienté résultat, approbations formulées en termes d’effet sur le module.

## Règles de validation

- tests frontend avec jeux de données représentatifs, dont un dialogue de 1 000 nœuds ;
- contrôle de mise en page aux tailles 1280×720, 1920×1080 et 2560×1440 ;
- navigation complète au clavier pour les actions principales ;
- aucun texte principal inférieur à 11 px et aucun titre d’atelier inférieur à 16 px ;
- `pnpm --filter @opennever/desktop test:run` et `build` verts ;
- régénération et contrôle du graphe d’architecture après modification des sources ;
- preuve NWN pour les changements de zone, walkmesh, porte, dialogue ou script exécuté.

## Statut honnête

Le produit possède un moteur d’édition réel, mais la capacité de remplacement quotidien d’Aurora
reste **en cours de qualification** jusqu’à réussite de tous les scénarios ci-dessus. Les mentions de
release technique ne doivent pas être interprétées comme une validation ergonomique.
