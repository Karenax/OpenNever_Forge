# Changelog

Tous les changements notables du projet sont consignés ici.

## [Unreleased]

### Added

- Socle initial Tauri 2, React 19, TypeScript strict et Rust stable.
- Architecture en crates pour les erreurs, les projets en lecture seule et l'index SQLite.
- Shell sombre de l'éditeur et premier job de hash SHA-256 annulable.
- Graphe d'architecture déterministe et contrôle de fraîcheur.
- Lecteur ERF/MOD borné, inventaire des ressources et accès à la demande limité à 16 Mio.
- Lecteur GFF V3.2 minimal pour le nom du module, sa version, sa zone d'entrée, ses HAK et son TLK.
- Explorateur filtrable et inspecteur synchronisé avec les résultats de l'analyse native.

### Fixed

- Empêche l'en-tête flottant de l'inventaire de recouvrir le panneau Diagnostics.
- Met à jour le compteur de l'inspecteur lorsque l'inventaire asynchrone arrive.
