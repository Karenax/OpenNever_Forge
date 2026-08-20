# Consolidation UX d’OpenNever Forge — 20 août 2026

## Objet

Cette livraison ajoute une couche UX isolée au-dessus du shell React existant afin d’améliorer
l’utilisabilité sans modifier les transactions, les formats NWN, les appels Tauri ni le moteur Rust.
Elle sert aussi de spécification exécutable pour la refactorisation native ultérieure du shell.

## Fichiers concernés

- `apps/desktop/src/main.tsx`
- `apps/desktop/src/components/UxEnhancements.tsx`
- `apps/desktop/src/components/UxEnhancements.model.ts`
- `apps/desktop/src/components/UxEnhancements.dom.ts`
- `apps/desktop/src/components/UxEnhancements.navigation.tsx`
- `apps/desktop/src/components/UxEnhancements.portals.tsx`
- `apps/desktop/src/components/UxEnhancements.css`
- `apps/desktop/src/components/UxEnhancements.test.ts`

## Améliorations livrées

### Navigation

- six domaines fonctionnels cohérents : Projet, Monde, Narration, Contenu, Validation et Agent ;
- filtrage de l’explorateur selon le domaine actif ;
- aide retirée de la navigation métier et conservée dans l’accès d’aide existant ;
- raccourcis `Alt+1` à `Alt+6` ;
- palette de commandes `Ctrl+K`, avec recherche insensible aux accents.

### Shell de travail

- largeur de l’explorateur et de l’inspecteur redimensionnable par glisser-déposer ;
- persistance locale des largeurs et des états réduit/déployé ;
- libellés fonctionnels « Explorateur » et « Inspecteur », avec le vocabulaire RPG conservé en
  infobulle ;
- alias de tokens CSS manquants pour éviter les variables non résolues.

### Lisibilité et accessibilité

- plancher typographique de 11 à 13 px pour les informations fonctionnelles ;
- couleur secondaire remontée à un contraste AA sur les fonds principaux ;
- focus clavier visible et homogène ;
- contrôles principaux d’au moins 36 px ;
- réduction des informations techniques minuscules.

### Diagnostics

- suppression visuelle des onglets inactifs `Import` et `Journal` ;
- filtres Erreurs, Avertissements et Informations ;
- compteurs par gravité ;
- copie de l’ensemble des diagnostics ou d’une ligne par double-clic ;
- retrait du message permanent `SOURCE_READ_ONLY` du compteur, puisque cette garantie est déjà
  affichée dans le badge global.

### Créateur de cartes

- présentation en étapes : Décrire, Générer, Ajuster, Créer et Atlas ;
- libellés orientés utilisateur : « Créer une zone à partir d’un brief » et « Créer la zone » ;
- masquage par défaut des champs de connexion, ResRef et tuile de base ;
- mode expert pour la connexion et les listes précises de blueprints ;
- densités accompagnées d’une qualification lisible : Aucune, Faible, Normale, Riche ou Très riche ;
- suppression du doublon d’accès à Agent Studio ;
- prévisualisation et action de création mises au premier plan.

## Validation effectuée hors dépôt

- analyse syntaxique TypeScript de tous les fichiers TS/TSX de la couche : valide ;
- vérification stricte avec des déclarations de modules de test reproduisant `strict`,
  `noUnusedLocals` et `noUnusedParameters` : valide ;
- analyse syntaxique CSS avec PostCSS : valide ;
- contraste de `--faint: #92897d` : entre 4,95:1 et 5,72:1 sur les principaux fonds sombres.

Les tests et le build complets doivent être exécutés dans le dépôt réel, car l’environnement ayant
produit ce patch ne possède ni le checkout privé complet ni ses `node_modules`.

## Limites volontaires de cette livraison

La couche est déjà divisée en modèle, annotations DOM, navigation et portails React. Elle utilise
encore des portails React et une synchronisation DOM pour éviter une modification massive
de `App.tsx`. Cette approche réduit le risque immédiat mais ne doit pas devenir l’architecture finale.
Le prompt `UX_REFACTOR_FOLLOWUP_PROMPT.md` décrit la migration native attendue : découpage du shell,
vrais onglets multi-documents, paramètres IA centralisés, sélecteur ResRef, diagnostics navigables et
tests visuels.
