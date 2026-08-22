# Area Migration Bundle v1

`area-migration-bundle@1.0.0` est un format d’échange local, neutre et versionné pour une zone NWN.
Il est produit par OpenNever Forge sans dépendance d’exécution vers un moteur consommateur. Les
fichiers MOD, HAK, TLK et d’installation sont ouverts en lecture seule. L’empreinte du MOD analysé
est revérifiée avant publication atomique ; les ressources effectivement lues conservent leur
provenance sélectionnée/masquée et leur empreinte de contenu lorsqu’elle a été calculée.

## Classification et droits

Le manifeste porte `classification: local_only_proprietary` et
`redistribution: not_redistributable_without_separate_rights`. Convertir ou copier une ressource
dans le bundle ne change pas sa licence. Le bundle doit rester local sauf si l’utilisateur détient
séparément tous les droits nécessaires.

## Arborescence minimale

```text
<resref>.area-migration-v1/
  manifest.json
  area.json
  identity-map.json
  diagnostics.jsonl
  migration-report.json
  assets/
    models/*.glb
    textures/*.png
    source-navigation/*.{wok,pwk,dwk}
```

Les dossiers d’assets peuvent être vides lorsqu’aucune ressource de ce type n’est résolue. Dans ce
cas, un diagnostic explicite décrit l’absence, le format non pris en charge ou le placeholder.

## Coordonnées canoniques

Le bundle utilise un repère main droite Y-up distinct du manifeste de scène historique :

```text
NWN [x, y, z] -> canonique [x, z, -y]
basisRows = [[1,0,0], [0,0,1], [0,-1,0]]
```

Les quarts de tour des tuiles et les bearings d’instances sont convertis en quaternions autour de
l’axe Y. `area.json` conserve la transformée source et la transformée canonique afin de rendre la
conversion vérifiable. La basis a un déterminant positif ; lors de l’écriture GLB, les indices des
faces sont ordonnés explicitement pour adapter la convention de face avant NWN à celle de glTF.

## Identités et déterminisme

Chaque tuile, instance et asset reçoit un identifiant `amv1-<sha256>` calculé avec des parties
préfixées par leur longueur : version du schéma, SHA-256 du module source, ResRef de zone, clé de
ressource et identité d’instance. L’ordre des tableaux, diagnostics, fichiers et provenances est
stable. Aucun horodatage ni chemin de destination n’entre dans le contenu sémantique.

Les noms d’assets sont normalisés et suffixés par une empreinte courte pour éviter les collisions.
Une zone inchangée, le même catalogue et les mêmes dépendances produisent les mêmes octets dans
deux destinations nouvelles.

## Résolution et provenance

Le Resource Manager existant reste l’autorité. Le manifeste indique la politique de priorité, la
version sélectionnée et les versions masquées de chaque ressource de la fermeture. Les chemins
locaux complets ne sont pas publiés : le nom de fichier et une empreinte du chemin suffisent à la
preuve locale. Les dépendances HAK/TLK conservent état, taille et empreinte disponibles.

Les modèles de tuiles viennent du SET indexé ; portes, placeables et créatures utilisent les
références de modèles déjà calculées par l’index du monde. Chaque ResRef de modèle est exporté une
seule fois via `aurora-mdl`. La première texture résolue et convertible de façon sûre devient une
URI glTF relative. Les chemins bornés actuels couvrent PNG direct, TGA vrai-couleur/gris avec ou
sans RLE, PLT et DDS DXT1/DXT5. Tout autre format conserve le facteur de couleur du matériau et
émet un diagnostic de repli.

Les WOK, PWK et DWK disponibles sont copiés octet pour octet dans `source-navigation`. Leur statut
est toujours `preserved-not-converted` et `navigationConverted` reste `false`. Lorsque le lecteur
borné reconnaît le format, leurs identifiants de surface sont indexés dans l’asset ; un échec
d’indexation reste un diagnostic explicite et n’empêche jamais la préservation des octets source.

## Intégrité, atomicité et budgets

Chaque fichier payload est listé dans `manifest.json` avec son chemin relatif, son rôle, sa taille
et son SHA-256. Le manifeste est la racine de confiance : son propre SHA-256 et sa taille sont
retournés par le résultat du job/CLI, car un fichier ne peut contenir l’empreinte de ses propres
octets finaux. Les compteurs de `migration-report.json` couvrent les contenus écrits avant le
rapport et le manifeste ; l’inventaire final du manifeste couvre également le rapport.

L’export est construit dans un dossier temporaire adjacent, vérifié, puis renommé vers une
destination qui ne doit pas déjà exister. Une annulation ou une erreur ne publie pas de dossier
partiel. Les limites v1 sont 50 000 fichiers payload et 4 Gio par bundle ; les décodeurs de textures
conservent en plus leurs limites existantes de dimensions et de pixels.

## Diagnostics et statuts

`diagnostics.jsonl` contient un objet JSON déterministe par ligne. Chaque entrée porte gravité,
phase, code, message, ressource, identité et statut parmi `exact`, `converted`, `approximated`,
`placeholder`, `manual`, `unsupported`, `missing` et `license-blocked`. Les éléments inconnus,
absents ou non pris en charge ne sont jamais omis silencieusement.

Le schéma JSON du manifeste se trouve dans
[`schemas/area-migration-bundle-v1.schema.json`](schemas/area-migration-bundle-v1.schema.json).

## Readiness et politique de complétude

La même readiness est appliquée par l’audit, le service d’export, Tauri et le CLI. Les statuts
`missing`, `unsupported`, `license-blocked`, les erreurs et les dépendances non vérifiées bloquent
la publication. Les statuts `approximated`, `placeholder` et `manual` autorisent un bundle local
avec réserves, mais forcent `complete: false` et restent présents dans le rapport.

Pour les instances GIT, `Bearing` est prioritaire. En son absence, la rotation suit la convention
`yaw = atan2(YOrientation, XOrientation)` ; un vecteur nul ou non fini produit un diagnostic au
lieu d’une rotation inventée. Le fallback texture est déterministe : DDS, TGA, PNG puis PLT.
Chaque candidat invalide ou non convertible est diagnostiqué avant l’essai du suivant ; si tous
échouent, le statut est `unsupported`, tandis qu’une ressource absente reste `missing`.
