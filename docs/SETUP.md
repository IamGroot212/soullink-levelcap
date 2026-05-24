# Setup-Anleitung für Mitspieler

## Windows 10/11

1. Lade von der [Releases-Seite](https://github.com/IamGroot212/soullink-levelcap/releases) die neueste Version:
   - `soullink-levelcap-windows-x64.exe`
   - `caps.example.txt`
2. Lege beide in einen Ordner, z. B. `C:\Tools\soullink\`.
3. Benenne `caps.example.txt` um zu `caps.txt`.
4. Starte Citra mit Pokémon Alpha Saphir.
5. Doppelklick auf `soullink-levelcap-windows-x64.exe`.
   - Bei SmartScreen-Warnung: "Weitere Informationen" → "Trotzdem ausführen".
6. Es öffnet sich ein Konsolenfenster mit Live-Status. **Fenster offen lassen** während ihr spielt.

## Steam Deck (Desktop-Mode) / Linux

1. Wechsle in den Desktop-Mode.
2. Lade von der Releases-Seite:
   - `soullink-levelcap-linux-x64`
   - `caps.example.txt`
3. In einem Terminal:
   ```bash
   chmod +x soullink-levelcap-linux-x64
   mv caps.example.txt caps.txt
   ```
4. Starte Citra mit Pokémon Alpha Saphir.
5. Im gleichen Terminal:
   ```bash
   ./soullink-levelcap-linux-x64
   ```
6. Terminal offen lassen während ihr spielt.

## Wie funktioniert es?

Der Daemon liest deine aktuelle Orden-Anzahl direkt aus dem Spielspeicher und
schaut in `caps.txt` nach dem passenden Level-Cap. EXP der Party-Pokémon werden
automatisch auf diesem Cap eingefroren — sobald du einen neuen Orden gewinnst,
steigt der Cap automatisch.

## caps.txt anpassen

```
0=15
1=19
...
8=48
```

Linkes Feld: Anzahl gewonnener Hoenn-Liga-Orden. Rechtes Feld: maximaler Level.
Zeilen mit `#` sind Kommentare. Alle Mitspieler müssen die gleiche Datei haben.

## Probleme?

Siehe [TROUBLESHOOTING.md](TROUBLESHOOTING.md).
