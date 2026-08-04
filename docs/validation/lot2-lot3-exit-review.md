# Revue de sortie — Lots 2 et 3

- Date : 3 août 2026
- Statut : accepté
- Portée : Resource Manager, KEY/BIF, GFF générique, TLK, 2DA et premiers objets métier

## Exigences

| Exigence | État | Preuve |
| --- | --- | --- |
| Clé et provenance stables | Réussie | `ResourceKey`, sources typées, priorité, version sélectionnée et versions masquées |
| MOD/HAK/override/development/patch/KEY-BIF | Réussie | catalogue unifié, ordre explicite et ADR 0005 |
| Lecture et extraction défensives | Réussie | offsets bornés, plafonds, annulation, refus de traversée et cache séparé |
| Recherche paginée | Réussie | commande Tauri bornée à 500 résultats, filtres ResRef/type/source |
| Inspecteur brut | Réussie | GFF, 2DA et TLK structurés à la demande, hash et aperçu hexadécimal sinon |
| GFF générique complet | Réussie | types 0 à 15, structures, listes, ordre des champs et données inconnues conservés |
| TLK et chaînes localisées | Réussie | texte embarqué, `dialog.tlk`, TLK personnalisé, langue, genre, origine et état |
| Gestionnaire 2DA | Réussie | `****`, `DEFAULT`, texte NWN, accès par colonne, versions et comparaison |
| Premiers objets métier | Réussie | module, ARE, instances GIT, données GIC et blueprints prioritaires |
| Persistance SQLite | Réussie | migrations 2 et 3, remplacement atomique du catalogue et baseline des dépendances |
| Build release Windows | Réussie | frontend production, binaire Tauri et installateurs x64 |
| Sources NWN immuables | Réussie | toutes les lectures sont non mutantes et les modules restent hors Git |

## Validation sur le corpus local

Les huit modules officiels présents dans l'installation locale ont été analysés en lecture seule.
Tous les GFF découverts ont été ouverts sans échec et le Resource Manager n'a produit aucun
diagnostic de résolution.

| Module | Ressources sélectionnées | GFF ouverts | 2DA | Zones | Blueprints |
| --- | ---: | ---: | ---: | ---: | ---: |
| Contest of Champions | 113 798 | 6 827 / 6 827 | 601 | 8 | 6 558 |
| Kingmaker | 118 124 | 7 948 / 7 948 | 602 | 42 | 7 470 |
| Neverwinter Chess | 113 917 | 6 750 / 6 750 | 601 | 1 | 6 513 |
| ShadowGuard | 117 272 | 8 035 / 8 035 | 601 | 25 | 7 673 |
| The Dark Ranger's Treasure | 113 655 | 6 761 / 6 761 | 601 | 3 | 6 517 |
| The Winds of Eremor | 113 705 | 6 809 / 6 809 | 601 | 3 | 6 559 |
| To Heir Is Human | 113 689 | 6 761 / 6 761 | 601 | 5 | 6 505 |
| Witch's Wake | 116 357 | 7 578 / 7 578 | 601 | 29 | 7 221 |

Le module de travail copié sous `.tmp/modules` confirme aussi 60 versions masquées sur 113 715
versions cataloguées. Les modules déclarant un HAK ont été résolus avec leur provenance. La fixture
synthétique redistribuable couvre le TLK personnalisé, absent des huit modules officiels.

## Limites explicites

- La langue de l'installation est choisie automatiquement (`en` si présent, sinon la première
  langue trouvée) ; un sélecteur utilisateur pourra être ajouté sans changer le modèle.
- NWSync reste hors du périmètre de ces lots.
- Les entrées BIF de type fixe, rarement utilisées par NWN, sont refusées avec un diagnostic stable
  au lieu d'être interprétées approximativement.
- Les adaptateurs GIT/GIC exposent les inventaires et métriques nécessaires au socle métier ; les
  coordonnées et la composition cartographique détaillées appartiennent au Lot 7.

Ces limites sont visibles et n'entraînent aucune perte silencieuse. Le Lot 4 peut commencer sans
activer la moindre écriture dans les ressources NWN.
