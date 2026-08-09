# Lot 40 — qualification moteur et publication contrôlée

Statut au 9 août 2026 : **exécuté jusqu’aux limites externes — `BLOQUÉ_EXTERNE`**
Nature : lot de qualification et de distribution, sans nouvelle fonction métier

La chaîne locale est livrée et `RELEASE_VERIFICATION_PASS` est observé. G1, G2 et G4 passent : le
serveur témoin, l’overlay et le client NWN ont réellement chargé les WOK/PWK/DWK et les deux états
de porte sans crash. La clôture reste bloquée par l’arbre Git sale, l’absence de certificat
Authenticode, l’absence de profil Windows propre et l’absence volontaire d’autorisation de
publication. La preuve détaillée est dans `docs/validation/lot40-exit-review.md`.

## 1. Objectif

Transformer la candidate locale issue des Lots 36 à 39 en une livraison Windows traçable :

- construite depuis un commit et une version identifiables, avec un arbre de travail propre ;
- signée avec un certificat de signature de code fourni hors dépôt ;
- installée et testée sur un profil Windows propre ;
- validée dans NWN sur un environnement où le témoin moteur fonctionne réellement ;
- publiable sur GitHub avec manifeste, sommes de contrôle, SBOM et notes de version ;
- toujours strictement non mutante vis-à-vis des modules NWN sources.

Le Lot 40 n'est terminé que lorsque les preuves moteur et distribution sont toutes observées. Un
prérequis externe absent conserve le statut `BLOQUÉ_EXTERNE` ou `NON_OBSERVÉ` ; il ne devient jamais
un succès documentaire.

## 2. Point de départ vérifié

- `scripts/verify_release.ps1` passe localement et produit l'exécutable desktop, le compagnon MCP,
  l'installeur NSIS, 17 SBOM CycloneDX, un manifeste schéma 2 et 20 checksums ;
- les artefacts actuels sont non signés et leur manifeste porte `dirty: true`, car les Lots 36 à 39
  ne sont pas encore publiés dans Git ;
- le cycle Aurora Toolset comparer → synchroniser → compiler → sauvegarder → fermer → rouvrir a été
  observé le 4 août 2026. Il devient un contrôle de non-régression de la candidate finale ;
- l'ancien arrêt `0xC0000005` provenait du dossier de travail du harnais. Après correction, le
  binaire Steam version `89.8193.37-17` atteint l'écoute pour le témoin sur 5139 et l'overlay sur
  5140. Après libération de la clé CD, le client Steam s’est connecté au rejeu isolé sur 5142 : WOK
  praticable et limite bloquante, PWK à hooks seuls non bloquant, puis porte DWK fermée bloquante et
  ouverte franchissable jusqu’au chargement d’une autre zone, sans crash ;
- aucun certificat Authenticode ni aucune autorisation de publication GitHub ne sont présumés.

Les preuves actuelles sont conservées dans `docs/validation/lot36-lot39-exit-review.md` et
`docs/validation/release-closure-2026-08-04.md`.

## 3. Hors périmètre

- nouveaux formats NWN, nouveaux éditeurs ou nouvelles capacités IA/MCP ;
- refonte générale de `App.tsx`, `commands.rs` ou `aurora-edit` hors correction bloquante ;
- téléchargement ou redistribution de contenu NWN propriétaire ;
- contournement des approbations GitHub ou stockage d'un certificat, mot de passe ou token dans le
  dépôt, les logs ou les artefacts ;
- affaiblissement des audits, budgets, validations d'overlay ou protections en lecture seule.

## 4. Prérequis externes

| Prérequis | Preuve exigée avant action | Si absent |
| --- | --- | --- |
| environnement NWN fonctionnel | le `nwserver` témoin atteint l'écoute avec un profil isolé | qualification moteur bloquée |
| copie autorisée du module de test | taille et SHA-256 consignés, original hors destination d'écriture | aucun test utilisateur |
| certificat de signature de code | certificat accessible via un magasin/secret approuvé et chaîne valide | aucune candidate signée |
| horodatage RFC 3161 | URL approuvée et joignable | signature de release bloquée |
| droits GitHub | dépôt, branche, version et environnement de publication confirmés | préparation locale seulement |
| autorisation explicite | confirmation distincte avant tag, push ou GitHub Release | aucune publication distante |

