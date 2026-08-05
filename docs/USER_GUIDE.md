# Guide utilisateur OpenNever Forge

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

La carte **Proposition d’opérations** fonctionne hors ligne par défaut. OpenNever ne choisit pas le
fournisseur à votre place et ne conserve aucun secret.

1. Saisir l’URL complète de l’endpoint compatible et le nom exact du modèle. HTTP est réservé aux
   modèles locaux ; un fournisseur distant doit utiliser HTTPS.
2. Saisir la clé éventuelle. Elle est effacée de l’interface dès la fin de l’appel et n’est pas
   écrite dans le workspace.
3. Décrire la modification.
4. Cocher **Autoriser cet appel réseau**. L’envoi des métadonnées et du contenu de la ressource
   sélectionnée sont deux consentements facultatifs distincts.
5. Cliquer **Générer et prévisualiser** puis examiner chaque précondition. Une proposition refusée
   ne peut pas être appliquée.
6. Confirmer l’empreinte et les opérations. Elles sont alors ajoutées à l’historique et peuvent être
   annulées avec **Annuler**.

Le modèle ne peut modifier que des champs GFF existants ou le texte d’un NSS existant. Il n’accède
ni au système de fichiers, ni aux commandes du système, ni à l’archive source. Pour tester ou
utiliser un adaptateur externe sans réseau, ouvrir **Prévisualiser une proposition JSON locale** et
coller un `AiChangeSet` conforme. Après toute modification NSS, compiler le NCS avant build,
déploiement ou sauvegarde Toolset.

## Agent Studio

La carte **Construction agentique** étend l’assistant ponctuel avec des exécutions multi-outils
persistées. Commencez avec le niveau **Assistant** ou **Agent supervisé**, configurez explicitement
le fournisseur et les dossiers d’outils, puis examinez la matrice des fonctions. Changer de niveau
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
