# Plan d’amélioration transversal — 21 août 2026

Statut : lots I à V livrés et qualifiés localement le 22 août 2026. Ce document conserve le constat
initial et la séquence suivie après le Lot 40, en cohérence avec `CONTEXT.md` et
`docs/UX_REFONDATION.md`.

## Avancement au 22 août 2026

- Lot I (CI + couverture + descriptions) : **livré**.
- Lot II (`tauri.ts` en modules par domaine ; DTOs de `commands.rs` extraits vers
  `commands/dto.rs`, 7 820 → 6 841 lignes) : **livré**.
- Lot III : **livré**. `aurora-edit/lib.rs` 9 876 → 6 583 lignes via `types.rs`, `walkmesh.rs`,
  `workspace_io.rs` puis `workspace.rs` (struct `EditWorkspace` et son impl transactionnel).
  Reste volontairement dans lib.rs : l'édition structurée par domaine, extractible de la même
  façon lors du prochain travail sur ces domaines.
- Lot IV : **livré**. Arborescence `features/` par atelier avec barrels, stores Zustand
  (`uiStore`, `workbenchStore`) branchés sur App, socle commun des trois ateliers d'export dans
  `features/exports/ExportWorkshopShell.tsx`. Le scénario « dialogue 1 000 lignes borné » existe
  déjà dans `App.test.tsx`.
- Lot V : **livré**. Générateur déterministe `scripts/generate_volume_fixture.py`
  (dialogue 1 001 nœuds : 3 cycles, 2 inaccessibles, liens partagés ; zone 16x15 à 444 instances ;
  manifeste SHA-256 et mode `--check`) consommé par
  `crates/aurora-dialogue/tests/volume_fixture.rs` (< 5 s budget). Le test Rust de construction
  de zone dense via `EditWorkspace` reste à écrire lorsque l'API publique s'y prêtera sans
  modification de sources.
- Plafonds verrouillés : `App.tsx` 2 950, `commands.rs` 7 200, `aurora-edit/lib.rs` 7 000.
- Passe complète de vérification exécutée le 22 août 2026 : fmt, clippy workspace `-D warnings`,
  218 tests Rust réussis (un test local optionnel ignoré), 63 tests frontend, 13 tests Python,
  lint/build frontend, budgets bundle+sources, couverture Vitest au-dessus des seuils, graphe
  d'architecture frais, build Tauri/NSIS et distribution locale vérifiée sur 24 fichiers.

## Constat du 21 août 2026

1. Les deux workflows GitHub sont en `workflow_dispatch` uniquement : clippy, tests, audits,
   budgets et contrôle de fraîcheur du graphe ne protègent aucune branche.
2. Aucune mesure de couverture de tests (frontend comme Rust), alors que le projet exige des
   preuves partout ailleurs.
3. Trois monolithes confinés par budgets sans être remboursés :
   - `apps/desktop/src/App.tsx` — 2 901 / 3 050 lignes ;
   - `apps/desktop/src-tauri/src/commands.rs` — 7 526 / 7 700 lignes ;
   - `crates/aurora-edit/src/lib.rs` — 9 536 / 9 950 lignes.
4. L’état applicatif frontend vit dans `App.tsx` ; `store/uiStore.ts` est réduit à un champ.
5. Aucune `description` dans les 20 `Cargo.toml` du workspace.
6. `lib/tauri.ts` concentre 1 189 lignes de bindings Tauri non découpés ; les trois ateliers
   d’export dupliquent une structure de workspace quasi identique.
7. La refondation UX (`docs/UX_REFONDATION.md`) n’est pas encore portée par la structure du code
   frontend, restée monolithique.

## Axe 1 — CI sur push/PR et couverture

Objectif : faire protéger chaque contribution par les portes existantes, puis mesurer ce qui est
testé.

Travaux :

1. Déclencher `ci.yml` sur `push` (branches principales) et `pull_request`, avec groupe de
   concurrence pour annuler les exécutions redondantes. `release.yml` reste manuel.
2. Couverture frontend : Vitest provider `v8`, rapport `text-summary`, seuils initiaux honnêtes
   fixés à la valeur mesurée puis relevés à chaque lot. Étape CI dédiée bloquante.
