# ADR 0006 — Lecteur MDL Apache et cache GLB versionné

- Statut : accepté
- Date : 4 août 2026

## Contexte

Le Lot 8 exige la lecture des MDL NWN1 binaires et ASCII, des supermodèles, références, skins,
animations et walkmeshes. Les lecteurs complets identifiés sous GPL ne peuvent pas être intégrés à
la distribution Apache-2.0. Le template `NWN1MDL.bt` de xoreos-docs décrit le format sous CC0-1.0.

## Décision

Le workspace fournit `aurora-mdl`, une implémentation Rust stable et indépendante sous Apache-2.0.
Elle lit défensivement les deux représentations MDL, conserve les diagnostics locaux et produit un
GLB 2.0 déterministe. Aucun composant GPL n'est lié, copié, téléchargé par le build ou requis à
l'exécution.

Le service de projet résout chaque MDL et ses dépendances uniquement par le Resource Manager. Le
hash composite inclut le modèle, les supermodèles et les modèles référencés. Le résultat est écrit
atomiquement sous `asset-cache/glb-v<schéma>/<hash>.glb`, avec un manifeste JSON vérifié avant tout
cache hit. Toute modification du format d'export incrémente le numéro de schéma.

Les gros buffers GLB et textures traversent l'IPC Tauri sous forme binaire. Babylon.js ne reçoit ni
chemin arbitraire ni accès direct aux archives NWN. Les sources utilisateur restent immuables.

Les PLT sont décodés en interne vers un PNG de prévisualisation par couches. Cet aperçu préserve la
distinction des dix couches, mais ne prétend pas reproduire les couleurs finales d'une apparence :
celles-ci dépendent des choix du blueprint et seront appliquées dans l'éditeur d'apparence.

Pour le Lot 20, le même périmètre indépendant couvre les identifiants de surface des faces MDL.
Le writer WOK/PWK/DWK produit volontairement les grammaires ASCII autonomes observées dans le
corpus NWN : `WALKMESH` avec arbre AABB et multimaterial, `PWKMESH` avec points d'usage, et
`DWKMESH` avec états fermé/ouvert et points de porte. L'ancien en-tête `#MAXDOOR ASCII` reste lu,
y compris pour les PWK sans géométrie. Le format évite d'introduire un writer binaire AABB avant
de disposer d'une validation moteur concluante. Un import binaire peut être projeté en géométrie
éditable, mais son enregistrement est signalé comme remplacement complet et exige une confirmation.

## Validation

- fixtures synthétiques pour limites, cycles, ASCII commenté, cache et dépendances ;
- corpus local pour MDL binaire, supermodèle, référence, skin, animation, AABB, PWK, DWK et PLT ;
- contrôle des GLB réels avec Khronos `gltf-validator` Apache-2.0 ;
- contrôle avant/après du SHA-256 du module source ;
- build TypeScript et tests Rust/frontend sans écriture NWN.

## Conséquences

- le Lot 8 n'a plus besoin d'un convertisseur externe ni d'un mode dégradé global ;
- les nœuds sans géométrie et fonctionnalités locales non rendues restent visibles par diagnostic ;
- l'assemblage des GLB dans les zones appartient au Lot 9 et reste séparé du parsing d'assets ;
- une future modification de la licence ou l'intégration de code copyleft exige un nouvel ADR.
