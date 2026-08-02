# ADR 0003 — Lecture défensive ERF et GFF

- Statut : accepté
- Date : 2 août 2026

## Contexte

Les modules NWN sont des fichiers binaires non fiables et potentiellement volumineux. Le premier
parcours réel doit inventorier un conteneur MOD puis lire `module.ifo`, sans extraire les autres
ressources et sans dépendre d'une bibliothèque LGPL ou de Rust nightly.

## Décision

Deux crates Rust stables et internes portent les formats :

- `aurora-erf` valide l'en-tête V1.0, les tables, les identifiants, les additions et les bornes avant
  de produire uniquement des métadonnées ;
- `aurora-gff` valide les sections V3.2 et ne lit pour l'instant que le sous-ensemble nécessaire à
  `ModuleInfo` : CExoString, ResRef, CExoLocString et listes de structures.

Une ressource n'est chargée en mémoire qu'à la demande, après une nouvelle vérification de sa plage,
avec une limite de 16 Mio. Le lecteur d'inventaire limite aussi le nombre d'entrées déclaré. Les
erreurs conservent un code stable, la source et l'étape d'import.

## Conséquences

- l'inventaire ERF ne copie jamais les contenus des ressources ;
- `module.ifo` est la seule ressource chargée pendant ce parcours ;
- le lecteur GFF n'est pas encore un inspecteur générique et ne doit pas être présenté comme tel ;
- les chaînes occidentales non UTF-8 utilisent temporairement un repli octet vers Unicode ; les
  codepages dépendantes de la langue seront traitées avec la résolution TLK ;
- toute extraction future passe par le cache et ajoute une protection contre la traversée de chemin.
