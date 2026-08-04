# Revue d'avancement — Phases 2 et 3

Date : 4 août 2026

## Livré et vérifié

| Lot | Incrément | Preuve actuelle |
|---:|---|---|
| 11 | Espace transactionnel | schéma v2 récupérable après interruption, commande liée exactement à ses octets, undo/redo atomique et identifiants distincts par overlay |
| 12 | Sérialisation | writers GFF V3.2 et ERF/MOD/HAK déterministes, payloads en streaming, métadonnées ERF préservées et round-trip |
| 13 | Propriétés structurées | champs DLG/JRL/FAC/blueprints éditables sans perte ; structures DLG/JRL/FAC et listes prioritaires UTC/UTI/UTS/UTE transactionnelles, réindexées et réhydratées depuis l'overlay |
| 14 | NWScript | édition Monaco, `nwnsc` explicite et hashé, includes transitifs exacts, NSS → NCS obligatoire et détection de NCS périmé |
| 15 | Zones | déplacement, ajout et suppression d'instances, modification des tuiles et aperçu 2D de l'overlay |
| 16 | Build et test | nouveau MOD, build streaming, export `development`, propriété par workspace et nettoyage par hash |
| 17 | Module neuf et palettes | IFO + zone d'entrée ARE/GIT/GIC canoniques, palette typée et validation par `neverwinter.nim` 2.1.2 |
| 18 | Création de zones | transaction ARE/GIT/GIC atomique jusqu'à 64×64, suppression réversible et réhydratation immédiate dans l'UI |
| 19 | Objets de zone | placement/suppression des neuf catégories GIT, polygones typés UTT/UTE, points d'apparition, transitions complètes et inventaires incorporés de placeables/magasins |
| 20 | Walkmeshes | grammaires autonomes WOK/PWK/DWK, ancien `#MAXDOOR`, surfaces, variantes/hooks, AABB déterministe, validation topologique et opérations déplacer/découper/supprimer/extruder/souder via l'atelier transactionnel |
| 21 | Contenu personnalisé | writer HAK interne et profil HAK/TLK validé |
| 22 | Reproductibilité/Git | export trié des ressources modifiées avec hashes et suppressions déclarées |
| 23 | Synchronisation Aurora | scan strictement en lecture seule, profondeur et nombre de fichiers bornés, extensions autorisées, liens symboliques ignorés |
| 25 | IA contrôlée | prévisualisation séquentielle d'un lot de commandes sans mutation implicite |

## Contrôles externes du 4 août 2026

- Le MOD synthétique est ouvert par `nwn_erf.exe` 2.1.2 et ses quatre GFF sont convertis par
  `nwn_gff.exe` 2.1.2 sans erreur.
- Le module BioWare autorisé reste inchangé ; sa copie et une copie réemballée sont toutes deux
  lisibles par l'oracle indépendant.
- `nwserver.exe` retourne `0xC0000005` avec le MOD synthétique **et** avec le module BioWare original
  dans le profil de validation isolé. Ce résultat classe le contrôle moteur local comme
  environnement invalide/inconclusif ; il ne peut pas servir de preuve négative sur le writer.
- Le harnais Lot 20 a produit, relu puis déployé un WOK, un PWK à hooks seuls et un DWK à deux
  variantes réellement référencés par la copie autorisée. Les SHA-256 avant/après du MOD source
  sont identiques. Le témoin et l'overlay échouent tous deux avant écoute avec le même code serveur.
- Le client Windows NWN:EE 89.8193.37-17 démarre avec ces overrides dans `development`. Le contrôle
  automatisé s'est arrêté à la sélection du module lorsqu'une intervention utilisateur a été
  détectée ; aucune conclusion de chargement en jeu n'est donc inventée.

## Restant avant remplacement complet d'Aurora

- confirmation du chargement en jeu sur un profil où le témoin moteur démarre ; l'implémentation et
  le harnais WOK/PWK/DWK du Lot 20 sont terminés ;
- éditeurs TLK/2DA complets et gestion graphique des dépendances HAK ;
- profils de lancement du jeu/nwserver, intégration Git visible et synchronisation bidirectionnelle
  contrôlée avec un projet Toolset ;
- documentation utilisateur complète, migrations de projets et assistance IA branchée à un modèle
  choisi par l'utilisateur.

La fondation de chaque chantier est présente, mais cette revue ne déclare pas la Phase 3 terminée.
Le critère final reste la production d'un module complexe sans Aurora.

## Ordre recommandé restant

1. livrer les éditeurs TLK/2DA et le gestionnaire HAK du Lot 21 ;
2. terminer les Lots 16 et 22 par les profils de lancement et l'intégration Git visible ;
3. terminer la synchronisation bidirectionnelle contrôlée du Lot 23 ;
4. clore le Lot 24 (documentation/migrations), puis brancher le modèle explicite du Lot 25 ;
5. rejouer le harnais Lot 20 dès qu'un profil `nwserver` témoin fonctionnel est disponible.
