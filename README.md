# OpenNever Forge

> **Orientation produit du 10 août 2026 :** le moteur d’édition est réel, mais le remplacement
> quotidien d’Aurora n’est pas encore qualifié. Le projet est en refondation d’utilisabilité : une
> fonction n’est désormais livrée que lorsque son parcours humain complet fonctionne sur un module
> représentatif. Voir [`docs/UX_REFONDATION.md`](docs/UX_REFONDATION.md).

Les Lots 21 à 25 ajoutent les éditeurs transactionnels TLK/2DA, les profils reproductibles, la
synchronisation contrôlée avec les dossiers temporaires d’Aurora Toolset et les migrations
sauvegardées des workspaces, puis l’assistant IA à contexte explicitement sélectionné. Toutes les sorties
restent séparées du module source, qui n'est jamais ouvert en écriture.

OpenNever Forge est un éditeur tiers moderne pour Neverwinter Nights: Enhanced Edition. Les sources
NWN restent strictement en lecture seule : l'application ouvre une copie de travail, indexe ses
ressources et applique les modifications dans un overlay transactionnel séparé avant de construire
un nouveau `.mod` ou de déployer explicitement des fichiers dans `development`.

Le contexte complet est dans [`CONTEXT.md`](CONTEXT.md), le séquencement dans
[`CONSTRUCTION_PLAN.md`](CONSTRUCTION_PLAN.md) et la prochaine qualification dans
[`docs/LOT40_RELEASE_ACCEPTANCE_PLAN.md`](docs/LOT40_RELEASE_ACCEPTANCE_PLAN.md).

## État actuel

Les portes techniques des Lots 0 à 40 ne constituent plus, à elles seules, un verdict de produit
exploitable. Les ateliers Dialogues, Zones, Journal, Factions, Blueprints et Agent Studio sont en
cours de refondation selon des scénarios d’acceptation à volume réel. Toute promotion publique reste
donc bloquée à la fois par les exigences externes du Lot 40 et par ces portes d’utilisabilité.

Les Lots 0 à 10 disposent de leur porte de sortie et la Phase 1 de lecture est complète. Les Lots
11 à 16 fournissent le socle transactionnel récupérable, les writers GFF/ERF en streaming, l'édition
NSS, la compilation NCS liée aux includes exacts, l'édition 2D des zones et la construction d'un
nouveau MOD. La Phase 3 est exécutable jusqu’au Lot 25 : création de module et de zones, palettes,
walkmeshes, contenus personnalisés, builds reproductibles, synchronisation Toolset et migrations de
projets. L’assistance IA propose uniquement des opérations GFF/NSS bornées, prévisualisées contre
les octets courants et confirmées par empreinte avant application annulable. Les Lots 36 à 39
ajoutent les audits de dépendances, le cache persistant de l'installation, la progression par
phases, la qualification de release, les manifestes SHA-256, le chargement paresseux de
Monaco/React Flow et des budgets bloquants. Ces lots sont implémentés. Le périmètre logiciel local
du Lot 40 est également livré : SBOM CycloneDX, manifeste de distribution, checksums, signature
conditionnelle et workflow manuel protégé. La qualification client NWN WOK/PWK/DWK passe. Son
verdict reste `BLOQUÉ_EXTERNE` jusqu’au certificat, au profil Windows propre et à l’autorisation de
publication.
Le shell Tauri/React calcule l'empreinte d'une copie `.mod`, résout
son catalogue MOD/HAK/override/development/patch/KEY-BIF et explique la provenance de chaque
version. L'explorateur paginé ouvre à la demande les GFF, TLK et 2DA, ou un aperçu binaire pour les
formats inconnus. Le cœur expose déjà le module, les zones ARE, les instances GIT, les données GIC
et les principaux blueprints. Les catalogues et références de dépendances sont persistés dans
SQLite. Le Lot 4 ajoute l'inventaire NSS/NCS, la recherche plein texte, un éditeur Monaco et les
références entrantes depuis les objets GFF. Le Lot 5 représente les DLG comme des
graphes fidèles, avec arbre borné, vue React Flow complète, GFF brut, textes localisés, scripts et
références entrantes. Les vues de fin de Phase 1 ajoutent le journal JRL, les factions FAC, la carte
2D ARE/GIT/GIC, l'inventaire d'assets, un lecteur MDL binaire/ASCII interne, un cache GLB versionné,
les textures TGA/DDS/KTX/PLT et un manifeste de scène Rust rendu par Babylon.js. Les SET,
blueprints et 2DA alimentent directement les tuiles, portes, placeables et créatures composites de
la vue 3D, avec chargement GLB progressif, budget mémoire et marqueurs locaux en cas d'échec. Le
graphe global reste exportable en JSON ou HTML. Aucune archive source n'est ouverte en écriture :
les builds, exports et déploiements sont de nouvelles sorties explicites.

