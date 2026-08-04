# Revue de sortie — Lots 6 à 10

- Date : 4 août 2026
- Portée : narration, carte 2D, assets, manifeste 3D et graphe global
- Mode : lecture seule stricte des ressources NWN

## Résultat par lot

| Lot | État | Preuve |
| --- | --- | --- |
| 6 — journal et factions | Accepté | JRL/FAC typés, textes TLK résolus, étapes finales, matrice de réputation et relations dialogue/script avec confiance |
| 7 — carte 2D | Accepté | ARE/GIT/GIC agrégés, grille orientée, toutes les instances positionnées et provenance affichée |
| 8 — assets | Accepté | lecteur MDL binaire/ASCII Apache-2.0, GLB versionné, supermodèles, références, skins, animations, AABB, textures directes et aperçu PLT |
| 9 — vue 3D | Accepté | SET, blueprints et 2DA résolus par Rust ; tuiles, portes, placeables et créatures composites instanciés en GLB avec budget, modes techniques, picking et surbrillance |
| 10 — graphe et validation | Accepté | modèle Rust indépendant, preuves/confiance, diagnostics, vues ciblées et rapports JSON/HTML autonomes |

La Gate Phase 1 est **satisfaite** : le pipeline du Lot 8 alimente désormais les scènes du Lot 9 sans
seconde logique de priorité. Les assets non supportés restent locaux et visibles ; ils n'empêchent
pas le reste de la zone de s'afficher. L'édition demeure désactivée tant qu'une unité de Phase 2
n'est pas explicitement autorisée.

L'audit de licence écarte les parseurs complets repérés sous GPL-3.0-only. La spécification binaire
CC0 `NWN1MDL.bt` de xoreos a servi de description de format au lecteur interne indépendant
`aurora-mdl` ; aucun code copyleft n'a été lié ou copié dans le dépôt. Le validateur Khronos utilisé
pour les preuves GLB et les bibliothèques Babylon restent sous Apache-2.0.

## Validation réelle du Lot 8

L'analyse de `The Dark Ranger's Treasure` avec les ressources de l'installation locale résout
113 655 ressources. Dans l'échantillon borné de 2 048 assets, 844 MDL sont inspectés : 821 sont
convertibles en GLB, pour 14 896 meshes, 539 648 triangles, 340 skins, 72 walkmeshes et 367 liens de
supermodèles. Les 23 autres modèles conservent un diagnostic local, principalement parce qu'ils ne
portent aucune géométrie prévisualisable.

| Preuve | Résultat | Validation Khronos |
| --- | --- | --- |
| `a_ba` | 46 meshes, 169 animations, cache miss puis hit | 0 erreur, 0 avertissement |
| `a_fa2_coat` | 34 meshes, 5 skins, 64 animations | 0 erreur, 0 avertissement |
| `amp01_a01_01` | 2 meshes dont walkmesh AABB | 0 erreur, 0 avertissement |
| `c_mindalhoon` | références de modèles développées, cycles de parent diagnostiqués et neutralisés | 0 erreur, 0 avertissement |
| `tno01_a01_01` | tuile, 9 meshes | 0 erreur, 0 avertissement |
| `cloak_001.plt` | aperçu PNG local 512 × 512 par couches recolorables | conversion réussie |

Le SHA-256 du module avant et après ces contrôles reste
`172C06CD5A2178AF46CC5C2828985EAB65FB5DD68898241333B391AB4FC26019`.

## Validation réelle du Lot 9

Sur les trois zones de `The Dark Ranger's Treasure`, les 195 tuiles, 7 portes, 73 placeables et la
créature composite sont résolus depuis les données source. Les 120 triggers, encounters, waypoints,
sons et autres objets techniques restent des overlays explicites. Aucun objet nécessitant un modèle
ne tombe en mode dégradé.

Le corpus de scène contient 104 MDL uniques (pièces de créature comprises). Leur conversion produit
104/104 GLB valides pour 4 858 736 octets, sous le budget de 256 Mio. Le chargeur groupe chaque modèle,
l'instancie à toutes ses positions, s'arrête au démontage de la vue et remplace uniquement un échec
local par un marqueur. Le build Tauri embarqué démarre sans serveur de développement ; le contrôle
visuel interactif devra être rejoué sur un bureau Windows déverrouillé, la session de validation
ayant refusé l'injection d'entrée sur l'écran verrouillé.

## Validation du corpus officiel

| Module | Quêtes | Factions | Zones | Tuiles | Instances | Objets de scène | Relations |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Contest of Champions | 0 | 6 | 8 | 413 | 477 | 890 | 35 510 |
| Kingmaker | 17 | 28 | 42 | 2 236 | 2 312 | 4 548 | 52 232 |
| Neverwinter Chess | 0 | 5 | 1 | 36 | 3 | 39 | 34 528 |
| ShadowGuard | 18 | 38 | 25 | 1 060 | 1 347 | 2 407 | 48 611 |
| The Dark Ranger's Treasure | 2 | 5 | 3 | 195 | 201 | 396 | 35 164 |
| The Winds of Eremor | 1 | 6 | 3 | 329 | 596 | 925 | 36 891 |
| To Heir is Human | 2 | 9 | 5 | 552 | 146 | 698 | 35 268 |
| Witch's Wake | 20 | 21 | 29 | 1 195 | 2 766 | 3 961 | 50 803 |
| **Total** | **60** | **118** | **116** | **6 016** | **7 848** | **13 864** | **329 007** |

Les 57 469 ressources GFF découvertes sont toutes ouvertes sans échec. Chaque analyse inspecte de
façon bornée 2 048 assets résolus ; 663 disposent d'un aperçu direct dans ce corpus commun. La
limite ASSET_PROBE_LIMIT est visible et n'est jamais silencieuse.

## Contrats livrés

- **aurora-world** conserve des modèles sérialisables indépendants de React et Babylon.js.
- Les relations **certain**, **probable** et **possible** gardent une ressource et un chemin de preuve.
- Les coordonnées NWN restent dans le modèle 2D ; le manifeste effectue explicitement le passage
  vers les axes de scène.
- La vue de zone Babylon.js consomme uniquement le **SceneManifest**. Le viewer d'assets isolé reçoit
  des GLB et textures déjà résolus par Rust via des commandes binaires bornées.
- SQLite v6 conserve le rapport stable associé à l'empreinte du module.
- Le rapport HTML ne contient ni asset propriétaire ni chemin absolu ; le JSON est trié avant
  sérialisation.
- Les sources NWN ne sont ouvertes qu'en lecture et les modules du corpus restent hors Git.

## Prochaine unité

Définir explicitement la première unité de Phase 2 avant d'autoriser toute écriture : transactions,
compilation NSS → NCS, sauvegarde Toolset vérifiée et déploiement `development` séparé.
