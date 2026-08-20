# Guide utilisateur OpenNever Forge

## Principe de travail depuis la refondation d’utilisabilité

OpenNever Forge organise le travail par ateliers spécialisés : Dialogues, Zones, Journal et quêtes,
Factions, Scripts et Blueprints. La liste sert à trouver une ressource, la grande zone centrale à la
comprendre et la modifier, et le panneau contextuel à afficher les propriétés de la sélection. Les
données GFF brutes sont réservées au diagnostic avancé.

Journal/quêtes et Factions sont volontairement séparés : ils utilisent des formats, des gestes et
des validations différents. De même, un blueprint se modifie dans son atelier central, pas dans le
petit inspecteur global. Une modification n’atteint jamais le MOD source : elle est enregistrée dans
l’overlay, puis peut être annulée, validée, construite ou déployée pour test.

Le statut complet et les scénarios bloquants sont décrits dans
[`UX_REFONDATION.md`](UX_REFONDATION.md).

Version du guide : 4 août 2026

## Principes de sécurité

OpenNever Forge n’écrit jamais dans le module `.mod` choisi comme source. Les modifications sont
enregistrées dans un workspace séparé, puis produisent soit un nouveau MOD/HAK, soit des fichiers
dans `development`, soit une synchronisation explicitement confirmée vers un dossier temporaire du
Toolset. La ligne **Source intacte** doit rester positive avant toute construction.

Conservez une copie externe du module original. Les HAK, TLK et ressources propriétaires ne doivent
pas être ajoutés au dépôt Git.

## Ouvrir et modifier un module

1. Choisir le `.mod`, la racine du jeu et le dossier utilisateur NWN.
2. Lancer **Analyser la copie** et attendre la fin de l’inventaire.
   Après cette première analyse, OpenNever sauvegarde localement le résultat complet. Aux ouvertures
   suivantes, il restaure automatiquement l’analyse, la dernière page et le curseur du workspace tant
   que le MOD, ses dépendances, le catalogue du jeu, `development` et `override` n’ont pas changé.
   **Réanalyser maintenant** reste disponible pour forcer explicitement une actualisation.
3. Examiner les dépendances manquantes ou modifiées.
4. Créer l’espace d’édition.
5. Modifier les ressources. Chaque opération est prévisualisée, validée et ajoutée à l’historique
   undo/redo du workspace.
6. Construire un nouveau MOD ou déployer l’overlay dans `development`.

## NWScript : règle compiler puis sauvegarder

Une modification `.nss` ne devient exécutable qu’après compilation en `.ncs`. OpenNever enregistre
le compilateur, les includes transitifs et leurs empreintes. Une compilation devenue obsolète bloque
le build et l’envoi d’un NSS vers le Toolset.

Dans Aurora Toolset, les fichiers visibles dans son dossier temporaire ne sont pas persistants :

1. modifier ou synchroniser le NSS ;
2. compiler pour produire le NCS ;
3. vérifier les diagnostics ;
4. sauvegarder explicitement le module dans le Toolset.

À la prochaine ouverture, Aurora recrée son dossier temporaire depuis la dernière sauvegarde du
module. Fermer sans sauvegarder perd les changements temporaires.

## Synchroniser avec Aurora Toolset

La carte **Workspace temporaire du Toolset** met en œuvre une comparaison à trois états : version
Toolset actuelle, version OpenNever actuelle et baseline du dernier échange réussi.

1. Ouvrir le module dans Aurora et sélectionner son dossier temporaire extrait.
2. Cliquer **Comparer**.
3. Examiner chaque état : identique, modifié côté Toolset, modifié côté OpenNever ou conflit.
4. Choisir **Importer du Toolset** ou **Envoyer vers Toolset** pour chaque différence utile.
5. Les conflits exigent toujours un choix manuel ; ils ne sont jamais fusionnés silencieusement.
6. Cliquer **Synchroniser la sélection**.
7. Compiler les NSS concernés, puis sauvegarder le module dans Aurora.

Avant une écriture ou une suppression côté Toolset, OpenNever conserve l’ancienne version sous
`.opennever-backups/<empreinte>/`. Les SHA-256 présentés lors de la prévisualisation sont revérifiés
au moment de l’application. Si un fichier change entre les deux, l’opération est refusée.

## Test en direct avec `development`

Le dossier `<données utilisateur NWN>/development` prend priorité sur les ressources chargées par
le module. Utiliser **Déployer development** pour tester sans reconstruire ni redémarrer le module.
Le nettoyage OpenNever ne supprime que les fichiers dont l’empreinte correspond encore au manifeste
de déploiement ; un fichier modifié extérieurement est conservé et signalé.

## Builds et profils de test

Un profil fixe le nom du MOD, les HAK, le TLK, la politique d’avertissements et le déploiement
optionnel. **Vérifier ×2** produit deux builds indépendants et compare leurs SHA-256. Les profils
`nwmain.exe` et `nwserver.exe` sont lancés directement, sans shell, avec arguments bornés et journaux
dans le workspace.

## Récupération et migrations

OpenNever restaure automatiquement une transaction interrompue. Lors de l’ouverture d’un ancien
workspace compatible, une copie exacte `workspace.json.v<version>.bak` est créée avant migration.
L’historique et le chemin de sauvegarde sont visibles dans la carte Toolset. Une version future
inconnue est refusée au lieu d’être réécrite. Voir [MIGRATIONS.md](MIGRATIONS.md).

## Assistant IA contrôlé

La carte **Proposition d’opérations** n’effectue aucun appel tant que vous ne cliquez pas sur
**Générer et prévisualiser**. OpenNever ne choisit pas le fournisseur à votre place et ne conserve
aucun secret.