Le Lot 13 permet maintenant de modifier transactionnellement les champs existants des DLG, JRL,
FAC et des blueprints, y compris les variantes localisées, les cibles de liens et les réputations.
La création/suppression structurelle est également disponible pour les nœuds/liens DLG et les
catégories/étapes JRL, ainsi que pour les factions et réputations FAC. Les suppressions FAC
réindexent automatiquement parents et relations, tandis que l'ajout complète la matrice Aurora.
Les sous-structures prioritaires des blueprints sont également transactionnelles : dons,
capacités, classes et équipement UTC, propriétés UTI, sons UTS et créatures UTE. Le Lot 19 ajoute
les polygones UTT/UTE, les points d'apparition, les transitions et les inventaires des instances
de placeables et magasins. Un objet ajouté est incorporé depuis son blueprint UTI résolu afin de
préserver ses propriétés et les champs GFF inconnus ; la carte est ensuite relue depuis l'overlay.
Le Lot 20 ajoute un atelier WOK/PWK/DWK avec aperçu topologique, surfaces par face, déplacement de
sommet, découpe, suppression, extrusion et soudure contrôlées. Le writer produit les grammaires
ASCII autonomes réelles de NWN, y compris l'arbre AABB déterministe des WOK et les variantes/hooks
des PWK/DWK. Les nouvelles ressources sont créées dans l'overlay ; le remplacement d'un walkmesh
existant reste une action complète explicitement confirmée et annulable.

Les revues de sortie et leurs limites sont consignées dans
[`docs/validation/lot1-exit-review.md`](docs/validation/lot1-exit-review.md) et
[`docs/validation/lot2-lot3-exit-review.md`](docs/validation/lot2-lot3-exit-review.md), puis dans
[`docs/validation/lot4-exit-review.md`](docs/validation/lot4-exit-review.md) et
[`docs/validation/lot5-exit-review.md`](docs/validation/lot5-exit-review.md), puis
[`docs/validation/lot6-lot10-exit-review.md`](docs/validation/lot6-lot10-exit-review.md) et
[`docs/validation/lot20-exit-review.md`](docs/validation/lot20-exit-review.md), puis
[`docs/validation/lot21-lot22-exit-review.md`](docs/validation/lot21-lot22-exit-review.md) et
[`docs/validation/lot23-lot24-exit-review.md`](docs/validation/lot23-lot24-exit-review.md), puis
[`docs/validation/lot25-exit-review.md`](docs/validation/lot25-exit-review.md) et
[`docs/validation/lot36-lot39-exit-review.md`](docs/validation/lot36-lot39-exit-review.md), puis
[`docs/validation/lot40-exit-review.md`](docs/validation/lot40-exit-review.md).
L'état précis des Lots 11 à 25, les contrôles d'oracle et l'ordre restant sont dans
[`docs/validation/phase2-phase3-foundation-review.md`](docs/validation/phase2-phase3-foundation-review.md).
La [documentation complète utilisateur et technique en HTML](docs/OpenNever_Forge_Manuel_Complet.html)
réunit les parcours, l'architecture, les formats NWN, la sécurité IA et MCP. Le parcours opérationnel
condensé est décrit dans [`docs/USER_GUIDE.md`](docs/USER_GUIDE.md) et la politique de compatibilité
dans [`docs/MIGRATIONS.md`](docs/MIGRATIONS.md).

## Ouvrir directement une carte `.are`

Le sélecteur de source accepte un module `.mod` complet ou une carte `.are` autonome. Pour une
carte, OpenNever charge automatiquement les fichiers `.git` et `.gic` portant le même ResRef dans
le même dossier, puis utilise les racines NWN indiquées pour résoudre tilesets, modèles et textures.

Ce parcours reste volontairement en lecture seule : il donne accès aux cartes 2D/3D, aux instances,
aux diagnostics et aux exportateurs locaux, mais ne crée pas d’espace d’édition. Il faut ouvrir un
module `.mod` pour produire des modifications transactionnelles ou construire un nouveau MOD.

## Exporter une carte vers un bundle local

