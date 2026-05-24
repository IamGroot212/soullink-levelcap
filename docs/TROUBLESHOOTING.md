# Troubleshooting

## "Citra-Prozess nicht gefunden"

- Stelle sicher, dass Citra **läuft** und **Pokémon Alpha Saphir bereits geladen** ist (nicht nur das Menü).
- Der Daemon sucht nach Prozessen, deren Name mit `citra` beginnt (z. B. `citra`, `citra-qt`, `citra-qt.exe`). Wenn dein Citra-Build einen anderen Namen hat, melde es als Issue.

## Linux: "process_vm_readv ... Operation not permitted"

Standard-Ubuntu/Debian setzt `kernel.yama.ptrace_scope=1`, was OK ist, weil Daemon und Citra unter demselben User laufen. Wenn du `ptrace_scope=2` oder `3` hast:

```bash
# Prüfen:
sysctl kernel.yama.ptrace_scope

# Temporär auf 1 setzen (bis Reboot):
sudo sysctl kernel.yama.ptrace_scope=1

# Permanent (Vorsicht — schwächt Härtung):
echo "kernel.yama.ptrace_scope = 1" | sudo tee /etc/sysctl.d/10-ptrace.conf
```

Auf Steam Deck ist `ptrace_scope=1` Standard, sollte also out-of-the-box gehen.

## Windows: SmartScreen / Defender flaggt die Binary

`ReadProcessMemory` und `WriteProcessMemory` triggern manche Heuristiken (Game-Cheats nutzen die gleichen APIs).

- SmartScreen-Dialog: "Weitere Informationen" → "Trotzdem ausführen".
- Falls Defender den Download quarantäniert: Ausnahme für den Ordner hinzufügen (Windows-Sicherheit → Viren- & Bedrohungsschutz → Einstellungen → Ausschlüsse).
- Quell-Code ist offen — du kannst die Binary selbst aus dem Repo bauen, falls du dem Release-Artefakt nicht traust: `cargo build --release`.

## "caps.txt nicht gefunden"

Der Daemon sucht `caps.txt` im **aktuellen Arbeitsverzeichnis** (nicht neben der Binary, falls von woanders aufgerufen!). Lösung:

```bash
# Linux:
cd /pfad/zum/ordner/mit/caps.txt
./soullink-levelcap-linux-x64

# Oder expliziten Pfad:
./soullink-levelcap-linux-x64 --caps-file /pfad/zu/caps.txt
```

Auf Windows: Doppelklick aus dem Ordner mit `caps.txt` heraus.

## "Kein anonymer RW-Block ≥ 128 MiB im Prozess gefunden"

Citra läuft, aber kein Spiel ist geladen (FCRAM wird erst beim Boot eines Spiels alloziert).
Lade Pokémon Alpha Saphir in Citra, der Daemon versucht in 2-Sekunden-Intervallen einen Reconnect.

## "Kein Cap für N Orden in caps.txt definiert"

Deine `caps.txt` deckt nicht alle 9 Stufen (0–8) ab. Füge die fehlende Zeile hinzu. Bewusst harte Semantik — kein impliziter Fallback.

## EXP wird trotz Cap nicht eingefroren

Wahrscheinlich ist `BADGE_BYTE_OFFSET_3DS` oder `PARTY_BASE_3DS` für deine ORAS-Version falsch (oder noch ein Platzhalter — siehe `docs/OFFSETS.md`). Logs zeigen "0 Orden → Cap N" obwohl du z. B. 3 Orden hast → Badge-Offset falsch.

Issue mit Citra-Version und Spielausgabe (DE/EN/US/JP/AU/v1.0/v1.4) öffnen.
