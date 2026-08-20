# Prompt Codex — terminer la refonte UX native d’OpenNever Forge

Tu travailles dans le dépôt **OpenNever_Forge**. Commence par lire intégralement `CONTEXT.md`,
`AGENTS.md`, `docs/UX_REFONDATION.md`, `docs/MAP_CREATOR_PLAN.md` et
`docs/UX_IMPLEMENTATION_2026-08-20.md`.

Une première couche de consolidation existe normalement dans :

- `apps/desktop/src/components/UxEnhancements.tsx`
- `apps/desktop/src/components/UxEnhancements.model.ts`
- `apps/desktop/src/components/UxEnhancements.dom.ts`
- `apps/desktop/src/components/UxEnhancements.navigation.tsx`
- `apps/desktop/src/components/UxEnhancements.portals.tsx`
- `apps/desktop/src/components/UxEnhancements.css`
- `apps/desktop/src/components/UxEnhancements.test.ts`
- `apps/desktop/src/main.tsx`

Si ces fichiers sont absents, applique d’abord le patch `opennever-ux-improvements.patch` fourni avec
ce prompt. Ne réimplémente pas en parallèle une seconde solution concurrente.

## But

Transformer les améliorations provisoires en architecture React native, puis terminer les éléments
qui nécessitent l’accès au dépôt complet, l’exécution de l’application et une validation visuelle.
L’interface finale doit rester sombre, médiévale et technique, mais les noms fonctionnels doivent être
lus avant le vocabulaire d’ambiance. Ne modifie aucun fichier ou asset NWN propriétaire.

## Contraintes absolues

1. Préserve les garanties transactionnelles et l’immutabilité du MOD, des HAK et de l’installation.
2. Ne change aucun contrat Tauri/Rust sans nécessité démontrée.
3. Ne supprime aucune capacité métier existante.
4. Ne remplace pas les ateliers spécialisés par un formulaire GFF générique.
5. Ne crée pas de second store global concurrent ; étends proprement Zustand ou crée des stores
   spécialisés explicitement nommés.
6. Toutes les actions destructrices conservent confirmation, annulation et préconditions existantes.
7. Aucun texte fonctionnel sous 11 px ; contrôles principaux de 36 px minimum ; focus clavier visible.
8. Ne committe et ne pousse rien sans demande explicite. Travaille sur une branche de fonctionnalité
   si un commit est ensuite autorisé.

## Travail à réaliser

### 1. Intégrer nativement le shell UX

Découpe progressivement `App.tsx` sans réécrire les ateliers métier :

- `components/shell/AppShell.tsx`
- `components/shell/DomainNavigation.tsx`
- `components/shell/WorkspaceExplorer.tsx`
- `components/shell/WorkspaceTabs.tsx`
- `components/shell/ContextInspector.tsx`
- `components/shell/DiagnosticsPanel.tsx`
- `components/shell/CommandPalette.tsx`
- `store/workbenchStore.ts`

Déplace dans ces composants le comportement actuellement assuré par `UxEnhancements.tsx` : domaines,
filtrage de l’explorateur, `Ctrl+K`, raccourcis `Alt+1…6`, tailles de panneaux, persistance et filtres
de diagnostics. Une fois chaque comportement couvert par des tests natifs, retire son équivalent DOM
de `UxEnhancements`. À la fin, `UxEnhancements` doit être supprimé ou réduit à zéro comportement.

### 2. Créer de vrais onglets multi-documents

Implémente un modèle de document ouvert pour les zones, scripts, dialogues, blueprints, tables et
ressources :

- ouverture simple ou en nouvel onglet ;
- réutilisation de l’onglet si le même document est déjà ouvert ;
- point ou badge pour les modifications présentes dans l’overlay ;
- fermeture avec sélection intelligente de l’onglet voisin ;
- restauration des onglets à la réouverture du projet ;
- raccourcis `Ctrl+W`, `Ctrl+Tab`, `Ctrl+Shift+Tab` ;
- aucune perte de la sélection, du filtre ou de la position de défilement lors d’un changement
  d’onglet.

Le bandeau actuel qui imite un onglet unique doit devenir ce vrai système de documents.

### 3. Centraliser les fournisseurs IA

Crée une configuration commune utilisée par le Créateur de cartes, Agent Studio et l’assistant
ponctuel :

- fournisseur ;
- endpoint ;
- modèle ;
- test de connexion ;
- capacités structurées/outils ;
- coût facultatif ;
- état local/distant ;
- clé API éphémère, jamais persistée.