L'acquisition du certificat ne fait pas partie du code du Lot 40 et conditionne encore sa clôture.
La session client NWN requise a été rendue disponible et G4 est désormais observé.

## 5. Unités d'exécution

### 40.0 — gel et provenance

1. inventorier précisément les changements des Lots 36 à 39 et exclure secrets, caches, modules et
   sorties locales ;
2. exécuter la porte complète, puis publier ces changements uniquement sur instruction explicite ;
3. choisir la version de la candidate sans supposer un passage à `1.0.0` ;
4. exiger un commit identifié, une branche synchronisée et `dirty: false` avant la candidate finale ;
5. enregistrer versions Rust, Node, pnpm, Tauri, Windows et outils de signature dans le manifeste.

Porte 40.0 : le même commit propre reconstruit deux fois les mêmes artefacts avant signature, ou
toute divergence est expliquée et bloquante.

### 40.1 — provenance, SBOM et manifeste de distribution

1. étendre le manifeste avec version, commit, tag éventuel, toolchains, architecture, état de
   signature et empreinte du certificat public ;
2. produire une SBOM standard sans chemins locaux, contenu NWN ni secret ;
3. produire un fichier SHA-256 autonome pour tous les artefacts publiables ;
4. contrôler que le manifeste référence uniquement des fichiers issus de la même exécution ;
5. conserver le manifeste actuel de développement distinct du manifeste final signé.

Porte 40.1 : manifeste, SBOM et checksums sont cohérents, relisibles hors du workspace et ne
contiennent aucune donnée propriétaire ou locale sensible.

### 40.2 — signature Authenticode

1. injecter le certificat uniquement depuis le magasin Windows ou l'environnement protégé de CI ;
2. signer d'abord les exécutables embarqués avec SHA-256 et horodatage RFC 3161 ;
3. construire ensuite l'installeur, puis signer l'installeur ;
4. vérifier chaque fichier avec `Get-AuthenticodeSignature` et l'outil de vérification Windows ;
5. recalculer tailles et SHA-256 uniquement après toutes les signatures ;
6. échouer si un artefact est non signé, altéré après signature ou signé par un sujet inattendu.

Porte 40.2 : les trois artefacts ont le statut `Valid`, la chaîne attendue et un horodatage valide.
Le manifeste final porte `signed: true` et les empreintes calculées après signature.

### 40.3 — préflight et qualification moteur

1. étendre le préflight pour consigner version et SHA-256 des exécutables NWN, chemin du profil,
   ports, état d'écoute, événement Windows et journaux bornés/anonymisés ;
2. lancer d'abord un module témoin sans overlay dans un profil isolé ;
3. ne tester l'overlay que si le témoin atteint effectivement l'écoute ;
4. lancer la candidate avec les WOK/PWK/DWK produits, connecter un client NWN et charger la zone ;
5. observer au minimum le déplacement sur WOK, un placeable PWK et les deux états de porte DWK ;
6. comparer les hashes du module source avant et après, puis supprimer uniquement les sorties dont
   le manifeste prouve la propriété.

Porte 40.3 : témoin et overlay atteignent l'écoute, le client charge la zone, les trois familles de
walkmesh sont observées sans crash différencié et le module source est byte-for-byte intact.

### 40.4 — non-régression utilisateur et Toolset

1. tester installation, premier lancement, fermeture et désinstallation sur un profil Windows propre ;
2. tester séparément l'exécutable portable et l'installeur signé ;
3. ouvrir une copie de module, mesurer catalogue froid puis chaud, modifier dans l'overlay, compiler
   NSS → NCS et construire un nouveau MOD ;
