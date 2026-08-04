# Revue d'avancement — Phases 2 et 3

Date : 4 août 2026

## Livré et vérifié

| Lot | Incrément | Preuve actuelle |
|---:|---|---|
| 11 | Espace transactionnel | schéma v3 migrable avec sauvegarde, récupération après interruption, commande liée exactement à ses octets, undo/redo atomique et identifiants distincts par overlay |
| 12 | Sérialisation | writers GFF V3.2 et ERF/MOD/HAK déterministes, payloads en streaming, métadonnées ERF préservées et round-trip |
| 13 | Propriétés structurées | champs DLG/JRL/FAC/blueprints éditables sans perte ; structures DLG/JRL/FAC et listes prioritaires UTC/UTI/UTS/UTE transactionnelles, réindexées et réhydratées depuis l'overlay |
| 14 | NWScript | édition Monaco, `nwnsc` explicite et hashé, includes transitifs exacts, NSS → NCS obligatoire et détection de NCS périmé |
| 15 | Zones | déplacement, ajout et suppression d'instances, modification des tuiles et aperçu 2D de l'overlay |
| 16 | Build et test | nouveau MOD, build streaming, export `development`, propriété par workspace et nettoyage par hash |
| 17 | Module neuf et palettes | IFO + zone d'entrée ARE/GIT/GIC canoniques, palette typée et validation par `neverwinter.nim` 2.1.2 |
| 18 | Création de zones | transaction ARE/GIT/GIC atomique jusqu'à 64×64, suppression réversible et réhydratation immédiate dans l'UI |
| 19 | Objets de zone | placement/suppression des neuf catégories GIT, polygones typés UTT/UTE, points d'apparition, transitions complètes et inventaires incorporés de placeables/magasins |
| 20 | Walkmeshes | grammaires autonomes WOK/PWK/DWK, ancien `#MAXDOOR`, surfaces, variantes/hooks, AABB déterministe, validation topologique et opérations déplacer/découper/supprimer/extruder/souder via l'atelier transactionnel |
| 21 | Contenu personnalisé | writers TLK V3.0 et 2DA V2.0 déterministes, éditions typées annulables, gestionnaire graphique HAK/TLK appliqué à `module.ifo`, writer HAK interne |
| 22 | Reproductibilité/Git | profils persistants, cohérence HAK/TLK prévalidée, double build comparé par SHA-256, profils `nwmain`/`nwserver` journalisés et inspection Git bornée sans shell |
| 23 | Synchronisation Aurora | comparaison Toolset/OpenNever/baseline, conflits explicites, préconditions SHA-256, imports annulables et sauvegardes avant écriture ou suppression Toolset |
| 24 | Documentation/migrations | schéma v3 avec sauvegarde byte-for-byte, historique visible, guides utilisateur et migration, ADR et moteur de synchronisation isolé |
| 25 | IA contrôlée | endpoint/modèle explicites, consentements séparés, clé éphémère, import local, opérations GFF/NSS bornées, validation d’octets, empreinte de confirmation et application annulable |

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

## Validation locale finale sans Aurora

- Le test de cycle autonome crée un MOD IFO/ARE/GIT/GIC, ouvre son workspace, applique une
  proposition contrôlée, construit un nouveau MOD, le rouvre et vérifie la valeur modifiée.
- Le SHA-256 et les octets du MOD source sont identiques avant et après le cycle.
- L’interface vérifie le mode offline initial et le parcours JSON local sans fournisseur externe.
- Les commandes IA non admises, les préconditions périmées et l’empreinte de confirmation modifiée
  sont refusées.

## Acceptations externes restantes

- confirmation du chargement en jeu sur un profil où le témoin moteur démarre ; l'implémentation et
  le harnais WOK/PWK/DWK du Lot 20 sont terminés ;
- confirmation moteur des profils de lancement du jeu/nwserver sur un environnement où le témoin
  ne quitte pas avec `0xC0000005` ;
- essai facultatif auprès d’un fournisseur réel choisi par l’utilisateur. Un essai Ollama local a
  dépassé 90 secondes sans réponse exploitable ; le pipeline IA reste livré et prouvé sans réseau.

Tous les lots de développement sont terminés. La candidate Windows et le cycle Toolset réel
comparer → synchroniser → compiler → sauvegarder → rouvrir ont été contrôlés le 4 août 2026. La
preuve détaillée est conservée dans `release-closure-2026-08-04.md`. Cette revue ne déclare pas un
chargement moteur réussi tant qu'il n'a pas été observé sur un environnement fonctionnel.

## Ordre recommandé restant

1. compléter la preuve des profils `nwmain`/`nwserver` sur un environnement moteur fonctionnel ;
2. rejouer le harnais Lot 20 dès qu'un profil `nwserver` témoin fonctionnel est disponible ;
3. facultativement, rejouer l'appel IA auprès d'un fournisseur compatible réactif choisi par
   l'utilisateur.
