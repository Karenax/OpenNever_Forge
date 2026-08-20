# Créateur de cartes et vibecoding déterministe

## Objectif produit

La page **Construire → Créateur de cartes** transforme une intention humaine en une carte NWN
reproductible, révisable et accompagnée d’un plan de repérage. Le modèle IA décrit un contrat ; il
ne manipule jamais directement ARE, GIT, GIC, le MOD source ou le système de fichiers.

## Parcours retenu

1. Décrire l’ambiance, la circulation, les points d’intérêt et la population dans un brief.
2. Fixer les contraintes reproductibles : ResRef, tileset, dimensions, graine, marge, réserve libre,
   tuiles autorisées, densités, espacements et blueprints autorisés.
3. Générer localement un `MapGenerationPlan` et afficher son empreinte, ses métriques, ses
   avertissements, ses tuiles et ses placements.
4. Appliquer exactement cette empreinte dans une transaction unique qui crée ARE/GIT/GIC dans
   l’overlay.
5. Relire les octets produits avec les adaptateurs métier, puis afficher la zone dans l’atlas.
6. Exporter une carte de repérage SVG ou PNG depuis les tuiles et instances relues.
7. Pour le vibecoding direct, choisir OpenAI Responses, Chat Completions, Ollama ou une API
   compatible dans la page, puis demander exactement un contrat contrôlé `map.generate`.
8. Utiliser Agent Studio seulement pour un travail multi-outils plus avancé.
9. Depuis un client MCP, suivre `map.context` → `map.preview` → `map.apply` → `map.inspect`, puis
   effectuer des éditions ciblées avec les SHA-256 ARE/GIT retournés par l'inspection.

## Garanties déterministes

- Une même spécification et une même graine produisent le même SHA-256, les mêmes tuiles et les
  mêmes placements.
- Les placements utilisent une fonction de mélange locale stable ; aucun ordre aléatoire système,
  horloge ou réponse réseau n’entre dans le calcul.
- La densité est exprimée par cent tuiles constructibles. La marge, la réserve libre, l’occupation
  exclusive d’une cellule et l’espacement minimal ont priorité sur la quantité demandée.
- Les limites conservatrices sont bornées et partagées par l’interface, le schéma d’outil et le cœur
  Rust : zone 32×32 maximum (1 024 tuiles), ResRef de 16 caractères, 16 règles de densité,
  128 ResRef par règle et 2 048 placements.
- Le SET sélectionné est résolu via le Resource Manager ; son SHA-256 et ses identifiants de tuiles
  sont vérifiés avant la prévisualisation et de nouveau avant l’écriture.
- Les orientations générées restent à `0` tant que les connecteurs SET ne sont pas prouvés ; une
  rotation devient une édition explicite et préconditionnée après inspection.
- Un blueprint absent du Resource Manager et de l’overlay est refusé avant toute création.
- L’application recalcule le plan et compare son empreinte à la prévisualisation.

## IA directe et confidentialité

La page mémorise uniquement la nature du fournisseur, l’endpoint et le nom du modèle. La clé API
reste en mémoire vive jusqu’à l’appel, puis elle est effacée sans être écrite dans le stockage local. La requête contient le brief, le
contrat courant, les limites, les identifiants du SET et, avec consentement, au maximum 128 noms
ResRef par catégorie. Aucun octet NWN, GFF, script, dialogue, texture ou chemin local n’est transmis.
La réponse ne modifie rien : elle doit contenir exactement un appel `map.generate`, puis elle est
désérialisée, validée contre le catalogue local et prévisualisée avant approbation.

## Image de repérage

L’atlas n’est pas une capture décorative de la scène 3D. C’est une image stable destinée à
l’orientation : nord, grille, identifiants de tuiles, ResRef, tileset et marqueurs d’instances. Le
SVG reste éditable ; le PNG convient au partage, à la documentation et aux aides de jeu.

## État de livraison

### Livré dans le premier incrément

- contrat `MapGenerationSpec` versionné ;
- générateur Rust déterministe et tests de reproductibilité ;
- densité, espacement, marge et réserve libre ;
- création atomique ARE/GIT/GIC ;
- page complète dans Construire ;
- vérification du SET résolu et des identifiants de tuiles ;
- génération directe par fournisseur distant ou local avec clé éphémère ;
- passage du brief à Agent Studio ;
- capacité agentique `map.generate` avec politique, checkpoint et approbation ;
- atlas de toutes les zones relues et export SVG/PNG.
- surface MCP complète pour inspecter puis éditer tuiles et hauteurs, environnement, audio,
  instances, volumes, points d'apparition, transitions et inventaires ;
- résolution MCP locale du MOD, de ses HAK, de l'installation et des données utilisateur NWN ;
- préconditions SHA-256 sur chaque édition cartographique et ressource de guidage
  `opennever://map/authoring-contract`.
- atlas SVG MCP déterministe avec grille, identifiants, orientations, hauteurs et marqueurs.

### Incréments suivants

1. Ajouter un solveur des connecteurs et hauteurs SET pour prouver les raccords entre variantes.
2. Ajouter des régions sémantiques (route, salle, forêt, eau, intérieur) et leurs contraintes de
   voisinage.
3. Produire automatiquement les paires porte/transition et les connexions entre plusieurs zones ;
   l'édition unitaire de leurs destinations est déjà disponible via MCP.
4. Ajouter des objectifs spatiaux au contrat : centre, bord, proximité, évitement et alignement.
5. Relier les points d’intérêt aux dialogues, quêtes, scripts et rencontres générés dans le même
   lot approuvé.
6. Comparer le plan de repérage au rendu 3D puis exiger un test NWN pour qualifier une carte comme
   jouable.

## Critères d’acceptation

- deux prévisualisations identiques ont le même SHA-256 ;
- modifier la graine change la disposition sans changer les limites ;
- une densité impossible produit un avertissement, jamais une superposition silencieuse ;
- un ResRef absent bloque l’application avant le workspace ;
- un SET absent ou un identifiant de tuile inconnu bloque la prévisualisation ;
- une seule annulation retire les trois ressources et tous leurs placements ;
- la zone relue contient le nombre attendu de tuiles et d’instances ;
- chaque zone du module peut produire un SVG et un PNG de repérage ;
- le MOD, les HAK et l’installation source restent byte-for-byte inchangés.