3. Couverture Rust : `cargo-llvm-cov` installé via `taiki-e/install-action`, exécution en job
   séparé pour ne pas doubler le temps du job Windows, artefact LCOV publié, seuil informatif au
   départ.

Critères d’acceptation :

- toute PR déclenche lint + tests + budgets + clippy + graphe ;
- `pnpm coverage` échoue sous les seuils ;
- le rapport LCOV Rust est produit en CI.

## Axe 2 — Remboursement des monolithes

Principe : extraction mécanique par domaines cohérents, API publique inchangée, budgets abaissés
après chaque extraction réussie (jamais augmentés).

Ordre :

1. **`lib/tauri.ts`** → modules par domaine (`project`, `resources`, `gff`, `dialogues`,
   `zones`, `scripts`, `journal`, `factions`, `blueprints`, `exports`, `agent`, …) derrière un
   barrel `lib/tauri/index.ts` qui préserve tous les exports existants. Aucun composant modifié.
2. **`commands.rs`** → répertoire `src-tauri/src/commands/` avec un module par atelier ; façade
   `commands.rs` réduite aux réexportations. Le `generate_handler!` reste la seule liste
   d’enregistrement.
3. **`App.tsx`** → extraction progressive : état vers des slices Zustand typées, panneaux vers
   `features/<atelier>/`. Chaque extraction abaisse le plafond dans
   `tools/check-bundle-budget.mjs`.
4. **`aurora-edit/lib.rs`** → modules internes par domaine éditable (dlg, jrl, fac, blueprints,
   walkmesh, instances, sync déjà isolé), lib.rs réduit à l’assemblage public.

Critères d’acceptation :

- aucun changement de comportement observable (tests verts avant/après) ;
- chaque fichier extrait < 1 000 lignes à terme ;
- budgets sources abaissés d’au moins 20 % quand l’extraction du domaine est complète.

## Axe 3 — Structure frontend alignée sur la refondation UX

Dépend de l’axe 2 (l’état doit sortir d’`App.tsx` avant de restructurer les ateliers).

1. Arborescence `features/` par atelier (Dialogues, Zones, Journal, Factions, Blueprints,
   Exports, Agent Studio) conformes aux scénarios bloquants de `docs/UX_REFONDATION.md`.
2. Factoriser le socle commun des trois workspaces d’export (sélection → aperçu → confirmation →
   résultat) en un composant générique paramétrable.
3. Porter les scénarios d’acceptation UX (dialogue ~1 000 nœuds, zone >400 instances) en tests
   Vitest à volume réel adossés à des fixtures synthétiques générées.

Critères d’acceptation :

- chaque atelier est un dossier autonome importé par un shell mince ;
- un scénario UX par atelier tourne en CI à volume réel.

## Axe 4 — Qualité code à petit coût

1. `description` dans chaque `Cargo.toml` du workspace.
2. Barrel unique pour les bindings Tauri (fait avec l’axe 2.1).
3. Suppression des artefacts compilés suivis par erreur s’ils existent (`vitest.config.js`,
   `vitest.config.d.ts` à vérifier).

Critères d’acceptation : `cargo metadata` expose une description pour chaque membre ; aucun
artefact généré versionné.

## Axe 5 — Fixtures à volume réel

1. Générateur Python de module synthétique « gros volume » (dialogues cycliques ~1 000 nœuds,
   zone dense >400 instances, journal multi-quêtes), redistribuable, sous `fixtures/synthetic/`.
2. Tests Rust d’intégration et tests Vitest consommant ces fixtures.
3. Budgets de performance associés mesurés et consignés dans les revues de lot.

Critères d’acceptation : régénération déterministe, tests verts en CI sans ressource propriétaire.

## Séquencement retenu

| Lot | Contenu | Dépendance |
|---|---|---|
| I | Axe 1 (CI + couverture) + Axe 4.1 (descriptions) | aucune |
| II | Axe 2.1 (tauri.ts) puis 2.2 (commands.rs) | aucune |
| III | Axe 2.3–2.4 (App.tsx, aurora-edit) | II |
| IV | Axe 3 (features + factorisation exports + scénarios) | II, III |
| V | Axe 5 (fixtures volume réel) | IV pour les tests UI |

Chaque lot se termine par : lint, tests frontend et Rust, build, budgets, clippy/fmt,
`python scripts/architecture_graph.py generate` puis `check`, mise à jour de `CHANGELOG.md`.
