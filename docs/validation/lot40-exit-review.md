# Revue d’exécution du Lot 40

> **Portée de cette preuve : technique et moteur uniquement.** La revue ne valide pas l'ergonomie ni
> l'usage quotidien du logiciel. Depuis le 10 août 2026, la qualification produit dépend aussi des
> portes de `docs/UX_REFONDATION.md`; leur échec bloque toute conclusion « remplaçant d'Aurora ».

Date : 9 août 2026
Version candidate : `0.1.0`
Verdict courant : **`BLOQUÉ_EXTERNE`**

Le périmètre logiciel local du Lot 40 est implémenté et la porte release non signée passe. Le lot
n’est pas clos : l’arbre Git n’est pas encore publié, aucun certificat Authenticode n’est disponible,
la qualification produit finale sur profil Windows propre reste à exécuter et aucune GitHub Release
n’a été autorisée.

## Matrice G0–G6

| Gate | État | Preuve observée |
| --- | --- | --- |
| G0 — provenance propre | **BLOQUÉ** | `main` et `origin/main` pointent sur `acf400a352ac7d43ed30df79fe972fe56163d4de`, avec divergence `0/0`, mais les Lots 36 à 40 sont encore dans un arbre sale. Le manifeste porte donc `dirty: true` et aucun tag n’a été créé. |
| G1 — qualité | **PASS local** | `scripts/verify_release.ps1 -ExpectedVersion 0.1.0` termine par `RELEASE_VERIFICATION_PASS` : audit frontend sans vulnérabilité connue, TypeScript, 27 tests frontend, workspace Rust, Clippy/audits/budgets, 13 tests Python, build Tauri, NSIS et MCP. |
| G2 — distribution | **PASS local** | Manifeste schéma 2, 3 artefacts, 17 SBOM CycloneDX et 20 entrées dans `SHA256SUMS`; `verify_distribution.ps1` termine par `DISTRIBUTION_VERIFICATION_PASS` sur l’arborescence locale puis sur une copie plate simulant le retéléchargement GitHub. Aucun chemin local n’est présent dans les SBOM. |
| G3 — signature | **BLOQUÉ EXTERNE** | Aucun certificat de signature de code avec clé privée n’est présent dans les magasins Windows. Les trois artefacts sont `NotSigned`. Un faux thumbprint est refusé avant mutation et leurs hashes restent inchangés. |
| G4 — moteur | **PASS** | Le témoin écoute sur UDP 5139, l’overlay automatisé sur 5140 et le rejeu client sur 5142. Le client Steam charge `innofthelasthope` : déplacement et limite WOK observés, flamme `plc_t06` PWK franchie sans collision parasite, porte `t_door01` DWK bloquante fermée puis franchie ouverte jusqu’au chargement d’une autre zone, sans crash. Le module source reste intact. |
| G5 — produit | **PARTIEL** | Le portable exact démarre, répond avec le titre `OpenNever Forge`, puis s’arrête proprement; son SHA-256 reste inchangé. Le test d’installation sur profil propre n’est pas exécuté, car une installation utilisateur 0.1.0 existe déjà et ne doit pas être écrasée. Le cycle Toolset du 4 août reste une preuve historique, pas un rejeu de la candidate signée. |
| G6 — publication | **NON AUTORISÉ** | Le workflow manuel protégé est prêt, mais aucun tag, push, brouillon ou artefact distant n’a été créé. |

## Candidate locale

Le manifeste `target/release/release-manifest.json` identifie :

- version `0.1.0`, branche `main`, commit `acf400a352ac7d43ed30df79fe972fe56163d4de` ;
- Node `v22.22.3`, pnpm `11.18.0`, Rust/Cargo `1.97.1`, Tauri CLI `2.11.4` et
  Python `3.13.14` ;
- `opennever-forge-desktop.exe`, 24 824 832 octets,
  SHA-256 `1A34EA116BD85EC14E323E32B7040C35DC882BB685B3EB463D97414F05EED56C` ;
- `opennever-mcp.exe`, 1 782 784 octets,
  SHA-256 `436EE128BB1074945AAC3EF76997FCFCEB78D8280A0AA72A8745AFE72263BB01` ;
- `OpenNever Forge_0.1.0_x64-setup.exe`, 7 887 275 octets,
  SHA-256 `5480D0CB83627E056AF8EA82D73201CDD8B599F4AEED1030798FCA960E18285E`.

Ces empreintes décrivent uniquement la candidate locale non signée et sale. Toute signature ou
reconstruction depuis un commit propre doit produire un nouveau manifeste et de nouvelles
empreintes.

