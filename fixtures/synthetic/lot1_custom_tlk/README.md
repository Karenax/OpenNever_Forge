# Fixture synthétique du Lot 1

Cette fixture est créée entièrement par OpenNever Forge et placée sous CC0-1.0. Elle ne contient
aucune ressource provenant de Neverwinter Nights.

Contenu attendu :

- `module/forge_lot1.mod` : MOD V1.0 minimal avec un `module.ifo` GFF V3.2 ;
- `user/hak/forge_assets.hak` : HAK V1.0 vide et valide ;
- `user/tlk/forge_dialog.tlk` : TLK V3.0 minimal avec une chaîne synthétique ;
- `manifest.json` : attentes sémantiques, tailles et empreintes SHA-256.

Régénération déterministe :

```powershell
python scripts/generate_lot1_fixture.py fixtures/synthetic/lot1_custom_tlk --force
```

Le générateur refuse par défaut d'écraser les fichiers. `--force` ne remplace que les quatre sorties
connues de cette fixture.
