# Mitspieler Quick-Start (Copy-Paste für Discord/Slack)

Kurze Setup-Anweisung zum Weiterleiten an alle Soullink-Teilnehmer.

---

## Für Windows-Spieler (3 Personen)

Hi! Setup für unser Soullink-Levelcap-Tool, dauert ~5 Minuten:

1. **Download** von https://github.com/IamGroot212/soullink-levelcap/releases/latest:
   - `soullink-levelcap-windows-x64.exe`
   - `caps.example.txt`

2. **Beide Dateien in einen Ordner** (z.B. `C:\Tools\soullink\`).
   - `caps.example.txt` umbenennen zu `caps.txt`.

3. **Citra starten**, ORAS laden, **"Continue"** klicken (nicht im Hauptmenü stehen bleiben).

4. **Daemon starten** per Doppelklick auf `soullink-levelcap-windows-x64.exe`.
   - Bei SmartScreen-Warnung: "Weitere Informationen" → "Trotzdem ausführen".
   - Bei Defender-Quarantäne: Ordner zu Ausnahmen hinzufügen (oder Code selbst bauen).

5. **Konsolenfenster offen lassen**. Du solltest sehen:
   ```
   [INFO] Save-File: C:\Users\...\main
   [INFO] Offsets detected: BADGE=0x..., PARTY=0x...
   [INFO] Orden-Anzahl: N → Cap: Level X
   ```

   Im Kampf, sobald jemand übers Cap gehen will:
   ```
   [CAP] Slot 0 (species X) auf Lvl Y eingefroren (EXP M → N)
   ```

Während des Spielens das Konsolenfenster minimieren ist OK, nur **nicht schließen**.

---

## Für Steam Deck (Desktop-Modus)

1. **Desktop-Modus** aktivieren.

2. **Download** in `~/Downloads/`:
   - `soullink-levelcap-linux-x64`
   - `caps.example.txt`

3. **Terminal** öffnen, `cd ~/Downloads`:
   ```bash
   chmod +x soullink-levelcap-linux-x64
   cp caps.example.txt caps.txt
   ```

4. **Citra starten** (Flatpak oder AppImage), ORAS laden, "Continue".

5. **Daemon starten** im Terminal:
   ```bash
   ./soullink-levelcap-linux-x64
   ```

6. Terminal-Fenster offen lassen während ihr spielt.

**Wichtig**: Citra muss **nativ** auf Linux laufen, nicht via Proton/Wine.
Steam Deck hat citra-emu im Discover Store (Flatpak) — der ist nativ Linux. ✓

---

## Voraussetzungen für alle

- Jeder muss **eigene randomisierte ORAS-CXI** + eigenes Save haben.
- **Pokemon Bank / 3DS-Hardware nicht nötig** — alles läuft in Citra.
- Save-Pfade müssen Standard-Citra sein:
  - Windows: `%APPDATA%\Citra\sdmc\Nintendo 3DS\...`
  - Linux: `~/.local/share/citra-emu/sdmc/Nintendo 3DS/...`
  - Flatpak: `~/.var/app/org.citra_emu.citra/data/citra-emu/sdmc/...`
  
  Auto-Triangulation findet das selbst. Bei non-Standard-Pfad: `--sav-path "..."` Flag.

## Häufige Fragen

**Q: Was wenn das Tool sagt "Offset-Detection fehlgeschlagen"?**
A: Wahrscheinlich noch nicht in-game (Citra im Menü statt im Spiel). "Continue" klicken, das Tool versucht alle 2s neu.

**Q: Was wenn meine Pokemon zu Eiern werden?**
A: ⚠ Das war ein alter Bug (PB6-Checksum). Wenn du `v0.1.0` oder neuer nutzt, sollte das nicht mehr passieren. Falls doch: **nicht in-game speichern** und Issue öffnen.

**Q: Was wenn ich schon über dem Cap bin (z.B. Lvl 64 bei 2 Orden = Cap 24)?**
A: Daemon friert dich auf deinem **aktuellen** Level ein (Overshoot-Policy). Du wirst nicht zurück-gelevelt, du kannst nur nicht weiter wachsen.

**Q: Wie ändere ich die Caps?**
A: `caps.txt` editieren. Alle Spieler müssen die gleiche `caps.txt` haben. Format: `Orden=MaxLevel`, eine pro Zeile.
