# Dialogue Export v1

## Objet

`opennever-dialogue-export@1.0.0` est le format local produit par l’atelier **Exporter des
dialogues**. Il préserve la ressource DLG sélectionnée et ajoute deux représentations consultables,
sans modifier le module ni les dépendances NWN.

## Arborescence

```text
<resref>.dialogue-export-v1/
├── <resref>.dlg
├── dialogue.json
├── transcript.md
└── manifest.json
```

- `<resref>.dlg` contient exactement les octets de la révision choisie ;
- `dialogue.json` contient le graphe portable ;
- `transcript.md` déroule l’arbre de conversation pour une lecture humaine ;
- `manifest.json` décrit la révision, les diagnostics, les fichiers, tailles et SHA-256.

Le manifeste inventorie les trois payloads ; sa propre taille est incluse dans la limite totale mais
il ne s’auto-référence pas.

## Choix de la révision

L’exporteur utilise la ressource staged lorsqu’un dialogue est modifié ou créé dans le workspace
ouvert. Sans ressource staged, il utilise le dialogue de l’analyse active. Le badge d’aperçu annonce
explicitement `workspace` ou `analysis`, et l’empreinte affichée correspond au DLG exporté.
Cette empreinte devient une précondition : si la ressource change après l’aperçu, l’export est refusé
jusqu’à son rechargement.

## Structure portable

`dialogue.json` conserve :

- lignes PNJ et réponses joueur, textes localisés, locuteurs, sons, animations, quêtes et scripts ;
- liens, conditions, actions, racines et références partagées ;
- nœuds inaccessibles, cycles, liens cassés et diagnostics ;
- références entrantes sous forme de ResourceKey et chemin de champ.

La structure GFF brute, le chemin du module et les chemins des ressources référentes sont omis. Les
textes `displayText` utilisent la résolution TLK produite pendant l’analyse lorsqu’elle est
disponible. Un texte non résolu reste signalé, sans invention silencieuse.

## Transcript

Le transcript suit les racines et enfants du graphe. Les branches répétées et les cycles sont
marqués, la profondeur d’indentation est bornée et une liste plate sert de repli lorsqu’aucune racine
n’est disponible. Les scripts, références entrantes et diagnostics sont ajoutés dans des sections
séparées.

## Sécurité et limites

- la destination doit être un nouveau dossier absolu hors des racines NWN protégées ;
- les dossiers existants, liens symboliques et jonctions sont refusés ;
- la publication passe par un dossier temporaire adjacent puis un renommage atomique ;
- un DLG source est limité à 64 Mio et le bundle complet à 256 Mio ;
- le résultat reste `local-only` et `not_redistributable_without_separate_rights`.

L’export technique ne crée aucun droit de redistribution sur les dialogues, textes, sons, scripts ou
autres contenus référencés.
