# OpenNever Forge

OpenNever Forge est un éditeur tiers moderne pour Neverwinter Nights: Enhanced Edition. La première
phase du projet est strictement en lecture seule : elle ouvre une copie de travail d'un module,
indexe ses ressources et explique leur provenance sans modifier le `.mod`, les HAK ou
l'installation NWN d'origine.

Le contexte complet est dans [`CONTEXT.md`](CONTEXT.md) et le séquencement dans
[`CONSTRUCTION_PLAN.md`](CONSTRUCTION_PLAN.md).

## État actuel

Les Lots 0 à 10 disposent de leur porte de sortie et la Phase 1 de lecture est complète. Le shell
Tauri/React calcule l'empreinte d'une copie `.mod`, résout
son catalogue MOD/HAK/override/development/patch/KEY-BIF et explique la provenance de chaque
version. L'explorateur paginé ouvre à la demande les GFF, TLK et 2DA, ou un aperçu binaire pour les
formats inconnus. Le cœur expose déjà le module, les zones ARE, les instances GIT, les données GIC
et les principaux blueprints. Les catalogues et références de dépendances sont persistés dans
SQLite. Le Lot 4 ajoute l'inventaire NSS/NCS, la recherche plein texte, un éditeur Monaco en lecture
seule, une vue technique NCS et les références entrantes depuis les objets GFF. Aucune fonction
d'écriture NWN ni compilation de script n'est activée. Le Lot 5 représente les DLG comme des
graphes fidèles, avec arbre borné, vue React Flow complète, GFF brut, textes localisés, scripts et
références entrantes. Les vues de fin de Phase 1 ajoutent le journal JRL, les factions FAC, la carte
2D ARE/GIT/GIC, l'inventaire d'assets, un lecteur MDL binaire/ASCII interne, un cache GLB versionné,
les textures TGA/DDS/KTX/PLT et un manifeste de scène Rust rendu par Babylon.js. Les SET,
blueprints et 2DA alimentent directement les tuiles, portes, placeables et créatures composites de
la vue 3D, avec chargement GLB progressif, budget mémoire et marqueurs locaux en cas d'échec. Le
graphe global reste exportable en JSON ou HTML. L'application demeure volontairement en lecture
seule tant que la Phase 2 n'est pas engagée explicitement.

Les revues de sortie et leurs limites sont consignées dans
[`docs/validation/lot1-exit-review.md`](docs/validation/lot1-exit-review.md) et
[`docs/validation/lot2-lot3-exit-review.md`](docs/validation/lot2-lot3-exit-review.md), puis dans
[`docs/validation/lot4-exit-review.md`](docs/validation/lot4-exit-review.md) et
[`docs/validation/lot5-exit-review.md`](docs/validation/lot5-exit-review.md), puis
[`docs/validation/lot6-lot10-exit-review.md`](docs/validation/lot6-lot10-exit-review.md).

## Prérequis Windows

- Windows 10 ou 11 avec Microsoft Edge WebView2 ;
- Microsoft C++ Build Tools, charge `Desktop development with C++` ;
- Rust stable MSVC via rustup ;
- Node.js 22 et pnpm via Corepack ;
- Python 3.11+ pour le graphe d'architecture local.

## Développement

```powershell
corepack enable
pnpm install
pnpm test:run
pnpm build
cargo test --workspace
pnpm tauri dev
```

Le module de test local doit rester sous `.tmp/` ou `local-data/`, tous deux ignorés par Git.

La fixture synthétique et redistribuable du Lot 1 peut être régénérée puis contrôlée avec :

```powershell
python scripts/generate_lot1_fixture.py fixtures/synthetic/lot1_custom_tlk --force
python -m unittest discover -s tests -v
cargo test -p aurora-project --test synthetic_lot1
```

La comparaison indépendante avec `neverwinter.nim` reste opt-in et nécessite ses exécutables dans
un dossier externe :

```powershell
python tools/compare-oracles/compare_neverwinter_nim.py --oracle-dir C:\outils\neverwinter
```

## Architecture

Le frontend React appelle uniquement des commandes Tauri typées. Les opérations binaires, SQLite,
les jobs et les futurs parsers NWN vivent en Rust. Les gros assets seront transformés dans un cache
local puis exposés au frontend sous forme de manifestes ou de fichiers adaptés au rendu.

Le lecteur `aurora-mdl` est une implémentation Apache-2.0 indépendante fondée sur la description de
format CC0 `NWN1MDL.bt`. Aucun parseur GPL n'est lié ou copié. Les GLB produits peuvent être
contrôlés avec `pnpm validate:glb <fichier.glb>` ; ce contrôle utilise le validateur Khronos
Apache-2.0 uniquement en développement.

`nwn-lib-rs` n'est pas lié au binaire principal : sa licence LGPL-3.0-or-later et son usage actuel de
fonctionnalités Rust nightly ne correspondent pas au socle stable et permissif du projet. Il peut
servir d'oracle externe tant qu'aucun code n'est copié.

Le premier oracle effectivement validé est `neverwinter.nim` 2.1.2, sous licence MIT et exécuté
uniquement comme outil local opt-in. Il n'est ni téléchargé par le build ni distribué avec
l'application.

## Licence

Apache-2.0. Les ressources Neverwinter Nights chargées localement restent la propriété de leurs
ayants droit et ne font pas partie de ce dépôt.
