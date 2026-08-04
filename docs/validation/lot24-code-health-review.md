# Analyse de santé du code — Lot 24

Date : 4 août 2026

## Refactoring livré

- Le moteur de comparaison Toolset, auparavant limité à un scanner dans `aurora-edit/lib.rs`, est
  isolé dans `aurora-edit/src/sync.rs`. Ses états, plans, baselines et préconditions sont testables
  sans Tauri ni système graphique.
- Le panneau React de synchronisation est isolé dans `components/AuroraSyncPanel.tsx`; `App.tsx`
  conserve uniquement l’orchestration de l’espace de travail.
- Le schéma de workspace possède désormais un chemin de migration explicite et sauvegardé au lieu
  d’une simple réécriture silencieuse du numéro de version.
- La décision de sécurité et les procédures opérateur sont séparées du plan de construction.

## Risques contrôlés

| Risque | Contrôle livré |
|---|---|
| écrasement concurrent Toolset/OpenNever | comparaison à trois états et conflit obligatoire |
| fichier modifié après prévisualisation | seconde comparaison des SHA-256 |
| suppression irréversible côté Toolset | sauvegarde par empreinte avant retrait |
| sortie de racine ou jonction symbolique | chemins relatifs validés et symlinks refusés |
| migration destructive | copie byte-for-byte et refus des versions futures |
| NSS sauvegardé sans NCS exact | validation de provenance de compilation avant envoi |

## Dette non bloquante conservée

- `aurora-edit/src/lib.rs` et `App.tsx` restent volumineux. Une extraction mécanique globale pendant
  la clôture fonctionnelle augmenterait le risque de régression ; les nouvelles fonctions des Lots
  23 et 24 sont néanmoins déjà dans des modules séparés.
- Le bundle Babylon/Monaco reste volumineux et Vite signale des chunks supérieurs à 500 kB. Ce
  point affecte le temps de chargement, pas l’intégrité des modules, et devra être traité par
  chargement différé après la verticale IA.
- La synchronisation multi-ressource est atomique par ressource, mais pas comme transaction unique
  entre deux systèmes de fichiers. Toutes les actions sont prévalidées avant le premier changement
  et chaque cible Toolset possède sa sauvegarde ; le journal permet d’identifier les imports déjà
  appliqués si une panne I/O interrompt un lot.

Ces points sont visibles et ne sont pas présentés comme des preuves de validation moteur. La porte
de sortie du Lot 24 concerne la compatibilité du projet, la documentation et l’isolation des nouveaux
flux ; le chargement NWN positif reste un contrôle externe distinct.
