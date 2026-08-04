# Validation Lot 1 — oracle neverwinter.nim

- Date : 3 août 2026
- Oracle : `neverwinter.nim` 2.1.2 (`/07a475`, Nim 2.2.4)
- Source : <https://github.com/niv/neverwinter.nim>
- Licence : MIT
- Archive Windows : `neverwinter-x86_64-windows.zip`
- SHA-256 observé : `B00501CC57ADC63392F17D460D712EDCDCBE35CB37F7D7257AB23806ED86AED1`
- Fixture : `fixtures/synthetic/lot1_custom_tlk`

## Portée

L'oracle est téléchargé et exécuté uniquement dans `.tmp/`, ignoré par Git. Il n'est lié ni au cœur
Rust ni à l'application Tauri. Le script `tools/compare-oracles/compare_neverwinter_nim.py` appelle
séparément `nwn_erf`, `nwn_gff` et `nwn_tlk`, extrait `module.ifo` dans un dossier temporaire, puis
compare les valeurs à `manifest.json`.

## Résultat

Les huit contrôles réussissent :

1. `module.ifo` est présent dans le MOD ;
2. le type GFF est `IFO ` ;
3. le nom du module est identique ;
4. le tag est identique ;
5. la zone d'entrée est identique ;
6. le TLK personnalisé est identique ;
7. la liste des HAK est identique ;
8. le premier texte TLK est identique.

Aucune divergence n'a été observée. Ce résultat valide la compatibilité de cette fixture minimale,
pas l'ensemble des comportements ERF, GFF ou TLK. Toute divergence future sera conservée dans le
rapport et analysée ; l'oracle ne remplace jamais les exigences ni les tests internes.

## Reproduction

```powershell
python scripts/generate_lot1_fixture.py fixtures/synthetic/lot1_custom_tlk --force
python tools/compare-oracles/compare_neverwinter_nim.py `
  --oracle-dir C:\outils\neverwinter-2.1.2
```