4. rejouer le cycle Toolset déjà validé sur une copie jetable de la candidate finale ;
5. rouvrir le résultat dans OpenNever et vérifier l'absence de conflit ou perte inconnue ;
6. inspecter visuellement les parcours principaux et les états d'erreur du binaire exact à publier.

Porte 40.4 : les parcours portable, installé, OpenNever, Toolset et reconstruction passent sans
écriture dans la source ni régression des Lots 0 à 39.

### 40.5 — publication contrôlée

1. préparer les notes de version avec capacités, limites connues et compatibilité ;
2. lancer une publication à blanc qui vérifie noms, version, manifeste, SBOM, checksums et signatures ;
3. créer d'abord une GitHub Release brouillon, uniquement après autorisation explicite ;
4. télécharger à nouveau chaque artefact du brouillon et revérifier signature et SHA-256 ;
5. publier le brouillon et créer/associer le tag uniquement lorsque toutes les portes précédentes
   sont vertes et que la stratégie de tag a été confirmée ;
6. consigner l'URL, le commit, le tag et les empreintes dans la revue de sortie.

Porte 40.5 : la release distante expose exactement les artefacts validés, leurs métadonnées et leurs
limites. Aucune publication automatique n'est autorisée par la seule exécution de tests locaux.

## 6. Matrice d'acceptation

| Gate | Critère bloquant | Preuve attendue |
| --- | --- | --- |
| G0 | provenance propre | commit/version identifiés, `dirty: false`, double build comparé |
| G1 | qualité | `RELEASE_VERIFICATION_PASS`, audits et budgets verts |
| G2 | distribution | SBOM, manifeste final et checksums cohérents |
| G3 | signature | trois signatures Authenticode valides et horodatées |
| G4 | moteur | témoin puis overlay fonctionnels et contrôle client observé |
| G5 | produit | installateur, portable, overlay, build et Toolset sans régression |
| G6 | publication | artefacts retéléchargés identiques et GitHub Release autorisée |

Le verdict final possède seulement trois valeurs : `PASS`, `BLOQUÉ_EXTERNE` ou `FAIL`. Le statut
`PASS` exige G0 à G6. Une release locale prête mais non publiée reste `BLOQUÉ_EXTERNE` si G6 est une
exigence confirmée du lot.

## 7. Artefacts livrés localement

- `scripts/verify_release.ps1` avec modes propre/signé ;
- `scripts/create_release_manifest.ps1` pour le manifeste final et `SHA256SUMS` ;
- `scripts/create_sbom.ps1` pour une SBOM frontend et seize SBOM Rust ;
- `scripts/sign_release.ps1` pour l'ordre de signature et la vérification Authenticode ;
- `scripts/validate_nwn_runtime.ps1` durci avec version, empreinte, profil, ports et témoin
  obligatoire avant overlay ;
- `scripts/verify_distribution.ps1` pour une candidate locale ou retéléchargée ;
- workflow GitHub manuel protégé, sans publication lors des push ordinaires ;
- notes de version `docs/releases/v0.1.0.md` ;
- `docs/validation/lot40-exit-review.md` avec uniquement les preuves réellement observées.

## 8. Ordre critique et stratégie d'arrêt

Ordre recommandé : 40.0 → 40.1 → 40.3 → 40.2 → 40.4 → 40.5. La qualification moteur est placée
avant l'utilisation du certificat afin d'éviter de signer une candidate non acceptable. Un échec
G0/G1 est corrigé dans le dépôt puis toute la chaîne est rejouée. Un échec du témoin NWN arrête G4
comme `BLOQUÉ_EXTERNE`. Un échec de signature ou de retéléchargement interdit la publication.

Une release brouillon peut être supprimée ou remplacée selon l'autorisation donnée, mais aucun tag
public ou artefact publié n'est réécrit implicitement. Toute correction après signature crée une
nouvelle candidate, de nouvelles signatures et de nouvelles empreintes.