1. Saisir l’URL complète de l’endpoint compatible et le nom exact du modèle. HTTP est réservé aux
   modèles locaux ; un fournisseur distant doit utiliser HTTPS.
2. Saisir la clé éventuelle. Elle est effacée de l’interface dès la fin de l’appel et n’est pas
   écrite dans le workspace.
3. Décrire la modification.
4. Choisir facultativement d’envoyer les métadonnées ou le contenu de la ressource sélectionnée.
   Ces deux choix restent indépendants et désactivés par défaut.
5. Cliquer **Générer et prévisualiser** : ce clic déclenche l’appel au modèle. Examiner ensuite
   chaque précondition. Une proposition refusée
   ne peut pas être appliquée.
6. Confirmer l’empreinte et les opérations. Elles sont alors ajoutées à l’historique et peuvent être
   annulées avec **Annuler**.

Le modèle ne peut modifier que des champs GFF existants ou le texte d’un NSS existant. Il n’accède
ni au système de fichiers, ni aux commandes du système, ni à l’archive source. Pour tester ou
utiliser un adaptateur externe sans réseau, ouvrir **Prévisualiser une proposition JSON locale** et
coller un `AiChangeSet` conforme. Après toute modification NSS, compiler le NCS avant build,
déploiement ou sauvegarde Toolset.

## Créer une carte directement

Dans **Construire → Créateur de cartes**, saisissez le brief, le ResRef et le tileset. OpenNever
résout le SET réel et affiche son nombre de tuiles et son empreinte. Les zones sont limitées à
32×32 tuiles et les ResRef à 16 caractères afin de rester dans le contrat compatible commun à
l’interface, au moteur Rust et aux outils IA.

Pour un PC peu puissant, ouvrez **Générer directement avec une IA**, choisissez une API distante,
entrez son endpoint, le nom exact du modèle et une clé temporaire. Ollama et les API compatibles
locales restent disponibles. La clé n’est pas enregistrée et le champ est effacé après l’appel. Le partage des seuls noms ResRef de
blueprints peut être désactivé ; aucun contenu de module, script, GFF, texture ou chemin local n’est
envoyé. La proposition est toujours revalidée localement, puis doit être prévisualisée avant le
bouton **Créer ARE/GIT/GIC**.

Les identifiants de tuiles sont vérifiés dans le SET, mais les raccords visuels entre tuiles
alternatives ne sont pas encore prouvés. Laissez la liste de variantes vide pour le mode homogène
recommandé, puis testez la carte produite dans NWN avant de la qualifier de jouable.

Un client IA externe peut désormais piloter le même parcours avec `opennever-mcp.exe`. Configurez
dans Agent Studio les dossiers **installation NWN:EE** et **données utilisateur NWN:EE**, puis
accordez explicitement les capacités cartographiques souhaitées. Le parcours recommandé est
`map.context` → `map.preview` → `map.apply` → `map.inspect`. L'IA peut ensuite modifier les tuiles,
hauteurs, scripts de zone, météo, éclairage, audio, neuf catégories d'instances, polygones,
apparitions, transitions et inventaires. Chaque édition exige le SHA-256 ARE ou GIT renvoyé par la
dernière inspection ; un état périmé est refusé sans écriture.

## Agent Studio

Le parcours principal suit quatre étapes visibles : choisir le fournisseur et le modèle, tester la
liaison, contrôler le contexte courant, puis décrire le résultat et créer l’exécution. **Créer
l’exécution ne contacte pas le modèle et ne prétend pas produire un plan** : cette action persiste
seulement l’objectif, le fournisseur et les limites. Le premier appel portant l’objectif commence
avec **Lancer l’agent**.

La sélection courante peut être ajoutée au périmètre avec **Utiliser cette sélection** ; il n’est plus
nécessaire de saisir `resref:type` dans le parcours normal. Les réglages de sécurité, budgets, chemins,
`ModuleBlueprint` et matrice de fonctions sont regroupés sous **Réglages avancés**. Le mode de
proposition ponctuelle est également un mode expert replié sous l’agent principal.

Agent Studio étend l’assistant ponctuel avec des exécutions multi-outils persistées. Commencez avec le
niveau **Assistant** ou **Agent supervisé**. Changer de niveau
charge le preset correspondant sans effacer les chemins, les ressources/zones accordées ni les
hôtes déjà configurés. Une clé saisie dans
la carte est effacée après l’appel et n’est jamais sauvegardée.

Les métadonnées, contenus de ressources, diagnostics, graphe d’architecture et chemins locaux sont
cinq consentements distincts. Les chemins absolus sont masqués dans le contexte du modèle par
défaut, même lorsqu’un résultat d’outil interne en contient.

Avec Responses API, **Stockage de la conversation chez le fournisseur** est désactivé par défaut.
L’application rejoue alors localement l’historique nécessaire. Activez-le seulement si vous
acceptez la conservation distante prévue par votre fournisseur ; la reprise utilise alors les
identifiants de réponse au lieu du rejeu local.
Le plafond **Sortie modèle (tokens)** borne la génération côté fournisseur ; **Réponse (octets)**
reste une seconde barrière locale indépendante.

Pour créer un module complexe, décrivez les critères d’acceptation et fournissez facultativement un
`ModuleBlueprint` JSON. Les scripts créés doivent être compilés avant build, `development`, Toolset
ou lancement. Utilisez les demandes d’approbation affichées dans le journal ; **Arrêter** empêche le
prochain tour ou outil. Les checkpoints permettent de revenir au début d’un lot.

La configuration détaillée, les garanties de reprise et le lancement du serveur MCP sont décrits
dans [AGENT_STUDIO.md](AGENT_STUDIO.md).
