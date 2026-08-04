# Revue de sortie — Lot 25

Date : 4 août 2026

## Verdict

Le Lot 25 est **terminé pour son périmètre logiciel**. L’assistant contacte uniquement un endpoint
compatible choisi par l’utilisateur après consentement explicite. Une réponse n’est jamais
appliquée directement : elle est décodée en commandes bornées, validée contre les octets courants,
prévisualisée, liée à une empreinte SHA-256 puis confirmée par l’utilisateur.

## Parcours livré

1. Le réseau est désactivé par défaut.
2. L’utilisateur choisit l’endpoint, le modèle et, si nécessaire, une clé temporaire.
3. Il choisit séparément l’envoi des métadonnées et du contenu d’une ressource GFF/NSS.
4. La passerelle limite les entrées, le délai et la taille de réponse.
5. Le cœur accepte uniquement `set_field` et `replace_text` NSS.
6. Chaque opération est rejouée en mémoire sur la version courante de la ressource.
7. L’interface affiche chaque précondition et bloque une proposition invalide.
8. La confirmation porte sur le SHA-256 exact de la proposition.
9. L’application stage les octets dans l’overlay et ajoute les commandes à l’historique undo/redo.

Le même parcours est disponible pour une proposition JSON locale, sans réseau.

## Garanties vérifiées

- aucune clé, endpoint, préférence IA ou contenu envoyé n’est persisté dans le workspace ;
- aucune donnée n’est transmise sans consentement ;
- HTTP distant, URL avec identifiants et réponse Markdown sont refusés ;
- la réponse est limitée à 1 Mio et le lot à 32 commandes ;
- les commandes non autorisées sont rejetées avant résolution ou écriture ;
- les valeurs GFF `before` et le texte NSS `before` doivent correspondre aux octets courants ;
- le NSS proposé doit être parsable ;
- la source MOD reste byte-for-byte intacte.

## Preuves automatisées

- `cargo test --workspace` : 115 tests Rust réussis, dont la validation, l’application,
  l’annulation et le cycle complet création → édition → build → réouverture sans Aurora ;
- `pnpm --filter @opennever/desktop test:run` : 17 tests UI réussis, dont l’état offline initial et
  l’application d’une proposition locale validée ;
- `python -m unittest discover -s tests` : 12 tests réussis ;
- `pnpm --filter @opennever/desktop build` et le build Tauri release couvrent la verticale Windows ;
- le graphe d’architecture est régénéré puis contrôlé.

La candidate Windows produite est :

- portable : `target/release/opennever-forge-desktop.exe`, 22 648 320 octets,
  SHA-256 `8D95BFC27E6A2E2CEBF6B04DB3FB5241B6717C13B8973197AD6ECD2091012A8E` ;
- installateur NSIS : `target/release/bundle/nsis/OpenNever Forge_0.1.0_x64-setup.exe`,
  7 409 777 octets,
  SHA-256 `4BACD7D7753DFA966E797F4EDB11C3F5AA2B01EA8B5B0117E932C2FEA36D6923`.

L’exécutable portable démarre et reste actif pendant le contrôle de fumée de cinq secondes. Le
harnais NWN a été rejoué : témoin et overlay quittent encore avant écoute avec `0xC0000005`, donc le
verdict reste `INCONCLUSIVE_ENVIRONMENT` et ne devient pas une réussite inventée.

## Limites assumées

- la passerelle vise les endpoints compatibles avec la structure OpenAI sélectionnée ; elle ne
  stocke pas de profils ni de secrets ;
- le Lot 25 n’accorde volontairement à l’IA que l’édition de champs GFF existants et de NSS
  existants. Les créations structurelles restent des actions manuelles tant que leur contrat dédié
  n’a pas été ajouté ;
- un essai OpenAI-compatible auprès d'Ollama local avec `gemma4:26b-a4b-it-qat` a dépassé 90
  secondes sans réponse exploitable. Aucun succès auprès d'un fournisseur réel n'est donc
  revendiqué. Le mode local JSON constitue la preuve déterministe du pipeline.

## État du plan

Tous les Lots 0 à 25 disposent désormais d’une implémentation et de preuves locales. La candidate
Windows et le cycle Toolset réel compiler → sauvegarder → rouvrir ont été contrôlés le 4 août 2026.
Le contrôle restant est une acceptation externe : chargement manuel dans un environnement NWN où
le témoin moteur démarre. Il ne correspond plus à un lot de développement manquant et ne doit pas
être présenté comme réussi tant qu'il n'a pas été observé.
