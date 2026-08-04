# Clôture locale des Lots 21 à 25 — 4 août 2026

Cette revue rassemble les contrôles manuels et externes exécutés sur la candidate Windows. Elle
complète les revues de lot sans transformer un échec d'environnement en succès produit.

## Candidate Windows

- exécutable portable : `target/release/opennever-forge-desktop.exe`, 22 648 320 octets,
  SHA-256 `8D95BFC27E6A2E2CEBF6B04DB3FB5241B6717C13B8973197AD6ECD2091012A8E` ;
- installateur NSIS : `target/release/bundle/nsis/OpenNever Forge_0.1.0_x64-setup.exe`,
  7 409 777 octets,
  SHA-256 `4BACD7D7753DFA966E797F4EDB11C3F5AA2B01EA8B5B0117E932C2FEA36D6923` ;
- l'exécutable exact de la candidate a ouvert la copie de travail, indexé 113 655 ressources de
  jeu et affiché les panneaux des Lots 22, 23 et 25 sans défaut de disposition bloquant.

## Cycle Aurora Toolset réel

Le module installé `The Dark Ranger's Treasure.mod` est resté byte-for-byte intact : 594 030
octets et SHA-256
`172C06CD5A2178AF46CC5C2828985EAB65FB5DD68898241333B391AB4FC26019`.

Une copie dédiée nommée `OpenNever Closure Test.mod` a été ouverte dans Aurora Toolset. Le cycle
suivant a été observé de bout en bout :

1. comparaison initiale de 132 ressources identiques avec le workspace `temp0` ;
2. modification inoffensive de `buildnumber.nss` dans le workspace Toolset ;
3. détection d'un conflit, choix explicite « Importer du Toolset » et synchronisation d'une
   ressource vers OpenNever ;
4. compilation par F7 dans l'éditeur Aurora : `buildnumber.ncs` passe de 66 à 94 octets et reçoit
   le SHA-256 `713395249435EB9B44914CB9F2D4EAB09AE38863826A18919E10FA9D07E64C58` ;
5. sauvegarde explicite du module dans Aurora, fermeture, puis réouverture ;
6. régénération du workspace `temp0` avec 132 fichiers et présence confirmée du marqueur NSS et
   du NCS compilé.

La copie sauvegardée mesure 594 387 octets et porte le SHA-256
`5FBD73E26B9DE47BAD015E79CE84AD1E83D3B73283F8235BB2064BFC3E7CD4CF`. Ce contrôle clôt
l'acceptation Toolset comparer → synchroniser → compiler → sauvegarder → rouvrir.

## Contrôle NWN réel

Le harnais `scripts/validate_nwn_runtime.ps1` a été rejoué avec la copie sauvegardée sur les deux
installations locales connues, avec des profils utilisateur isolés et des ports distincts :

- `E:\SteamLibrary\steamapps\common\Neverwinter Nights` ;
- `E:\Jeux\Steam\steamapps\common\Neverwinter Nights`.

Dans les deux cas, le témoin et l'overlay WOK/PWK/DWK quittent avant l'écoute UDP avec le même code
Windows `0xC0000005`. Le verdict reste donc `INCONCLUSIVE_ENVIRONMENT`. Les overlays ont bien été
produits et relus, et le SHA-256 de la source n'a pas changé. Une preuve positive de chargement
moteur devra être rejouée sur un profil où le témoin `nwserver` démarre normalement.

## Fournisseur IA local facultatif

Un appel OpenAI-compatible a été tenté vers Ollama local avec le modèle
`gemma4:26b-a4b-it-qat`, uniquement avec un NSS synthétique et sans ressource de module ni clé.
L'appel a dépassé 90 secondes et a été interrompu sans proposition exploitable. Aucun succès de
fournisseur réel n'est revendiqué. Ce contrôle facultatif ne remet pas en cause les preuves
déterministes du pipeline JSON local et de ses barrières de sécurité.

## Conclusion

Les Lots 21 à 25 sont fonctionnellement livrés et la candidate Windows ainsi que le cycle Toolset
réel sont contrôlés. La seule acceptation externe obligatoire encore ouverte est le chargement
moteur sur un environnement NWN où le témoin ne s'arrête pas avec `0xC0000005`.
