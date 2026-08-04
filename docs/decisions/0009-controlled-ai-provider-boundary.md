# ADR 0009 — Frontière de fournisseur IA contrôlée

- Statut : accepté
- Date : 4 août 2026

## Contexte

Le Lot 25 doit permettre à l’utilisateur de choisir un modèle sans donner à ce modèle un accès
direct aux fichiers NWN, au système de fichiers ou au moteur de commandes. Le réseau est désactivé
par défaut et les modules, scripts et dialogues peuvent contenir des données privées ou
propriétaires. Une réponse de modèle ne constitue jamais une preuve qu’une opération est valide.

## Décision

L’application fournit une passerelle HTTPS compatible avec le format de réponse OpenAI
`choices[0].message.content`. HTTP n’est accepté que pour `localhost`, `127.0.0.1` et `::1`, afin de
prendre en charge les modèles locaux. L’utilisateur saisit l’endpoint et le nom exact du modèle.
La clé éventuelle reste dans l’état mémoire du composant pendant l’appel, n’est pas persistée et est
effacée de l’interface à la fin de la requête.

Trois consentements restent distincts : autoriser l’appel réseau, inclure les métadonnées minimales
du module et inclure le contenu des ressources sélectionnées. Aucune ressource n’est envoyée par
défaut. Au plus huit GFF/NSS peuvent être sélectionnés ; chaque contexte est limité à 64 Kio, la
demande à 16 Kio et la réponse à 1 Mio. Les contenus et les clés ne sont jamais journalisés.

Le fournisseur doit renvoyer un `AiChangeSet` JSON strict. Seules deux commandes sont admises :
`set_field` sur un GFF pris en charge et `replace_text` sur un NSS. Le cœur refuse les autres types,
les propositions vides ou trop grandes, les ResRef invalides et les scripts qui ne se parsèrent pas.
La prévisualisation rejoue les transformations sur les octets courants, sans mutation, puis associe
au lot une empreinte SHA-256. L’application exige une confirmation explicite portant sur cette
empreinte avant d’appliquer les commandes dans l’overlay transactionnel. Chaque commande reste
annulable et le module source demeure immuable.

Une proposition JSON locale peut suivre exactement le même chemin de validation sans aucun appel
réseau. Ce mode fournit également une solution de repli testable lorsque le fournisseur n’est pas
disponible.

## Conséquences

- le fournisseur est remplaçable et n’est jamais une dépendance de lecture des formats NWN ;
- l’utilisateur contrôle précisément les données sortantes ;
- un modèle ne peut ni écrire directement un GFF, ni exécuter une commande système, ni lancer un
  script ;
- les opérations structurées non encore admises devront être ajoutées explicitement au contrat,
  avec validation d’octets, tests et interface de confirmation ;
- la compatibilité d’un fournisseur dépend de son acceptation du format de requête JSON choisi,
  mais l’import local reste disponible.

## Validation

- tests Rust des commandes admises/refusées, de l’empreinte, des préconditions d’octets, de
  l’application et de l’annulation ;
- test UI du réseau désactivé par défaut et du parcours proposition locale → prévisualisation →
  confirmation → application ;
- validation des endpoints, bornes, consentements et réponses avant tout accès au workspace ;
- cycle synthétique sans Aurora : création, édition contrôlée, build, réouverture et vérification de
  l’intégrité de la source.
