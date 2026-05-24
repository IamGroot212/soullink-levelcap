# soullink-levelcap

Level-Cap-Daemon für eine **Soullink-Challenge** in **Pokémon Alpha Saphir** auf dem **Citra-Emulator**.

Liest die Anzahl der Hoenn-Orden des Spielers direkt aus dem Prozessspeicher von Citra
und friert die EXP der Party-Pokémon auf einem aus `caps.txt` abgeleiteten Level-Cap ein.
Wenn ein Spieler den nächsten Orden gewinnt, steigt sein Cap automatisch — kein manuelles
Pflegen einer geteilten Cap-Datei mehr nötig.

> Status: **WIP**. Phase 1 (caps.txt-Parser) ist funktionsfähig. Die Memory-Offsets (R1-R5) brauchen
> noch eine Recherche-Session mit laufendem Citra + Save-States — siehe [`docs/OFFSETS.md`](docs/OFFSETS.md).

## Quick Start (Mitspieler)

1. Lade von der [Releases-Seite](https://github.com/IamGroot212/soullink-levelcap/releases) die passende Binary:
   - Windows: `soullink-levelcap-windows-x64.exe`
   - Linux / Steam Deck: `soullink-levelcap-linux-x64`
2. Lade zusätzlich `caps.example.txt` und benenne sie in `caps.txt` um.
3. Starte Citra mit Pokémon Alpha Saphir.
4. Starte die Binary (Doppelklick auf Windows, Terminal auf Linux).
5. Konsolenfenster offen lassen — der Daemon hält die Levelcaps während ihr spielt.

Ausführlich: [`docs/SETUP.md`](docs/SETUP.md).

## Dev (Raspberry Pi 5 / Linux ARM64)

```bash
cargo check
cargo test
cargo build --release
```

Cross-Compile zu Windows/Linux x86_64 läuft via GitHub Actions bei Tag-Push.

## Lizenz

Dual-licensed under MIT oder Apache-2.0 — wähle was dir besser passt.
