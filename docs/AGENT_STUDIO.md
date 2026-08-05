# Agent Studio et construction agentique

Agent Studio est l’interface de contrôle de l’orchestrateur IA d’OpenNever Forge. Le modèle ne
reçoit jamais un accès direct au MOD, au système de fichiers ou à un shell. Il propose des appels
d’outils typés ; le processus Rust évalue la politique, vérifie les paramètres, crée les
checkpoints, puis passe par le workspace transactionnel existant.

## Niveaux et matrice de capacités

Les six niveaux (`observer`, `advisor`, `assisted`, `supervised`, `autonomous`, `operator`) sont des
présélections chargées intégralement quand le niveau change. Les chemins d’outils, portées accordées
et hôtes autorisés déjà saisis sont conservés. La matrice par capacité reste la règle effective : accès, mode d’approbation,
périmètre et nombre maximal d’appels sont réglables séparément. Les invariants non désactivables
restent l’immuabilité du module source, la validation des ResRef, les préconditions SHA-256, les
writers/parsers locaux et l’absence de shell arbitraire.

Les actions externes (`development`, Toolset et lancement NWN) demandent en plus leur autorisation
globale. Une politique ne peut donc pas les ouvrir par accident avec une seule règle générique.

## Confidentialité et fournisseurs

Le réseau est désactivé par défaut. La liste d’hôtes autorisés, HTTP local, les métadonnées, le
contenu des ressources, les diagnostics, les chemins locaux et le sous-graphe d’architecture sont
indépendants. Les chemins sont expurgés récursivement des résultats renvoyés au modèle tant que leur
consentement reste désactivé. Une
clé de fournisseur est éphémère et n’entre jamais dans le fichier de politique, le run ou l’audit.
Si la conservation est désactivée, l’état indispensable à une approbation ou une reprise reste
disponible pendant le run, mais les requêtes/réponses fournisseur ne sont pas journalisées. À l’état
terminal, l’objectif, le blueprint et les arguments/résultats sont remplacés par des empreintes.

Les adaptateurs normalisent actuellement Responses API, Chat Completions compatible et Ollama. Le
prix d’entrée et de sortie par million de tokens est configurable ; l’exécution s’arrête lorsque le
budget calculé, le nombre de tours, les appels d’outils, la durée, la taille de réponse ou le nombre
de ressources de contexte atteint sa limite.
La limite de tokens de sortie est transmise directement au fournisseur (`max_output_tokens`,
`max_completion_tokens` ou `max_tokens` selon le protocole) et reste distincte de la limite dure en
octets appliquée pendant la lecture réseau.

Pour Responses API, le stockage fournisseur est un choix explicite et désactivé par défaut. Quand
il est actif, chaque réponse est continuée avec son `previous_response_id`. Quand il est désactivé,
OpenNever conserve pendant le run et rejoue les entrées et chaque élément de sortie Responses. Dans
les deux modes, chaque résultat d’outil est renvoyé comme `function_call_output` avec le `call_id`
exact. Cet état de protocole est persisté avant le tour suivant afin qu’une approbation ou un
redémarrage ne casse pas la chaîne. Les adaptateurs Chat compatibles reçoivent un contexte borné
reconstruit depuis le run.

## Construction d’un module

`ModuleBlueprint` décrit l’identité, la zone d’entrée, les zones et connexions, les scripts NSS, les
dialogues, le TLK et les HAK. Sa compilation produit un DAG déterministe : inspection, créations,
compilation de chaque NSS, puis validation. L’application conserve les zones déjà complètes,
refuse un triplet ARE/GIT/GIC partiel, crée les autres ressources dans l’overlay et transforme
`module.ifo` sans supprimer les champs inconnus.

Un NSS créé ou modifié n’est jamais considéré comme livrable sans son NCS. Le chemin du compilateur,
l’installation du jeu et les includes sont définis dans la politique. Build, déploiement
`development`, synchronisation Toolset et lancement NWN appellent tous la validation des
compilations avant d’agir.

## Reprise, annulation et audit

Les politiques et exécutions sont stockées sous `<workspace>/agent`. Chaque événement est aussi
ajouté à un journal JSONL. Avant une écriture réversible, l’orchestrateur persiste un checkpoint
`<run>:<cursor>`. `workspace.undo_batch` restaure exactement ce curseur. Un appel marqué `running`
après un arrêt inattendu n’est jamais rejoué automatiquement : le run s’arrête et demande une
vérification. Le bouton Arrêter positionne un jeton d’annulation contrôlé entre les tours et après
chaque attente réseau.

## MCP local

`opennever-mcp --workspace <dossier>` lance un serveur MCP stdio local. Il charge la même politique
et le même registre de capacités que l’interface. `tools/list` masque les capacités interdites par
la matrice ou la politique de contexte. Les mutations prises en charge passent par le même moteur
de commandes et restent dans l’overlay ; le MOD source n’est jamais ouvert en écriture.

Le serveur expose aussi les ressources MCP `opennever://workspace/snapshot`,
`opennever://agent/policy` et `opennever://agent/capabilities`. Une opération exigeant une
approbation humaine est refusée côté MCP tant qu’aucun mécanisme d’approbation externe sûr n’est
présent.

Le serveur négocie MCP `2025-11-25` et conserve la compatibilité `2025-06-18`. Il exige la séquence
`initialize` puis `notifications/initialized`, limite chaque message entrant à 4 Mio et applique
les budgets globaux de durée et d’appels ainsi que la limite propre à chaque capacité. Les chemins
locaux sont expurgés des ressources et sorties structurées tant que leur partage n’est pas autorisé.

## Limites restant intentionnelles

- aucun shell, terminal ou accès arbitraire aux chemins n’est une capacité ;
- les sorties de build sont bornées aux racines configurées et canonicalisées ;
- les actions Toolset utilisent une comparaison et des empreintes avant synchronisation ;
- l’agent ne sauvegarde pas lui-même un module dans Aurora Toolset : après compilation NCS et
  synchronisation, la sauvegarde explicite dans Aurora reste nécessaire ;
- le registre et les exécuteurs Tauri sont contrôlés par un test d’exhaustivité ; MCP n’expose que
  son sous-ensemble local sûr et refuse les fonctions nécessitant une approbation interactive.