## Qualification NWN réelle

Le harnais a utilisé l’installation demandée :

- serveur `E:\SteamLibrary\steamapps\common\Neverwinter Nights\bin\win32\nwserver.exe`, version
  `89.8193.37-17`, SHA-256
  `98951511AE7D06A251355F12D4F3E4B96269A2414FA3058E641AFD70098690F6` ;
- client lancé par Steam depuis `E:\Jeux\Steam\steamapps\common\Neverwinter Nights`, version
  `89.8193.37-17`, binaire identique à celui de l’installation demandée, SHA-256
  `3B7CB1252E0EDB2CE22D7971F333AADE027039AE30A45B4BC64732C3E6BEC73A` ;
- profil isolé `E:\OpenNever_Forge\.tmp\nwn-runtime-validation` ;
- témoin UDP 5139, overlay automatisé UDP 5140 et rejeu client UDP 5142 à l’écoute ;
- `tin01_o20_01.wok`, `plc_t06.pwk` et `t_door01.dwk` produits et chargés depuis `development` ;
- module source de 594 030 octets inchangé avant/après, SHA-256
  `172C06CD5A2178AF46CC5C2828985EAB65FB5DD68898241333B391AB4FC26019`.

Le journal serveur signale encore `Empty field label while reading: CURRENTGAME:onfvalid/MODULE.ifo`
et `Game Type: Bad Strref`, sans arrêt ni perte d’écoute. Après libération de la clé CD, le client
Steam s’est connecté sur le port isolé 5142 et a chargé `innofthelasthope`. Le personnage se déplace
sur la surface WOK et s’arrête à sa limite non praticable. Les deux flammes `plc_t06`, dont le PWK
ne porte volontairement que ses hooks, restent franchissables sans collision parasite. La porte
verrouillée `t_door01` arrête le personnage dans son état fermé ; une fois ouverte, elle est franchie
et la transition charge une autre zone. Aucun crash, clipping bloquant ou traversée de la porte
fermée n’a été observé. G4 est donc entièrement satisfait.

## Chaîne de distribution livrée

- `scripts/create_sbom.ps1` produit une SBOM frontend CycloneDX 1.7 et seize SBOM Rust CycloneDX
  1.5, puis supprime ses sorties temporaires dans les crates ;
- `scripts/create_release_manifest.ps1` construit le manifeste schéma 2 et `SHA256SUMS` après les
  artefacts et les signatures éventuelles ;
- `scripts/sign_release.ps1` signe et vérifie les exécutables puis l’installeur avec SignTool,
  SHA-256 et horodatage RFC 3161 ;
- `scripts/verify_distribution.ps1` relit une candidate locale ou retéléchargée, contrôle les
  hashes, la version, les SBOM et les signatures exigées ;
- `scripts/verify_release.ps1` orchestre la porte complète en modes local non signé ou release
  propre et signée ;
- `.github/workflows/release.yml` est déclenché manuellement, utilise l’environnement protégé
  `release-signing`, exige un tag préexistant cohérent et ne crée qu’un brouillon sur demande
  explicite avant retéléchargement et nouvelle vérification.

La CI ordinaire construit aussi les SBOM, le manifeste non signé, les checksums et vérifie la
distribution sans déclencher de publication.

## Contrôles de sûreté

- `-RequireClean` refuse l’arbre sale ;
- `-RequireSigned` refuse les trois artefacts non signés ;
- un certificat absent ou inattendu échoue avant toute modification de binaire ;
- aucun certificat, mot de passe, PFX, token ou contenu NWN n’est ajouté au dépôt ;
- aucun fichier suivi n’est supprimé ; les fichiers CycloneDX temporaires sont nettoyés après copie
  dans `target/release/sbom` ;
- le module NWN source reste byte-for-byte intact.

## Conditions de clôture restantes

1. intégrer les Lots 36 à 40 dans un commit propre, puis créer et pousser le tag choisi, uniquement
   après autorisation explicite ;
2. fournir un certificat Authenticode valide et les secrets associés dans l’environnement GitHub
   `release-signing`, puis obtenir trois signatures valides et horodatées ;
3. tester l’installeur signé, le portable et la désinstallation sur un profil Windows propre, puis
   rejouer le cycle Toolset sur cette candidate exacte ;
4. autoriser séparément le tag/push, la création du brouillon, puis la publication après
   retéléchargement et comparaison.

Tant que ces conditions ne sont pas toutes observées, le seul verdict conforme est
`BLOQUÉ_EXTERNE`.
