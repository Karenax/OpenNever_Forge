# Revue de sortie — Lot 35

Date : 4 août 2026

## Verdict

Le Lot 35 est **terminé pour son périmètre logiciel**. Agent Studio, l’orchestrateur local, les
adaptateurs fournisseur, les politiques fines, les 31 outils enregistrés, `ModuleBlueprint`, la
reprise, l’audit, les approbations par lot et l’adaptateur MCP sont intégrés dans la candidate
Windows. Les fichiers NWN sources restent immuables ; toute écriture passe par l’overlay
transactionnel existant.

Deux acceptations externes ne sont pas revendiquées : la réponse d’un fournisseur local assez
rapide et le chargement `nwserver` sur un profil qui démarre normalement. Elles ne correspondent
pas à du code manquant dans le lot.

## Contrôles automatisés

- `cargo test --workspace` : 142 tests Rust réussis, y compris le serveur MCP ;
- `aurora-agent` : 21 tests sur politiques, portées, blueprint, fournisseurs, continuité Responses,
  persistance,
  confidentialité et migration ;
- application Tauri : 6 tests, dont l’exhaustivité registre → exécuteurs ;
- adaptateur MCP : 2 tests couvrant filtrage, budgets, expurgation, initialisation et négociation
  `2025-11-25` / `2025-06-18` ;
- `aurora-edit` : 48 tests, dont création de dialogue, manifeste IFO, transactions, compilation
  exigée avant build/déploiement et synchronisation Toolset ;
- `pnpm lint` : contrat TypeScript valide ;
- `pnpm test:run` : 18 tests UI, dont les réglages fins d’Agent Studio et le chargement d’un preset ;
- `python scripts/architecture_graph.py check` : graphe frais après génération de 1 043 nœuds et
  1 155 relations ;
- `pnpm build`, `pnpm tauri build` et `cargo build --release -p opennever-mcp` : réussis.

## Candidate Windows

- portable : `target/release/opennever-forge-desktop.exe`, 24 620 032 octets,
  SHA-256 `0D753F30CBD2171A472C074753C60835ADAE3CB3FF34B2C6FBCF28E8A605F0C8` ;
- installateur NSIS : `target/release/bundle/nsis/OpenNever Forge_0.1.0_x64-setup.exe`,
  7 811 671 octets,
  SHA-256 `940EC55CB0ADA57FBD6D0CE949737F1DDCA0BF43195D20C0D1FE396D5ADB33A7` ;
- adaptateur MCP : `target/release/opennever-mcp.exe`, 1 782 784 octets,
  SHA-256 `F8D4086C35183FD41C16160D2DEC04B8BF31CE4CE5B8DB545670AFC31D788247`.

Le binaire desktop exact est resté actif pendant le contrôle de démarrage de quatre secondes, puis
a été arrêté proprement par le harnais.

## NWScript et module utilisateur

Le compilateur local `nwn_script_comp.exe` 2.1.2 a compilé `ai_smoke.nss` avec la racine réelle
`E:\Jeux\Steam\steamapps\common\Neverwinter Nights`. Le NCS produit mesure 67 octets et porte le
SHA-256 `381F3DC6CF66A9E4375C8F24E472C17AD03CA7011259BB1A91E3A86B12A88880`.

La copie de travail et le module installé `The Dark Ranger's Treasure.mod` mesurent tous deux
594 030 octets et portent le SHA-256
`172C06CD5A2178AF46CC5C2828985EAB65FB5DD68898241333B391AB4FC26019`.
Aucune qualification agentique n’a écrit dans l’un ou l’autre.

## Fournisseur et moteur externes

Ollama répond localement et annonce `gemma4:26b-a4b-it-qat`. Un appel d’outil strictement
synthétique, sans ressource NWN et limité à 64 tokens, a dépassé le délai de 45 secondes. Le verdict
est `INCONCLUSIVE_PROVIDER_PERFORMANCE`, pas un succès inventé. Les décodeurs Responses/Chat et la
construction des schémas d’outils, le stockage fournisseur optionnel, les continuités
`previous_response_id` ou rejeu local, `function_call_output` et la négociation MCP restent couverts
de façon déterministe par les tests.

Le contrôle `nwserver` reste celui de la clôture précédente : les deux installations locales
s’arrêtent avec `0xC0000005` avant écoute. Le verdict reste `INCONCLUSIVE_ENVIRONMENT`. Le cycle
Toolset réel comparer → synchroniser → compiler → sauvegarder → rouvrir est déjà positif et demeure
documenté dans `release-closure-2026-08-04.md`.
