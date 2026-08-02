# OpenNever Forge

OpenNever Forge est un éditeur tiers moderne pour Neverwinter Nights: Enhanced Edition. La première
phase du projet est strictement en lecture seule : elle ouvre une copie de travail d'un module,
indexe ses ressources et explique leur provenance sans modifier le `.mod`, les HAK ou
l'installation NWN d'origine.

Le contexte complet est dans [`CONTEXT.md`](CONTEXT.md) et le séquencement dans
[`CONSTRUCTION_PLAN.md`](CONSTRUCTION_PLAN.md).

## État actuel

Le Lot 0 est terminé et le premier parcours du Lot 1 est fonctionnel. Le shell Tauri/React calcule
l'empreinte d'une copie `.mod`, valide son index ERF, inventorie ses ressources et lit les
métadonnées minimales de `module.ifo`. Aucun contenu n'est extrait et aucune fonction d'écriture
n'est activée.

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

## Architecture

Le frontend React appelle uniquement des commandes Tauri typées. Les opérations binaires, SQLite,
les jobs et les futurs parsers NWN vivent en Rust. Les gros assets seront transformés dans un cache
local puis exposés au frontend sous forme de manifestes ou de fichiers adaptés au rendu.

`nwn-lib-rs` n'est pas lié au binaire principal : sa licence LGPL-3.0-or-later et son usage actuel de
fonctionnalités Rust nightly ne correspondent pas au socle stable et permissif du projet. Il peut
servir d'oracle externe tant qu'aucun code n'est copié.

## Licence

Apache-2.0. Les ressources Neverwinter Nights chargées localement restent la propriété de leurs
ayants droit et ne font pas partie de ce dépôt.
