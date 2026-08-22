# Asset Export v1

## Objet

`opennever-asset-export@1.0.0` est le format local produit par l’atelier **Exporter des assets**.
Il rassemble un modèle Aurora converti en GLB, les textures résolues et un manifeste vérifiable,
sans modifier les ressources NWN analysées.

## Arborescence

```text
<resref>.asset-export-v1/
├── <resref>.glb
├── manifest.json
└── textures/
    └── <texture>.png
```

Le GLB référence les PNG par des URI relatives. Les noms sont normalisés et les textures sont
dédupliquées. Une ressource non résolue ou non convertible est signalée dans les avertissements du
manifeste au lieu d’être remplacée silencieusement.

## Modèles statiques et animés

Le modèle est résolu avec ses dépendances et supermodels avant conversion. L’exporteur conserve les
clips Aurora sérialisables et leurs pistes de translation, rotation et échelle. Le type annoncé est
fondé sur le nombre d’animations réellement présentes dans le GLB produit :

- `static` : aucun clip GLB exporté ;
- `animated` : au moins un clip GLB exporté.

Le manifeste liste chaque clip, sa durée, ses pistes, ses événements connus et son statut
d’export. Les animations ou contrôleurs non pris en charge restent accompagnés d’un avertissement.

## Manifeste et intégrité

`manifest.json` contient notamment la version de schéma, le ResRef source, le mode statique ou
animé, l’empreinte SHA-256 du MDL résolu, les résumés d’animations et textures, ainsi que la taille
et le SHA-256 de chaque payload. Les dépendances sources sont recontrôlées avant publication afin
de détecter une modification concurrente.

## Sécurité et limites

- la destination est un nouveau dossier absolu, hors des racines NWN protégées ;
- un dossier existant, un lien symbolique ou une jonction est refusé ;
- la génération passe par un dossier temporaire adjacent, puis un renommage atomique ;
- les limites v1 sont de 4 096 textures et 1 Gio de payload ;
- l’export reste `local-only` et `redistribution-prohibited-without-separate-rights`.

La conversion ne crée aucun droit de redistribution. L’utilisateur doit disposer séparément des
droits applicables avant de partager un modèle, une texture ou une animation exportés.