L’écran métier ne doit afficher que le profil actif, le modèle et l’état de connexion. Les réglages
techniques restent dans un panneau « Fournisseurs IA ». Préserve les règles de confidentialité du
créateur de cartes.

### 4. Refactoriser réellement le Créateur de cartes

Scinde `MapCreator.tsx` en composants natifs :

- `MapCreatorWizard.tsx`
- `MapBriefStep.tsx`
- `MapGenerationStep.tsx`
- `MapAdjustmentStep.tsx`
- `MapCreationStep.tsx`
- `MapPreviewCanvas.tsx`
- `MapAtlas.tsx`
- `ResRefMultiSelect.tsx`

Le parcours doit être :

1. **Décrire** — brief, modèles d’ambiance, taille qualitative et points d’intérêt.
2. **Générer** — IA ou générateur local, fournisseur actif résumé, paramètres déterministes.
3. **Ajuster** — grande carte, filtres de catégories, zoom, légende, avertissements et densités
   qualitatives ; valeurs exactes en mode expert.
4. **Créer** — résumé des ressources produites, compatibilité, overlay ciblé et action
   « Créer la zone ».
5. **Atlas** — onglet séparé, pas troisième colonne permanente.

Le ResRef est proposé automatiquement à partir du nom mais reste modifiable en mode expert.
Le sélecteur de blueprints doit rechercher le catalogue local et afficher des chips supprimables ;
interdis les listes de ResRef séparées par des virgules dans le parcours normal.

### 5. Rendre les diagnostics navigables et actionnables

Étends le modèle de diagnostic frontend avec, lorsque disponible :

- `resourceKey` ;
- `workspaceView` ;
- `fieldPath` ;
- `suggestedAction` ;
- `technicalDetails`.

Un clic ouvre l’atelier et la ressource concernés. Ajoute « Copier le détail technique » et, seulement
lorsqu’une action déterministe existe, « Corriger ». Regroupe par analyse, import, validation, build et
test. N’affiche aucun onglet vide.

### 6. Consolider le design system

Réduis les redéfinitions en cascade de `App.css`. Crée :

- `styles/tokens.css`
- `styles/reset.css`
- `styles/typography.css`
- `styles/controls.css`
- `styles/shell.css`
- un fichier CSS par atelier.

Crée ou consolide les composants communs : `WorkspaceHeader`, `BrowserPanel`, `InspectorPanel`,
`Toolbar`, `Tabs`, `StatusBanner`, `Metric`, `EmptyState`, `SearchField`, `PropertyField`, `Badge` et
`SplitPane`.

Supprime les variables inexistantes et garde une seule source pour les tokens. Vérifie au minimum un
contraste WCAG AA pour tout texte fonctionnel.

### 7. Tests fonctionnels, clavier, accessibilité et visuels

Conserve tous les tests existants et ajoute :

- navigation complète par domaine ;
- palette `Ctrl+K` et raccourcis clavier ;
- restauration des panneaux et onglets ;
- filtrage et navigation des diagnostics ;
- parcours complet du créateur de cartes ;
- sélection de plusieurs blueprints ;
- absence de persistance de clé API ;
- navigation au clavier des listes, onglets, boîtes de dialogue et actions principales.

Ajoute une validation visuelle automatisée aux tailles :

- 1280×720 ;
- 1920×1080 ;
- 2560×1440.

Génère des captures représentatives pour : Table de campagne, Zones, Dialogues, Blueprints, Créateur
de cartes, Construire et tester, Agent Studio et Diagnostics. Range-les dans
`docs/validation/ux-refactor/` avec un court rapport indiquant les défauts trouvés et corrigés.

## Validation obligatoire

Exécute, corrige et relance jusqu’à réussite :

```bash
pnpm --filter @opennever/desktop lint
pnpm --filter @opennever/desktop test:run
pnpm --filter @opennever/desktop build
python scripts/architecture_graph.py generate
python scripts/architecture_graph.py check
python -m unittest tests/test_architecture_graph.py
```

Si des fichiers Rust sont modifiés, ajoute :

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

## Livrables attendus

1. Code refactorisé et tests verts.
2. Rapport `docs/validation/ux-refactor-exit-review.md` contenant :
   - changements réalisés ;
   - parcours validés ;
   - résultats de tests ;
   - captures ;
   - limites restantes, sans exagération.
3. Graphe d’architecture régénéré.
4. État Git final listant précisément les fichiers modifiés.
5. Aucun commit ou push sans autorisation explicite.
