# Revue de sortie — Lots 36 à 39

Date de validation : 2026-08-09
Verdict : **PASS local pour les Lots 36 à 39**

Cette revue couvre la stabilisation, les performances, la reproductibilité de la candidate Windows
et la maîtrise de la dette structurelle. Les sources NWN sont restées strictement en lecture seule.

## Lot 36 — CI, qualité et dépendances

- la CI Windows exécute installation figée, audit pnpm, typecheck, tests frontend, build, budgets,
  formatage, Clippy avec avertissements interdits, tests Rust, build Tauri, build MCP, tests Python et
  fraîcheur du graphe ;
- un job Linux séparé applique `cargo-deny` et l'audit RustSec ;
- `dompurify` est verrouillé en version `3.4.13` et `pnpm audit --prod --audit-level=low` ne remonte
  aucune vulnérabilité connue ;
- `cargo deny check all` passe localement. Les dépendances GTK 3 transitives de Tauri restent
  signalées comme non maintenues par RustSec sur Linux, sans dépendance directe non maintenue dans
  le workspace ;
- le défaut Clippy de construction de `ContextPolicy` est corrigé.

## Lot 37 — Catalogue, cache et progression

Le catalogue du jeu dispose maintenant d'un cache JSON versionné. Sa signature couvre le chemin de
la racine du jeu, les fichiers KEY/BIF et les fichiers d'override avec leur taille et leur date de
modification. Toute corruption, divergence de schéma ou modification de source provoque un miss et
une reconstruction. Les modules, HAK et dossiers de développement sont toujours relus à chaque
analyse.

Mesure release sur `The Dark Ranger's Treasure.mod` :

| Passage | Cache | Durée | Ressources | Versions | Scripts | Dialogues | Zones |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | miss puis écriture | 5 081 ms | 113 655 | 113 715 | 4 233 | 42 | 3 |
| 2 | hit | 2 992 ms | 113 655 | 113 715 | 4 233 | 42 | 3 |
| 3 | hit | 2 636 ms | 113 655 | 113 715 | 4 233 | 42 | 3 |

Les résultats métier sont identiques entre cache froid et cache chaud. La progression expose des
phases distinctes : hash, inventaire, dépendances, catalogue, ressources structurées, scripts,
dialogues, monde et persistance.

## Lot 38 — Candidate, manifeste et oracle d'exécution

`scripts/verify_release.ps1` a terminé avec `RELEASE_VERIFICATION_PASS` en 84,2 secondes. Le passage
complet comprend 27 tests frontend, 151 tests Rust recensés, 13 tests Python, les audits, le graphe,
les budgets et les trois artefacts Windows.

Le manifeste local `target/release/release-manifest.json` contient :

| Artefact | Taille | SHA-256 |
| --- | ---: | --- |
| `opennever-forge-desktop.exe` | 24 824 832 | `8FEAE35370173984D9494286178783F3C5CDCE3935A6CC903839FAB9E48D0426` |
| `opennever-mcp.exe` | 1 782 784 | `436EE128BB1074945AAC3EF76997FCFCEB78D8280A0AA72A8745AFE72263BB01` |
| installeur NSIS | 7 888 092 | `D090D51CEE1BF37B194A40F38E50373B643C620BDE232308D22F08B70E0B584D` |

La candidate exacte a été ouverte et inspectée visuellement : le shell, le badge de protection des
sources et la nouvelle métrique `Catalogue direct` sont présents.

Un premier test `nwserver` avait produit `0xC0000005` pour le témoin et l'overlay. La comparaison
avec un lancement manuel a révélé que le harnais héritait du dossier de travail du dépôt, alors que
le serveur exige son propre dossier `bin/win32`. Après correction de `Start-Process`, le même binaire
`E:\SteamLibrary\steamapps\common\Neverwinter Nights\bin\win32\nwserver.exe` version
`89.8193.37-17` passe le 9 août 2026 : témoin en écoute UDP sur 5139, overlay en écoute sur 5140,
trois walkmeshes produits et `Status: PASS`.

Ce résultat prouve le démarrage serveur du témoin et de l'overlay. La connexion d'un client et
l'observation en jeu des collisions WOK/PWK/DWK restent une acceptation manuelle distincte du
Lot 40.

Le module source contrôlé mesure 594 030 octets et conserve le SHA-256
`172C06CD5A2178AF46CC5C2828985EAB65FB5DD68898241333B391AB4FC26019`.

## Lot 39 — Chargement différé et budgets

- Monaco et ReactFlow ne sont plus importés au démarrage ; les éditeurs NWScript et dialogue sont
  chargés à la demande avec `React.lazy` et `Suspense` ;
- le cache du catalogue est isolé dans son propre module Rust ;
- les budgets échouent explicitement en cas de dépassement du bundle ou de croissance des trois
  principaux fichiers monolithiques.

Mesures finales :

| Élément | Mesure | Budget |
| --- | ---: | ---: |
| entrée JavaScript | 3 162 460 octets / 828 950 gzip | 3 300 000 / 900 000 |
| plus gros chunk | 3 765 850 octets / 830 620 gzip | 4 000 000 / 950 000 |
| CSS | 173 739 octets | 200 000 |
| `App.tsx` | 2 800 lignes | 2 825 |
| `commands.rs` | 6 394 lignes | 6 500 |
| `aurora-edit/src/lib.rs` | 9 090 lignes | 9 100 |

Les chunks Vite restent volumineux et les budgets de lignes sont proches de leur plafond : la dette
est maintenant bornée et bloquante en CI, mais une modularisation supplémentaire demeure souhaitable.

## Limites de livraison externes

- les binaires et l'installeur sont volontairement non signés (`signed: false`) faute de certificat ;
- aucun tag, commit, push ou GitHub Release n'a été créé par cette validation locale ;
- la connexion d'un client NWN et l'observation en jeu des trois walkmeshes restent à effectuer sur
  la candidate finale du Lot 40.