L’atelier **Exporter une carte** audite une zone de l’analyse active puis produit un
[`Area Migration Bundle v1`](docs/AREA_MIGRATION_BUNDLE_V1.md) déterministe sans modifier le MOD,
les HAK, le TLK ou l’installation. Le bundle reste local : convertir ou préserver une ressource
NWN ne lui accorde aucun droit de redistribution. Les WOK/PWK/DWK disponibles sont conservés mais
ne sont pas convertis en navigation.

Le même moteur est disponible sans interface :

```powershell
cargo run -p opennever-migrate -- area-audit --module C:\modules\source.mod --area startarea
cargo run -p opennever-migrate -- area-export --module C:\modules\source.mod --area startarea --output C:\exports\startarea.area-migration-v1
```

Ajoutez `--game-install` et `--user-data` lorsque la fermeture de ressources dépend de
l’installation, des HAK, du TLK ou des overrides. La CLI n’effectue aucune découverte implicite de
ces racines et retourne un code non nul lorsque le résultat n’est pas complet.

## Exporter un asset statique ou animé

Le menu supérieur **Export** contient aussi l’atelier **Exporter des assets**, immédiatement après
**Exporter une carte**. À partir de l’analyse active, il résout un modèle Aurora et ses dépendances,
puis crée un nouveau dossier local contenant un GLB, ses textures PNG et un `manifest.json` avec
tailles et SHA-256. Les animations réellement écrites dans le GLB, y compris celles héritées d’un
supermodel résolu, sont affichées avant l’export ; un modèle sans clip exporté reste explicitement
qualifié de statique.

L’export ne modifie jamais les MOD, HAK, KEY/BIF, overrides ni fichiers autonomes analysés. La
destination doit être neuve, hors des racines protégées, et l’utilisateur doit confirmer que le
résultat reste local sauf s’il possède séparément les droits de redistribution. Le format est décrit
dans [`docs/ASSET_EXPORT_V1.md`](docs/ASSET_EXPORT_V1.md).

## Exporter un dialogue

Le troisième atelier du menu **Export**, **Exporter des dialogues**, privilégie la version modifiée
du workspace lorsqu’elle existe, sinon la version analysée. Il crée un dossier local contenant le
DLG exact sélectionné, un `dialogue.json` portable sans chemins sources, un `transcript.md` lisible
et un manifeste SHA-256. Les cycles, liens cassés, nœuds partagés ou inaccessibles, scripts et
références restent explicitement inventoriés.

Les dialogues nouvellement créés dans le workspace sont également exportables tant que cet espace
d’édition reste ouvert. La destination est neuve et extérieure aux racines NWN protégées ; aucun
MOD, HAK, TLK ou DLG source n’est réécrit. Le contrat est décrit dans
[`docs/DIALOGUE_EXPORT_V1.md`](docs/DIALOGUE_EXPORT_V1.md).

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

La porte locale complète de candidate Windows s'exécute avec :

```powershell
./scripts/verify_release.ps1 -ExpectedVersion 0.1.0
./scripts/verify_distribution.ps1 -ExpectedVersion 0.1.0
```

Elle produit `target/release/release-manifest.json`, `target/release/SHA256SUMS` et les SBOM dans
`target/release/sbom` après les audits, tests, budgets, builds Tauri et MCP. Le manifeste indique
explicitement si l’arbre est sale et si les binaires sont signés. Les modes `-RequireClean` et
`-RequireSigned` bloquent une promotion incomplète.

La publication est préparée par le workflow manuel `.github/workflows/release.yml`. Elle exige un
tag préexistant cohérent, un certificat injecté par l’environnement protégé `release-signing` et une
demande explicite de brouillon. Un test local ne crée jamais de tag, de push ou de GitHub Release.

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

OpenNever Forge est distribué sous la **PolyForm Noncommercial License 1.0.0** pour les usages autorisés non commerciaux. Voir [`LICENSE`](LICENSE).

Toute utilisation commerciale d’OpenNever Forge nécessite une licence commerciale séparée. Les modalités commerciales sont décrites dans [`COMMERCIAL-LICENSE.md`](COMMERCIAL-LICENSE.md).

Les versions publiées avant ce changement de licence restent régies par la licence qui leur était applicable au moment de leur publication, notamment Apache License 2.0 lorsque celle-ci s’appliquait.

Les ressources Neverwinter Nights chargées localement restent la propriété de leurs ayants droit et ne font pas partie de ce dépôt.
