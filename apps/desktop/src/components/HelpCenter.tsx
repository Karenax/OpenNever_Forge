import {
  Archive,
  BookOpenCheck,
  Bot,
  Boxes,
  CheckCircle2,
  Code2,
  Compass,
  Hammer,
  Map,
  Play,
  Search,
  ScrollText,
  Settings2,
  ShieldCheck,
  Sparkles,
  TriangleAlert,
  Workflow,
} from "lucide-react";
import { useMemo, useState, type ComponentType } from "react";
import fullManualHtml from "../../../../docs/OpenNever_Forge_Manuel_Complet.html?raw";

type HelpCenterProps = {
  hasModule: boolean;
  hasWorkspace: boolean;
  onNavigate: (view: string) => void;
};

type GuideStep = {
  title: string;
  instruction: string;
  control?: string;
  expected: string;
};

type GuideTopic = {
  id: string;
  category: string;
  title: string;
  summary: string;
  icon: ComponentType<{ size?: number; strokeWidth?: number }>;
  prerequisites: string[];
  steps: GuideStep[];
  outcome: string;
  warning?: string;
  troubleshooting: { problem: string; resolution: string }[];
  action: { label: string; view: string };
};

const topics: GuideTopic[] = [
  {
    id: "install",
    category: "Premiers pas",
    title: "Renseigner les chemins NWN",
    summary: "Identifier le MOD, l’installation du jeu et le dossier utilisateur sans confondre leurs rôles.",
    icon: Settings2,
    prerequisites: ["Neverwinter Nights: Enhanced Edition est installé.", "Vous disposez d’une copie du fichier .mod à étudier."],
    steps: [
      { title: "Choisir le module", instruction: "Sélectionnez une copie de travail du fichier .mod. Conservez votre original ailleurs : l’analyse reste en lecture seule, mais une copie évite toute confusion ultérieure.", control: "Parcourir — Module NWN", expected: "Le chemin absolu du .mod apparaît dans le premier champ." },
      { title: "Localiser l’installation", instruction: "Choisissez le dossier qui contient l’installation NWN:EE. Selon Steam, il se termine généralement par Neverwinter Nights et contient bin, data et lang.", control: "Parcourir — Installation du jeu", expected: "OpenNever peut résoudre les ressources de base et les outils du jeu." },
      { title: "Localiser les données utilisateur", instruction: "Choisissez le dossier Documents/Neverwinter Nights utilisé par le jeu. Il contient notamment development, hak, modules, override et tlk.", control: "Parcourir — Données utilisateur", expected: "Les contenus personnalisés et la destination development deviennent accessibles." },
      { title: "Contrôler avant analyse", instruction: "Vérifiez que les trois chemins correspondent à trois emplacements distincts. Ils sont mémorisés localement et restaurés au prochain démarrage.", expected: "Le bouton d’analyse est activé et les chemins restent visibles après relance." },
    ],
    outcome: "OpenNever connaît la source à analyser, les données officielles à résoudre et l’emplacement utilisateur où tester.",
    troubleshooting: [
      { problem: "Le bouton d’analyse reste désactivé.", resolution: "Un des trois champs est vide. Renseignez le .mod, l’installation du jeu et le dossier utilisateur." },
      { problem: "Les HAK ou TLK ne sont pas trouvés.", resolution: "Vérifiez que Données utilisateur pointe sur le dossier Neverwinter Nights lui-même, pas sur son sous-dossier modules." },
    ],
    action: { label: "Ouvrir la table de campagne", view: "module" },
  },
  {
    id: "start",
    category: "Premiers pas",
    title: "Analyser un module existant",
    summary: "Indexer le MOD et vérifier ses dépendances avant toute modification.",
    icon: Compass,
    prerequisites: ["Les trois chemins NWN sont renseignés.", "Le fichier .mod sélectionné est une copie de travail."],
    steps: [
      { title: "Lancer l’indexation", instruction: "Démarrez la lecture de l’index ERF. Cette étape inventorie les ressources sans les extraire ni modifier le fichier source.", control: "Analyser la copie", expected: "La progression atteint 100 % et l’état indique que l’analyse est terminée." },
      { title: "Reprendre automatiquement", instruction: "Après une analyse réussie, fermez puis rouvrez OpenNever. Le résultat complet, le workspace et sa révision sont restaurés si le MOD, ses dépendances et les couches de ressources n’ont pas changé.", expected: "L’état indique Session restaurée et aucune nouvelle analyse ne démarre." },
      { title: "Lire l’inventaire", instruction: "Contrôlez le nombre de zones, dialogues, scripts, blueprints et ressources. Une valeur nulle inattendue signale un mauvais fichier ou une lecture incomplète.", expected: "Les catégories de l’explorateur affichent des compteurs cohérents." },
      { title: "Vérifier les dépendances", instruction: "Consultez les HAK, le TLK et les 2DA déclarés. Une dépendance manquante peut rendre l’affichage ou la compilation incomplets.", expected: "Chaque dépendance indique sa provenance ou un diagnostic exploitable." },
      { title: "Créer l’atelier", instruction: "Après l’analyse, créez l’espace transactionnel. Les futures écritures iront dans cet atelier séparé, jamais dans le MOD source.", control: "Créer l’espace d’édition", expected: "L’état Atelier créé passe à Prêt et les fonctions d’édition s’activent." },
    ],
    outcome: "Le module est indexé, sa provenance est connue et un atelier isolé est prêt pour les changements.",
    warning: "Ne remplacez jamais votre unique MOD original par un fichier produit pendant les essais.",
    troubleshooting: [
      { problem: "L’analyse échoue immédiatement.", resolution: "Vérifiez que le fichier existe, porte l’extension .mod et n’est pas verrouillé par une copie ou un téléchargement incomplet." },
      { problem: "La session n’est pas restaurée.", resolution: "Le cache est absent, invalide, ou une source surveillée a changé. Utilisez Analyser la copie ; le nouveau résultat remplacera ensuite le cache local." },
      { problem: "Créer l’espace d’édition n’apparaît pas.", resolution: "Attendez la fin de l’analyse et corrigez d’abord toute erreur bloquante affichée dans Diagnostics." },
    ],
    action: { label: "Analyser ou préparer le module", view: "module" },
  },
  {
    id: "create",
    category: "Premiers pas",
    title: "Créer un module vide",
    summary: "Produire une base minimale avec une zone d’entrée exploitable.",
    icon: Archive,
    prerequisites: ["L’installation du jeu et les données utilisateur sont renseignées.", "Vous avez choisi un dossier de sortie distinct de vos originaux."],
    steps: [
      { title: "Ouvrir le formulaire", instruction: "Dans la Table de campagne, développez la carte Créer un module vide.", control: "Configurer", expected: "Les champs Nom, Tag, Zone d’entrée et Tileset sont visibles." },
      { title: "Définir les identifiants", instruction: "Saisissez un nom lisible. Utilisez pour le tag et le ResRef de zone des identifiants stables, courts, sans espace ; le ResRef est limité à 16 caractères.", expected: "Les identifiants ne contiennent ni espace ni caractère ambigu." },
      { title: "Choisir le tileset", instruction: "Indiquez le ResRef exact d’un tileset disponible dans le jeu ou les HAK chargés. Le choix détermine les tuiles utilisables dans la zone initiale.", expected: "Le tileset est résolu sans diagnostic de ressource manquante." },
      { title: "Créer puis analyser", instruction: "Générez le MOD, sélectionnez-le ensuite comme Module NWN et lancez son analyse comme pour un module existant.", control: "Créer le nouveau MOD", expected: "Un .mod est créé et peut être ouvert dans OpenNever ou Aurora Toolset." },
    ],
    outcome: "Vous obtenez un MOD minimal, identifiable et prêt à recevoir du contenu dans un atelier.",
    troubleshooting: [
      { problem: "La création refuse le nom de zone.", resolution: "Réduisez le ResRef à 16 caractères ASCII, en minuscules, sans espace." },
      { problem: "La zone s’ouvre sans tuiles correctes.", resolution: "Corrigez le ResRef du tileset et recréez la base ; un nom d’affichage n’est pas un ResRef de tileset." },
    ],
    action: { label: "Configurer un nouveau module", view: "module" },
  },
  {
    id: "understand",
    category: "Explorer",
    title: "Retrouver et comprendre une ressource",
    summary: "Rechercher par ResRef, lire la provenance et suivre les références.",
    icon: Boxes,
    prerequisites: ["Un module a été analysé."],
    steps: [
      { title: "Rechercher", instruction: "Ouvrez Ressources et saisissez tout ou partie du ResRef, du type ou du nom connu. Filtrez le type pour réduire les homonymes.", expected: "La liste ne conserve que les ressources correspondant aux critères." },
      { title: "Lire la provenance", instruction: "Sélectionnez une ressource puis consultez l’inspecteur. La couche gagnante peut venir du workspace, de development, override, d’un HAK, du MOD ou du jeu de base.", expected: "La source effective et les éventuelles versions masquées sont identifiées." },
      { title: "Inspecter le contenu", instruction: "Ouvrez la vue structurée pour les GFF ou la source pour les NSS. Les champs inconnus restent visibles afin d’éviter une perte silencieuse lors d’une future édition.", expected: "Vous distinguez les valeurs métier, les données brutes et leur origine." },
      { title: "Suivre les liens", instruction: "Ouvrez Références pour repérer qui utilise cette ressource et ce qu’elle utilise elle-même avant de la renommer ou de la supprimer.", expected: "Les impacts entrants et sortants sont connus avant modification." },
    ],
    outcome: "Vous savez quelle version est réellement chargée par NWN et quelles ressources dépendraient d’un changement.",
    troubleshooting: [
      { problem: "Deux ressources portent le même ResRef.", resolution: "Comparez leur type et leur couche. NWN résout la priorité des couches ; la ligne gagnante doit correspondre à votre intention." },
      { problem: "Une référence semble absente.", resolution: "Relancez l’analyse après un changement externe et vérifiez que le HAK ou TLK concerné est bien déclaré dans module.ifo." },
    ],
    action: { label: "Explorer les ressources", view: "resources" },
  },
  {
    id: "workspace",
    category: "Éditer",
    title: "Comprendre l’atelier transactionnel",
    summary: "Modifier, annuler et contrôler les fichiers produits sans toucher au MOD source.",
    icon: Workflow,
    prerequisites: ["Un module est analysé.", "L’espace d’édition a été créé."],
    steps: [
      { title: "Éditer dans l’overlay", instruction: "Toute modification validée produit une ressource dans l’atelier. Le fichier source reste figé et sert de référence pour les comparaisons.", expected: "La ressource modifiée apparaît avec une provenance workspace." },
      { title: "Contrôler le journal", instruction: "Après chaque opération importante, consultez le journal et les diagnostics. Le journal décrit la commande, la ressource cible et le résultat.", expected: "Chaque modification possède une trace explicite et datée." },
      { title: "Annuler ou rétablir", instruction: "Utilisez Annuler/Rétablir pour revenir sur les commandes transactionnelles. Vérifiez ensuite la ressource concernée, pas seulement le compteur global.", expected: "Le curseur de commandes et l’aperçu reviennent à l’état attendu." },
      { title: "Valider avant construction", instruction: "Résolvez les erreurs bloquantes puis exécutez les validations adaptées au type modifié.", expected: "Aucun diagnostic bloquant ne subsiste avant Construire." },
    ],
    outcome: "Vos changements sont traçables, réversibles et séparés de la source jusqu’à la construction finale.",
    warning: "Le workspace n’est pas un MOD jouable : utilisez Construire pour produire un conteneur, ou development pour un test live.",
    troubleshooting: [
      { problem: "Une modification n’apparaît pas dans le jeu.", resolution: "Un changement présent uniquement dans le workspace doit être construit ou déployé dans development avant le test." },
      { problem: "Annuler est désactivé.", resolution: "Aucune commande transactionnelle n’est disponible avant le curseur courant, ou l’opération n’a pas encore été enregistrée." },
    ],
    action: { label: "Voir l’état du projet", view: "module" },
  },
  {
    id: "world",
    category: "Éditer",
    title: "Créer une zone et placer une instance",
    summary: "Construire le trio ARE/GIT/GIC puis ajouter des objets avec des coordonnées contrôlées.",
    icon: Map,
    prerequisites: ["L’atelier transactionnel est créé.", "Le tileset et le blueprint à utiliser sont disponibles."],
    steps: [
      { title: "Créer la zone", instruction: "Ouvrez Zones, développez Nouvelle zone et renseignez un ResRef de 16 caractères maximum, un nom, le tileset, la largeur et la hauteur.", control: "+ Nouvelle zone → Créer ARE/GIT/GIC", expected: "Trois ressources de même ResRef sont créées : .are, .git et .gic." },
      { title: "Vérifier la grille", instruction: "Sélectionnez la zone et contrôlez les dimensions, les tuiles et leurs orientations dans la vue 2D avant de placer du contenu.", expected: "La grille correspond aux dimensions demandées et aucun diagnostic de tuile n’est bloquant." },
      { title: "Choisir dans la palette", instruction: "Cliquez sur Ouvrir la palette, choisissez une catégorie puis recherchez le blueprint par ResRef. La liste provient du Resource Manager et montre la couche réellement chargée.", control: "+ Ouvrir la palette", expected: "Le blueprint choisi apparaît dans le résumé de placement avec sa catégorie métier." },
      { title: "Placer puis déplacer", instruction: "Ajoutez l’instance à sa position initiale, puis faites glisser son marqueur sur la carte. Le panneau de droite reflète les coordonnées enregistrées dans l’overlay.", control: "3 · Ajouter à la zone", expected: "Le marqueur et les coordonnées décrivent la même position ; Annuler restaure la précédente." },
      { title: "Contrôler le déplacement", instruction: "Inspectez le walkmesh et ses diagnostics. Corrigez les surfaces non marchables, transitions ou portes incohérentes avant un test en jeu.", expected: "Le parcours prévu est praticable et les zones interdites restent bloquées." },
    ],
    outcome: "La zone possède ses trois ressources cohérentes, son contenu placé et un walkmesh contrôlé.",
    troubleshooting: [
      { problem: "Le blueprint n’apparaît pas après placement.", resolution: "Vérifiez son ResRef, sa catégorie et sa provenance ; un blueprint absent du MOD/HAK/jeu ne peut pas être résolu." },
      { problem: "L’objet flotte ou traverse le sol.", resolution: "Ajustez Z et contrôlez le walkmesh. Les coordonnées d’instance ne recalculent pas automatiquement la hauteur de surface." },
    ],
    action: { label: "Ouvrir l’atelier du monde", view: "areas" },
  },
  {
    id: "map-vibecoding",
    category: "Construire",
    title: "Vibecoder une carte déterministe",
    summary: "Transformer un brief en plan reproductible, appliquer ARE/GIT/GIC et exporter une carte de repérage.",
    icon: Sparkles,
    prerequisites: ["Un module est analysé.", "L’atelier transactionnel est créé pour appliquer la carte.", "Les ResRef des blueprints à placer sont résolus par le module, ses HAK ou le jeu."],
    steps: [
      { title: "Décrire la carte", instruction: "Dans Créateur de cartes, décrivez l’ambiance, la circulation, les lieux importants et la population attendue. Le texte sert d’intention ; il n’écrit jamais directement dans les fichiers NWN.", control: "Brief de la carte", expected: "Le brief est assez précis pour distinguer décor, rencontres, accès et espaces libres." },
      { title: "Verrouiller les contraintes", instruction: "Choisissez dimensions, tileset, tuile de base, graine, marge et réserve libre. Une même configuration et une même graine doivent toujours donner le même plan.", expected: "Le pourcentage réservé laisse des zones de circulation et la marge n’occupe pas toute la carte." },
      { title: "Régler les densités", instruction: "Pour chaque catégorie, indiquez un nombre cible par cent tuiles, l’espacement minimal et les ResRef autorisés. Une catégorie sans blueprint est ignorée et signalée.", expected: "Les compteurs du plan restent sous les limites et aucun blueprint requis n’est introuvable." },
      { title: "Prévisualiser puis appliquer", instruction: "Calculez le plan, relisez ses métriques et avertissements, puis créez le lot ARE/GIT/GIC. L’application vérifie l’empreinte exacte du plan et crée une seule commande annulable.", control: "Prévisualiser → Créer ARE/GIT/GIC", expected: "La zone est relue depuis l’overlay et apparaît dans l’atlas du module." },
      { title: "Exporter le repère", instruction: "Sélectionnez n’importe quelle zone dans l’atlas puis exportez SVG pour une version éditable ou PNG pour une image immédiatement partageable.", control: "SVG ou PNG", expected: "L’image indique le nord, la grille, les identifiants de tuiles et les instances principales." },
      { title: "Confier les détails au modèle", instruction: "Utilisez Confier le brief à l’Agent pour préremplir Agent Studio. Le modèle doit rechercher des blueprints résolus, ajuster le contrat puis appeler map.generate sous le profil d’approbation choisi.", control: "Confier le brief à l’Agent", expected: "L’objectif contient le brief, le contrat initial et la demande explicite de validation." },
    ],
    outcome: "Vous obtenez une zone NWN reproductible, annulable et accompagnée d’une carte de repérage dérivée des ressources réellement relues.",
    warning: "Les tuiles alternatives sont distribuées de manière déterministe, mais leur compatibilité visuelle dépend du fichier SET du tileset et doit encore être contrôlée dans la vue 3D puis dans NWN.",
    troubleshooting: [
      { problem: "La densité demandée n’est pas atteinte.", resolution: "Réduisez l’espacement, la marge ou la réserve libre. Le moteur privilégie les contraintes de circulation à la quantité brute." },
      { problem: "L’application refuse un blueprint.", resolution: "Corrigez son ResRef et vérifiez sa catégorie dans Ressources. Le plan n’invente jamais un blueprint absent du Resource Manager." },
    ],
    action: { label: "Ouvrir le créateur de cartes", view: "map_creator" },
  },
  {
    id: "narrative",
    category: "Éditer",
    title: "Structurer le journal d’une quête",
    summary: "Créer des étapes lisibles et des états finaux dans l’atelier Journal et quêtes.",
    icon: ScrollText,
    prerequisites: ["L’atelier transactionnel est créé.", "Le déroulé et les états finaux de la quête sont définis."],
    steps: [
      { title: "Ouvrir le bon atelier", instruction: "Choisissez Journal et quêtes dans la section Récit. Les factions possèdent désormais leur propre atelier et ne réduisent plus la largeur du journal.", expected: "La liste des quêtes est à gauche et une seule quête s’ouvre au centre." },
      { title: "Créer ou sélectionner la quête", instruction: "Utilisez + Catégorie pour une nouvelle quête, ou sélectionnez une catégorie existante par son nom. Le tag, la priorité et les XP se modifient dans la fiche centrale.", control: "+ Catégorie", expected: "La quête sélectionnée reste visible pendant l’édition de ses étapes." },
      { title: "Éditer les étapes", instruction: "Dépliez uniquement l’étape utile, modifiez son texte localisé, son identifiant et son état final, puis appliquez la valeur.", expected: "Les autres étapes restent repliées et le texte conserve une largeur de lecture normale." },
      { title: "Contrôler l’historique", instruction: "Après ajout ou suppression, utilisez Annuler puis Rétablir depuis Construire et revenez au journal pour contrôler le résultat.", expected: "La quête revient exactement à l’état précédent puis à l’état modifié." },
    ],
    outcome: "La quête possède un chemin compréhensible, des états persistants et des références valides.",
    troubleshooting: [
      { problem: "Une étape n’apparaît pas dans la liste.", resolution: "Vérifiez le filtre de la vue, sélectionnez la bonne catégorie et contrôlez que l’ajout a bien produit une commande dans l’overlay." },
      { problem: "La quête reste ouverte après la fin.", resolution: "Ajoutez une entrée de journal terminale et déclenchez-la sur toutes les branches de réussite ou d’échec." },
    ],
    action: { label: "Ouvrir Journal et quêtes", view: "journal" },
  },
  {
    id: "dialogues",
    category: "Éditer",
    title: "Modifier un dialogue volumineux",
    summary: "Modifier le texte, les réponses et leurs déclencheurs comme un document, sans dépendre du graphe.",
    icon: ScrollText,
    prerequisites: ["Le module est analysé.", "L’atelier transactionnel est requis pour enregistrer."],
    steps: [
      { title: "Trouver le DLG", instruction: "Ouvrez Dialogues et utilisez le filtre global pour réduire la liste. Sélectionnez le ResRef voulu dans la colonne gauche.", expected: "La vue Lignes s’ouvre avec le début du dialogue, les répliques PNJ et les réponses joueur." },
      { title: "Rechercher la réplique", instruction: "Dans Rechercher dans les lignes, saisissez une phrase, un locuteur, un commentaire ou un script.", control: "Rechercher dans les lignes", expected: "Seules les lignes correspondantes restent affichées, sans réduire leur texte." },
      { title: "Modifier et organiser", instruction: "Modifiez directement le texte d’une ligne. Utilisez + Réplique PNJ, + Réponse joueur, Supprimer la ligne et les boutons Associer pour construire le dialogue.", expected: "Chaque ligne et ses réponses restent regroupées au même endroit." },
      { title: "Associer les déclencheurs", instruction: "Sous une réponse, renseignez le script de condition dans Déclencheur et le script exécuté dans Action, puis enregistrez.", expected: "Le lien DLG contient les scripts Active et Script et reste visible sous la ligne source." },
      { title: "Contrôler la structure", instruction: "Ouvrez Graphe (avancé) seulement pour diagnostiquer les cycles, liens partagés ou branches inaccessibles.", expected: "Le graphe reste disponible sans être nécessaire pour l’édition courante." },
    ],
    outcome: "Une branche précise peut être trouvée, comprise et modifiée même dans un dialogue de plus de 1 000 nœuds.",
    troubleshooting: [
      { problem: "Le graphe indique qu’il est limité.", resolution: "C’est la protection de lisibilité. Recherchez le texte ou cliquez un nœud puis réduisez le voisinage au lieu d’afficher tout le DLG." },
      { problem: "Un nœud reste inaccessible.", resolution: "Ajoutez un départ ou un lien entrant valide, puis vérifiez les diagnostics du dialogue." },
    ],
    action: { label: "Ouvrir les dialogues", view: "dialogues" },
  },
  {
    id: "factions",
    category: "Éditer",
    title: "Modifier les factions et réputations",
    summary: "Travailler dans la matrice FAC sans mélanger les quêtes et leurs textes.",
    icon: Workflow,
    prerequisites: ["L’atelier transactionnel est créé.", "Les relations hostiles, neutres ou amicales attendues sont connues."],
    steps: [
      { title: "Sélectionner une faction", instruction: "Ouvrez Factions, puis choisissez son nom dans la liste de gauche. La matrice centrale affiche les noms complets dans les lignes et colonnes.", expected: "La fiche d’une seule faction est visible au-dessus de la matrice." },
      { title: "Créer ou modifier", instruction: "Ajoutez une faction avec son éventuel parent, ou dépliez sa fiche pour modifier son nom, son identifiant et son statut global.", control: "+ Faction", expected: "La matrice est complétée et les relations existantes sont préservées." },
      { title: "Régler une relation", instruction: "Dépliez Modifier ou ajouter une relation détaillée, choisissez source, cible et valeur entre 0 et 100.", expected: "La cellule correspondante change de valeur et de couleur." },
      { title: "Vérifier la suppression", instruction: "Avant de supprimer, contrôlez les parents et relations qui seront réindexés. Annulez si le résultat ne correspond pas à l’intention.", expected: "Aucun parent ni couple de réputation ne pointe vers un identifiant supprimé." },
    ],
    outcome: "Les factions et leur matrice restent compréhensibles sans afficher tous les formulaires simultanément.",
    troubleshooting: [{ problem: "Une relation n’est pas visible.", resolution: "Retirez le filtre, vérifiez que les deux factions sont présentes, puis ouvrez l’éditeur détaillé des réputations." }],
    action: { label: "Ouvrir les factions", view: "factions" },
  },
  {
    id: "blueprints",
    category: "Éditer",
    title: "Modifier un blueprint",
    summary: "Trouver une ressource par catégorie et l’éditer dans l’atelier central.",
    icon: Boxes,
    prerequisites: ["Le module est analysé.", "L’atelier transactionnel est créé pour enregistrer."],
    steps: [
      { title: "Filtrer la palette", instruction: "Ouvrez Blueprints, choisissez Créatures, Objets, Portes, Plaçables ou une autre catégorie, puis recherchez le ResRef.", control: "Rechercher un blueprint", expected: "La liste montre le type métier, le ResRef et la couche active." },
      { title: "Ouvrir la fiche centrale", instruction: "Sélectionnez la ressource. Propriétés métier affiche les champs nommés et leurs sous-structures sur toute la largeur utile.", expected: "Le formulaire n’est plus comprimé dans l’inspecteur global de droite." },
      { title: "Modifier", instruction: "Changez les valeurs utiles et utilisez les sections Compétences, Dons, Classes, Équipement, Propriétés ou variantes selon le type.", expected: "Chaque action affiche son résultat et produit une commande annulable." },
      { title: "Diagnostiquer si nécessaire", instruction: "Consultez Provenance pour les couches et GFF brut uniquement pour un champ non encore modélisé.", expected: "Le parcours normal reste lisible et aucune donnée inconnue n’est perdue." },
    ],
    outcome: "Un blueprint peut être trouvé et modifié sans menu étroit ni lecture préalable du JSON GFF.",
    troubleshooting: [{ problem: "Le type ou la valeur reste numérique.", resolution: "Vérifiez que le 2DA ou TLK correspondant est résolu dans la couche active ; ne remplacez pas l’index par un nom codé en dur." }],
    action: { label: "Ouvrir les blueprints", view: "blueprints" },
  },
  {
    id: "scripts",
    category: "Éditer",
    title: "Modifier et compiler un script NWScript",
    summary: "Enregistrer le NSS, compiler un NCS à jour et comprendre précisément ce qui sera exécuté.",
    icon: Code2,
    prerequisites: ["L’atelier transactionnel est créé.", "nwn_script_comp est localisé dans l’installation NWN:EE."],
    steps: [
      { title: "Ouvrir la source", instruction: "Dans Scripts, sélectionnez le ResRef voulu et ouvrez Source NSS. Le NSS est le texte éditable ; le NCS est le bytecode exécuté par le jeu.", expected: "L’éditeur affiche la source et l’état de correspondance NSS/NCS." },
      { title: "Enregistrer le texte", instruction: "Modifiez le code puis écrivez le NSS dans l’atelier. La compilation est volontairement impossible tant que l’éditeur contient des changements non enregistrés.", control: "Enregistrer NSS", expected: "L’indicateur de modification disparaît et le NSS du workspace correspond à l’éditeur." },
      { title: "Corriger les dépendances", instruction: "Vérifiez les includes, constantes et signatures signalés par les diagnostics. Les includes doivent être résolus depuis les couches connues du Resource Manager.", expected: "Aucune erreur de résolution préalable ne reste visible." },
      { title: "Compiler le bytecode", instruction: "Lancez le compilateur puis lisez sa sortie complète. Une sauvegarde NSS réussie ne remplace jamais cette étape.", control: "Compiler NSS → NCS", expected: "Un NCS récent est produit dans l’atelier et l’état n’indique plus Obsolète ou Absent." },
      { title: "Valider l’appelant", instruction: "Revenez aux Références pour confirmer que le ResRef du script est bien utilisé par le dialogue, l’évènement de module ou le blueprint prévu.", expected: "Le script compilé est relié à au moins un point d’entrée intentionnel." },
    ],
    outcome: "Le texte NSS enregistré et le bytecode NCS exécuté par NWN correspondent exactement.",
    warning: "Dans Aurora Toolset, utilisez F7 pour compiler puis sauvegardez le module avant de fermer. Sinon les fichiers temporaires peuvent être recréés sans vos derniers changements.",
    troubleshooting: [
      { problem: "Compiler NSS → NCS est désactivé.", resolution: "Enregistrez d’abord le NSS, renseignez le chemin du compilateur et vérifiez le chemin d’installation du jeu." },
      { problem: "Le jeu exécute une ancienne version.", resolution: "Contrôlez la date/empreinte du NCS et recherchez une version prioritaire dans development, override ou un HAK." },
      { problem: "Un include est introuvable.", resolution: "Vérifiez son ResRef, sa présence dans les couches chargées et la déclaration des HAK du module." },
    ],
    action: { label: "Ouvrir l’atelier NWScript", view: "scripts" },
  },
  {
    id: "custom",
    category: "Configurer",
    title: "Gérer HAK, TLK et 2DA",
    summary: "Comprendre les dépendances personnalisées et leur ordre de résolution.",
    icon: Boxes,
    prerequisites: ["Le module est analysé.", "Les fichiers personnalisés nécessaires sont présents dans les données utilisateur."],
    steps: [
      { title: "Lire module.ifo", instruction: "Identifiez la liste et l’ordre des HAK ainsi que le TLK personnalisé déclarés par le module.", expected: "Chaque nom déclaré correspond à un fichier local ou à une absence explicitement diagnostiquée." },
      { title: "Contrôler la priorité", instruction: "Dans Ressources, comparez les versions d’un même ResRef. Une ressource de couche supérieure masque les suivantes sans les supprimer.", expected: "La version effective est celle que vous souhaitez livrer." },
      { title: "Inspecter les 2DA", instruction: "Vérifiez les lignes ajoutées ou remplacées par les HAK et les références utilisées par les blueprints. Une ligne manquante peut produire une apparence invalide en jeu.", expected: "Les index 2DA référencés existent dans la table effectivement résolue." },
      { title: "Préparer la distribution", instruction: "Listez avec le MOD tous les HAK et TLK requis, avec leur version ou empreinte. Ne supposez pas qu’ils sont déjà installés chez le joueur.", expected: "Un autre poste peut reproduire le même ensemble de ressources." },
    ],
    outcome: "Les contenus personnalisés chargés sont connus, ordonnés et distribuables avec le module.",
    troubleshooting: [
      { problem: "Une apparence diffère entre deux postes.", resolution: "Comparez les HAK/TLK, leur ordre, leurs empreintes et les fichiers override/development locaux." },
      { problem: "Le texte affiche Bad StrRef.", resolution: "Vérifiez le TLK déclaré, sa langue et la validité du StrRef utilisé par la ressource." },
    ],
    action: { label: "Inspecter les ressources", view: "resources" },
  },
  {
    id: "ship",
    category: "Livrer",
    title: "Valider, construire et tester",
    summary: "Produire un MOD reproductible puis tester sans contaminer les données utilisateur.",
    icon: Hammer,
    prerequisites: ["Les changements utiles sont enregistrés dans l’atelier.", "Les NSS modifiés possèdent un NCS à jour."],
    steps: [
      { title: "Lire les diagnostics", instruction: "Corrigez d’abord les erreurs bloquantes, puis examinez les avertissements. Un avertissement accepté doit avoir une raison connue.", expected: "La construction n’est bloquée par aucune ressource invalide ou référence indispensable manquante." },
      { title: "Prouver la reproductibilité", instruction: "Construisez deux fois à partir du même état et comparez les empreintes. Un écart révèle une donnée variable ou une entrée non maîtrisée.", control: "Vérifier ×2", expected: "Les deux constructions produisent la même empreinte." },
      { title: "Produire le MOD", instruction: "Générez le conteneur final vers un chemin de sortie distinct. Le fichier source sélectionné au départ n’est pas écrasé.", control: "Construire", expected: "Un nouveau .mod est créé avec un journal de construction réussi." },
      { title: "Tester en surcouche", instruction: "Pour une itération rapide, déployez uniquement les ressources modifiées dans Documents/Neverwinter Nights/development. Cette couche prend priorité au chargement.", control: "Déployer development", expected: "Le manifeste de déploiement énumère exactement les fichiers copiés." },
      { title: "Nettoyer après essai", instruction: "Retirez avec OpenNever uniquement les fichiers présents dans son manifeste. Ne supprimez pas manuellement tout le dossier development.", control: "Nettoyer development", expected: "Les fichiers déployés par l’application sont retirés ; les autres restent intacts." },
    ],
    outcome: "Vous disposez d’un MOD reproductible et d’un cycle de test live réversible.",
    warning: "development masque les ressources du MOD. Un ancien fichier oublié peut fausser un test même si la construction est correcte.",
    troubleshooting: [
      { problem: "Vérifier ×2 produit deux empreintes différentes.", resolution: "Consultez le rapport de comparaison, stabilisez l’ordre, les horodatages ou les métadonnées variables avant livraison." },
      { problem: "Le jeu ne montre pas la dernière modification.", resolution: "Vérifiez le manifeste development, la provenance effective de la ressource et redémarrez la zone si le moteur la conserve en mémoire." },
    ],
    action: { label: "Ouvrir la salle de construction", view: "build" },
  },
  {
    id: "toolset",
    category: "Livrer",
    title: "Échanger avec Aurora Toolset",
    summary: "Comparer les fichiers temporaires du Toolset et récupérer les changements sans perte.",
    icon: Hammer,
    prerequisites: ["Le même module est ouvert ou a été sauvegardé dans Aurora Toolset.", "L’atelier OpenNever existe."],
    steps: [
      { title: "Sauvegarder côté Toolset", instruction: "Dans Aurora, compilez les scripts avec F7 puis sauvegardez le module. Les fichiers temporaires seuls ne constituent pas une sauvegarde durable.", expected: "Le MOD Toolset contient les derniers NCS et les dernières ressources enregistrées." },
      { title: "Détecter les écarts", instruction: "Ouvrez la synchronisation Toolset et comparez les ressources découvertes avec l’état du workspace.", control: "Comparer", expected: "Chaque différence indique sa ressource, son origine et le sens possible de synchronisation." },
      { title: "Choisir explicitement", instruction: "Cochez seulement les changements que vous comprenez. En cas de conflit, inspectez les deux versions avant d’en choisir une.", expected: "La sélection ne contient aucune écriture involontaire ou ressource inconnue." },
      { title: "Importer dans l’atelier", instruction: "Appliquez la sélection. OpenNever crée alors des commandes transactionnelles, sans modifier rétroactivement le MOD source initial.", control: "Synchroniser la sélection", expected: "Les ressources importées apparaissent dans le workspace et dans son journal." },
    ],
    outcome: "Les changements du Toolset sont récupérés de façon explicite, traçable et réversible.",
    warning: "Fermer Aurora sans F7 puis sauvegarde peut perdre un NSS/NCS récent lorsque le dossier temporaire est recréé à la prochaine ouverture.",
    troubleshooting: [
      { problem: "Comparer ne trouve aucun changement.", resolution: "Sauvegardez le module dans Aurora, vérifiez le chemin du dossier temporaire et relancez la comparaison." },
      { problem: "Un conflit concerne la même ressource.", resolution: "Comparez les empreintes et contenus, choisissez la version voulue ou reportez la synchronisation ; ne fusionnez pas à l’aveugle." },
    ],
    action: { label: "Ouvrir la synchronisation Toolset", view: "toolset" },
  },
  {
    id: "agent",
    category: "Automatiser",
    title: "Configurer et suivre l’IA",
    summary: "Tester le fournisseur, préparer un travail, suivre l’activité et approuver les opérations sensibles.",
    icon: Bot,
    prerequisites: ["L’atelier existe pour toute opération d’écriture.", "Le fournisseur, le modèle et la politique de sécurité sont configurés."],
    steps: [
      { title: "Choisir le moteur", instruction: "Dans l’étape 1, choisissez le fournisseur et le modèle. Endpoint, raisonnement et coût restent sous Réglages avancés lorsque les valeurs par défaut ne conviennent pas.", expected: "Le modèle exact est visible avant toute exécution." },
      { title: "Tester la liaison", instruction: "Cliquez Tester avant un objectif long. Ce premier appel envoie seulement une demande OK et aucune ressource NWN.", control: "Tester", expected: "Un état de réussite affiche le modèle et le temps de réponse." },
      { title: "Contrôler le contexte", instruction: "L’étape 2 rappelle l’atelier précédent et la ressource sélectionnée. Cliquez Utiliser cette sélection pour l’ajouter au périmètre sans saisir resref:type.", control: "Utiliser cette sélection", expected: "La ressource apparaît dans les portées du profil." },
      { title: "Créer l’exécution", instruction: "Décrivez un résultat vérifiable à l’étape 3, puis créez l’exécution. Cette action persiste l’objectif et les limites mais ne crée pas un plan IA et ne contacte pas le modèle.", control: "4 · Créer l’exécution", expected: "Une carte d’exécution apparaît avec l’état Prête à lancer." },
      { title: "Lancer et observer", instruction: "Démarrez la carte créée. Pendant l’appel, le bouton affiche un indicateur animé et la bannière d’activité précise l’étape en cours.", control: "Lancer l’agent", expected: "Le statut évolue, les tours et appels augmentent, et le journal décrit chaque étape." },
      { title: "Approuver sans déléguer la sécurité", instruction: "Relisez chaque commande proposée, sa cible et son aperçu. Accordez l’approbation uniquement aux opérations attendues ; refusez ou arrêtez sinon.", expected: "Seules les capacités autorisées et approuvées produisent des commandes dans le workspace." },
    ],
    outcome: "Le modèle communique correctement, son activité est visible et aucune écriture ne contourne les contrôles locaux.",
    warning: "N’accordez jamais au modèle un shell général ni un accès direct au MOD source. Les clés API sont temporaires et doivent rester hors des journaux.",
    troubleshooting: [
      { problem: "Le test de communication est désactivé.", resolution: "Choisissez un fournisseur réseau, autorisez le réseau, puis renseignez endpoint et modèle. Ajoutez une clé seulement si le serveur l’exige." },
      { problem: "Créer l’exécution fonctionne mais rien ne démarre.", resolution: "C’est normal : la création persiste seulement l’objectif et les limites. Cliquez ensuite sur Lancer l’agent, qui est la première action envoyant l’objectif au modèle." },
      { problem: "L’exécution attend indéfiniment.", resolution: "Lisez la bannière et le journal : elle peut attendre une approbation, une capacité autorisée, le réseau ou une réponse du fournisseur." },
      { problem: "Le fournisseur répond 401/403/404.", resolution: "401/403 indique généralement une clé ou permission incorrecte ; 404 indique souvent un endpoint, un protocole ou un nom de modèle erroné." },
    ],
    action: { label: "Ouvrir Agent Studio", view: "agent" },
  },
  {
    id: "diagnostics",
    category: "Dépanner",
    title: "Diagnostiquer un problème",
    summary: "Partir du symptôme, identifier la couche responsable et conserver une preuve exploitable.",
    icon: TriangleAlert,
    prerequisites: ["Le module ou l’atelier concerné est ouvert."],
    steps: [
      { title: "Noter le symptôme exact", instruction: "Relevez l’action, la ressource, l’heure et le message complet. Évitez de relancer plusieurs opérations avant d’avoir lu le premier échec.", expected: "Vous pouvez reproduire le problème avec une séquence courte." },
      { title: "Consulter Diagnostics", instruction: "Filtrez par gravité et ressource. Ouvrez les détails pour distinguer erreur de format, dépendance absente, compilation ou écriture refusée.", expected: "Le diagnostic pointe vers une ressource et une cause probable." },
      { title: "Vérifier la provenance", instruction: "Recherchez la ressource et contrôlez quelle couche gagne. development et override sont des causes fréquentes d’écart entre l’aperçu et le jeu.", expected: "La version réellement chargée est identifiée." },
      { title: "Reproduire proprement", instruction: "Nettoyez uniquement le déploiement géré, reconstruisez si nécessaire et répétez une seule action. Conservez le journal si l’échec persiste.", expected: "Le problème disparaît ou produit une preuve stable à partager." },
    ],
    outcome: "Vous avez soit corrigé la cause, soit isolé une reproduction avec message, ressource et provenance.",
    warning: "Ne supprimez jamais en bloc development, override, le cache ou le workspace pour faire disparaître un symptôme : vous perdriez la preuve et pourriez effacer des fichiers externes.",
    troubleshooting: [
      { problem: "L’interface semble bloquée.", resolution: "Cherchez l’indicateur d’activité, une fenêtre de sélection cachée ou une approbation en attente ; attendez la fin d’une opération d’écriture avant d’en lancer une autre." },
      { problem: "Le résultat diffère uniquement en jeu.", resolution: "Contrôlez development, override, les HAK, le cache de zone et la version du MOD effectivement lancé." },
    ],
    action: { label: "Ouvrir les diagnostics", view: "diagnostics" },
  },
];

