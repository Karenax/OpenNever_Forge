# ADR 0010 — Orchestrateur agentique local et frontière MCP

- Statut : accepté
- Date : 4 août 2026

## Contexte

Le Lot 25 sait produire un petit `AiChangeSet`, mais la création quasi complète d’un module exige
plusieurs tours, des outils spécialisés, des budgets, des approbations, une reprise après arrêt et
une interface utilisable par des modèles locaux ou distants. Donner au modèle un accès direct aux
fichiers ou à Aurora contournerait les garanties déjà établies.

## Décision

Le cœur est une crate Apache-2.0 `aurora-agent`. Elle contient le contrat de politique, le registre
de capacités, `ModuleBlueprint`, l’état persistant des runs, l’audit et les adaptateurs de protocole.
L’orchestrateur reste dans le processus Rust Tauri et exécute exclusivement des fonctions typées du
domaine. Les sorties du modèle ne sont que des propositions JSON.

L’interface Agent Studio configure tous les budgets, le partage de contexte, l’environnement des
outils, les actions externes et chaque capacité. Responses API, Chat Completions compatible et
Ollama sont des adaptateurs remplaçables ; aucune logique NWN ne dépend d’un fournisseur.
Responses conserve la continuité soit par identifiant de réponse quand le stockage fournisseur est
autorisé, soit par rejeu local complet quand il est désactivé. Les sorties d’outils restent liées à
leur `call_id` ; cet état fait partie du run récupérable, pas du domaine NWN.

MCP est un adaptateur externe stdio, pas le cœur de l’application. Il charge la même politique et
le même registre, et refuse les approbations impossibles à obtenir de façon sûre. Cette séparation
permet à un client MCP de piloter OpenNever sans lui donner un accès plus large que l’interface.
L’adaptateur négocie les révisions MCP courante et précédente et n’entre en phase opérationnelle
qu’après la notification d’initialisation du client.

## Conséquences

- le module source demeure immuable ; toutes les mutations vont dans `EditWorkspace` ;
- les secrets restent éphémères, tandis que décisions, empreintes et résultats sont auditables ;
- les checkpoints et le refus de rejouer un appel interrompu rendent la reprise explicite ;
- l’ajout d’une capacité demande un schéma, un niveau de risque, une politique et une
  implémentation locale testée ;
- les actions externes cumulent autorisation globale, règle de capacité et approbation ;
- MCP et les fournisseurs ne sont jamais des dépendances des parsers ou writers NWN.

## Alternatives rejetées

- accès direct du modèle au système de fichiers ou à un terminal : périmètre impossible à borner ;
- serveur HTTP local comme cœur : surface réseau inutile pour une application desktop locale ;
- MCP comme unique API interne : contrat trop orienté transport pour le domaine transactionnel ;
- logique distincte par fournisseur : politiques et audit divergents.
