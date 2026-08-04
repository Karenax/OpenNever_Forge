# Revue de sortie — Lot 20

Date : 4 août 2026

## Périmètre livré

- lecture des WOK/PWK/DWK binaires et ASCII, y compris les anciens PWK `#MAXDOOR ASCII` sans
  géométrie ;
- conservation des surfaces de faces, variantes géométriques et hooks d'usage ou de porte ;
- writers autonomes `WALKMESH`, `PWKMESH` et `DWKMESH`, avec multimaterial et arbre AABB WOK
  déterministes ;
- validation des bornes, indices, aires nulles, doublons, arêtes non-manifold, orientations et
  transformations non finies ;
- opérations contrôlées déplacer, affecter une surface, découper, supprimer, extruder et souder ;
- aperçu SVG sélectionnable, diagnostics visibles et passage exclusif par la commande Tauri typée ;
- création/remplacement transactionnel dans l'overlay, confirmation obligatoire du remplacement,
  relecture avant staging, undo/redo et déploiement `development` existants.

## Preuves automatisées

- `cargo test --workspace` : 99 tests Rust réussis ;
- `pnpm --dir apps/desktop test:run` : 13 tests UI réussis ;
- `pnpm --dir apps/desktop build` : TypeScript strict et build Vite réussis ;
- corpus local autorisé : 8 WOK, 8 PWK et 8 DWK consécutifs parsés sans erreur ;
- trois ressources réellement référencées par la copie de module ont été importées, transformées
  lorsque leur géométrie le permettait, sérialisées et relues :

| Ressource | Projection relue | SHA-256 généré |
|---|---:|---|
| `tin01_o20_01.wok` | 5 sommets, 4 faces | `93DB1F1C0F182E7E1AD77FE0AD7B62B434006F43FE7AE84E08EAAB603FFD1743` |
| `plc_t06.pwk` | 2 hooks, géométrie volontairement absente | `83B3126D9D579AB76C061ECEF9A2C3FBFC93D3CACB3D881C9FC99F3304668F09` |
| `t_door01.dwk` | 9 sommets, 12 faces, 2 variantes, 4 hooks | `209EB4D8CE840DFF8BBC6D903B1A71E36F8F381CAB0AA145EA9906CE647AD172` |

Le module source conserve l'empreinte
`172C06CD5A2178AF46CC5C2828985EAB65FB5DD68898241333B391AB4FC26019`.

## Contrôle moteur Windows

`scripts/validate_nwn_runtime.ps1` lance d'abord un témoin, génère ensuite les trois overrides dans
un profil isolé, puis relance le même module. Sur la machine de contrôle, les deux lancements
`nwserver.exe` s'arrêtent avant écoute avec `0xC0000005`. Le verdict est donc
`INCONCLUSIVE_ENVIRONMENT`, et non un échec de l'overlay.

Le client NWN:EE 89.8193.37-17 a démarré avec les mêmes ressources dans `development`. Le contrôle
Windows s'est interrompu, comme prévu par sa règle de sécurité, lorsqu'une intervention utilisateur
a été détectée au sélecteur de module. La sélection et le chargement final restent donc une preuve
manuelle à consigner, pas une réussite supposée.

## Verdict

L'implémentation du Lot 20 est complète et couverte par les tests, le corpus et un harnais moteur
reproductible. Le seul élément non conclu est la preuve de chargement en jeu sur ce profil Windows :
le serveur échoue déjà sur le témoin et l'automatisation du client a correctement cédé la main à
l'utilisateur. Aucun défaut de code identifié ne reste ouvert dans le périmètre du lot.