function topicMatches(topic: GuideTopic, query: string) {
  const haystack = [
    topic.category,
    topic.title,
    topic.summary,
    topic.outcome,
    topic.warning ?? "",
    ...topic.prerequisites,
    ...topic.steps.flatMap((step) => [step.title, step.instruction, step.control ?? "", step.expected]),
    ...topic.troubleshooting.flatMap((item) => [item.problem, item.resolution]),
  ].join(" ").toLocaleLowerCase("fr");
  return haystack.includes(query.trim().toLocaleLowerCase("fr"));
}

export function HelpCenter({ hasModule, hasWorkspace, onNavigate }: HelpCenterProps) {
  const [mode, setMode] = useState<"guide" | "manual">("guide");
  const [selectedTopic, setSelectedTopic] = useState("start");
  const [query, setQuery] = useState("");
  const filteredTopics = useMemo(() => topics.filter((candidate) => topicMatches(candidate, query)), [query]);
  const topic = topics.find((candidate) => candidate.id === selectedTopic) ?? topics[0];
  const TopicIcon = topic.icon;

  return (
    <section className="help-center workspace-page" aria-label="Aide utilisateur OpenNever Forge">
      <header className="workspace-page-header help-header">
        <div>
          <span className="rpg-kicker"><Sparkles size={13} /> Codex du bâtisseur</span>
          <h1>Aide et manuel</h1>
          <p>Des procédures précises pour agir, vérifier le résultat et résoudre les erreurs courantes.</p>
        </div>
        <div className="segmented-control" aria-label="Mode d’aide">
          <button type="button" className={mode === "guide" ? "active" : ""} onClick={() => setMode("guide")}>
            <Compass size={14} /> Tutoriels guidés
          </button>
          <button type="button" className={mode === "manual" ? "active" : ""} onClick={() => setMode("manual")}>
            <BookOpenCheck size={14} /> Manuel complet
          </button>
        </div>
      </header>

      {mode === "manual" ? (
        <div className="manual-viewport">
          <iframe title="Manuel complet OpenNever Forge" srcDoc={fullManualHtml} sandbox="allow-scripts" />
        </div>
      ) : (
        <div className="help-guide-layout">
          <nav className="help-topic-list" aria-label="Tutoriels disponibles">
            <label className="help-search">
              <Search size={14} />
              <input value={query} onChange={(event) => setQuery(event.currentTarget.value)} placeholder="Bouton, tâche ou erreur…" aria-label="Rechercher dans les tutoriels" />
            </label>
            <small className="help-search-count">{filteredTopics.length} tutoriel{filteredTopics.length > 1 ? "s" : ""}</small>
            {filteredTopics.map((candidate, index) => {
              const Icon = candidate.icon;
              const showCategory = index === 0 || filteredTopics[index - 1].category !== candidate.category;
              return (
                <div className="help-topic-entry" key={candidate.id}>
                  {showCategory && <span className="help-topic-category">{candidate.category}</span>}
                  <button type="button" className={candidate.id === topic.id ? "active" : ""} onClick={() => setSelectedTopic(candidate.id)}>
                    <Icon size={18} />
                    <span><strong>{candidate.title}</strong><small>{candidate.summary}</small></span>
                  </button>
                </div>
              );
            })}
            {!filteredTopics.length && <div className="help-empty"><Search size={18} /><strong>Aucun tutoriel trouvé</strong><small>Essayez un ResRef, un bouton comme « Compiler » ou une erreur comme « 401 ».</small></div>}
          </nav>
          <article className="help-topic-detail">
            <div className="help-topic-emblem"><TopicIcon size={30} strokeWidth={1.6} /></div>
            <span className="rpg-kicker">{topic.category} · Tutoriel guidé</span>
            <h2>{topic.title}</h2>
            <p>{topic.summary}</p>

            <section className="help-prerequisites" aria-labelledby="help-prerequisite-title">
              <h3 id="help-prerequisite-title">Avant de commencer</h3>
              <ul>{topic.prerequisites.map((item) => <li key={item}><CheckCircle2 size={14} /> {item}</li>)}</ul>
            </section>

            <ol className="quest-steps">
              {topic.steps.map((step, index) => (
                <li key={`${topic.id}-${step.title}`}>
                  <span>{index + 1}</span>
                  <div className="quest-step-content">
                    <strong>{step.title}</strong>
                    <p>{step.instruction}</p>
                    {step.control && <code className="help-control">Interface : {step.control}</code>}
                    <small><CheckCircle2 size={13} /> <b>Résultat attendu :</b> {step.expected}</small>
                  </div>
                </li>
              ))}
            </ol>

            <div className="help-outcome"><Sparkles size={17} /><span><strong>Parcours terminé</strong>{topic.outcome}</span></div>
            {topic.warning && <div className="help-warning"><ShieldCheck size={17} /><span><strong>Point de vigilance</strong>{topic.warning}</span></div>}

            <section className="help-troubleshooting" aria-labelledby="help-troubleshooting-title">
              <h3 id="help-troubleshooting-title"><TriangleAlert size={16} /> Si cela ne fonctionne pas</h3>
              {topic.troubleshooting.map((item) => (
                <details key={item.problem}>
                  <summary>{item.problem}</summary>
                  <p>{item.resolution}</p>
                </details>
              ))}
            </section>

            <button type="button" className="primary-button quest-action" onClick={() => onNavigate(topic.action.view)}>
              <Play size={15} /> {topic.action.label}
            </button>
          </article>
          <aside className="help-progress-card">
            <span className="rpg-kicker">Repères permanents</span>
            <h3>Cycle de travail sûr</h3>
            <div className={hasModule ? "help-state complete" : "help-state"}><span>1</span><div><strong>Analyser</strong><small>{hasModule ? "Module prêt" : "Source intacte"}</small></div></div>
            <div className={hasWorkspace ? "help-state complete" : "help-state"}><span>2</span><div><strong>Créer l’atelier</strong><small>{hasWorkspace ? "Overlay prêt" : "Écriture séparée"}</small></div></div>
            <div className="help-state"><span>3</span><div><strong>Modifier</strong><small>Commandes réversibles</small></div></div>
            <div className="help-state"><span>4</span><div><strong>Compiler</strong><small>NSS → NCS</small></div></div>
            <div className="help-state"><span>5</span><div><strong>Valider</strong><small>Diagnostics + ×2</small></div></div>
            <div className="help-state"><span>6</span><div><strong>Tester</strong><small>MOD ou development</small></div></div>
            <div className="help-legend"><ShieldCheck size={15} /><span><strong>Règle essentielle</strong>La source analysée n’est jamais la destination des écritures.</span></div>
          </aside>
        </div>
      )}
    </section>
  );
}
