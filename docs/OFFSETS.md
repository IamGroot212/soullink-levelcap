# Memory-Offsets — Research-Notes

> **Status: WIP**. Die hier dokumentierten Offsets sind teilweise Platzhalter und müssen vor v0.1.0
> mit einem laufenden Citra + ORAS-Save-State verifiziert werden.

## Ziel-Spiel

- **Pokémon Alpha Saphir v1.4** (PAL DE/EN, JP, US — alle teilen die gleichen FCRAM-Offsets, nur Title-ID differiert)
- **Emulator**: Citra (offizieller Build, EOL — stabile Memory-Layouts)
- **3DS-FCRAM**: 128 MiB ab 3DS-virtueller Adresse `0x08000000`

## R1: FCRAM-Mapping im Citra-Host-Prozess

**Approach (Linux)**: `/proc/<pid>/maps` parsen, größten anonymen RW-Block ≥ 128 MiB.
**Approach (Windows)**: `VirtualQueryEx` iterieren, Filter auf `MEM_COMMIT | MEM_PRIVATE | PAGE_READWRITE`, ≥ 128 MiB.

Implementation: [`src/emulator/citra.rs::find_fcram_base`](../src/emulator/citra.rs).

**Verifikation (TODO)**:
- [ ] Geldbetrag im Spiel merken (z. B. 1234 PD)
- [ ] In Cheat Engine / scanmem nach `1234` als 4-byte LE suchen in Citra-Prozess
- [ ] Geld ausgeben (z. B. Trank kaufen für 100 PD)
- [ ] Nach `1134` filtern → Adresse triangulieren
- [ ] Prüfen, ob die gefundene Adresse innerhalb `[fcram_base, fcram_base + 128 MiB]` liegt

## R2: Party-Struct-Offset

**Hypothese aus Brief**: Party-Base bei `0x08C861C8` (3DS-Adressraum) für ORAS v1.4.

**Quellen**:
- [`kcblack42/Citra-Tracker-v2`](https://github.com/kcblack42/Citra-Tracker-v2) — Python, enthält Party-Offsets
- Project Pokémon Wiki: ORAS Save-Structure

**Verifikation (TODO)**:
- [ ] Erste 4 Bytes ab `PARTY_BASE_3DS` sollten Header-Bytes sein, dann Species-ID des Starter-Pokémon (Geckarbor=252, Hydropi=258, Flemmli=255)
- [ ] Speichern in Citra, ein Pokémon umarrangieren, prüfen dass Reihenfolge im Memory passt

Aktueller Wert: [`src/game/party.rs::PARTY_BASE_3DS`](../src/game/party.rs).

## R3: Badge-Byte-Offset

**Im Save-File** (Quelle: [PKHeX](https://github.com/kwsch/PKHeX) — `PKHeX.Core/Saves/SAV6AO.cs`):
- ORAS hat 8 Hoenn-Liga-Orden als Bitflags in einem Byte (oder 16-bit Word) im Trainer-Profil
- Suche im Source nach `Badges` / `BadgeFlags`

**Im Live-Memory**:
- Save-File wird nur bei explizitem Speichern geschrieben → wir wollen den **Live-Zustand**
- Triangulation via scanmem / Cheat Engine:
  - [ ] Save-State mit 0 Orden laden → 1-Byte-Wert == `0` suchen (zu viele Treffer, weiter eingrenzen)
  - [ ] Save-State mit 3 Orden laden → unter den Treffern nach Wert `0b00000111 == 0x07` filtern
  - [ ] Save-State mit 8 Orden laden → Wert `0xFF` (alle 8 Bits)
  - [ ] Adresse triangulieren → in 3DS-virtuelle Adresse umrechnen (`host_addr - fcram_base + 0x08000000`)

Aktueller Wert ist **Platzhalter**: [`src/game/badges.rs::BADGE_BYTE_OFFSET_3DS`](../src/game/badges.rs).

## R4: Pokémon-Battle-Struct-Layout

Dekryptetes Layout im Party-Bereich (484 Bytes pro Slot, 6 Slots).

| Offset | Größe | Feld                  |
|-------:|------:|-----------------------|
| `0x08` |     2 | Species-ID (LE)       |
| `0x10` |     4 | EXP (u32 LE)          |
| `0xE0` |     1 | Current Level         |

Quelle: [PKHeX `PB6.cs`](https://github.com/kwsch/PKHeX/blob/master/PKHeX.Core/PKM/PB6.cs).

**Verifikation (TODO)**:
- [ ] Pokémon mit bekanntem Level (z. B. Lvl 5 Starter) lesen
- [ ] EXP-Wert gegen min_exp_for_level(5, growth_rate_of(252)) prüfen
- [ ] +1 Level erspielen → erneut lesen, Level-Byte sollte 6 sein

Aktuelle Offsets: [`src/game/party.rs`](../src/game/party.rs).

## R5: Wachstumsraten-Tabelle

**Quelle**: PokéAPI (`https://pokeapi.co/api/v2/pokemon-species/{1..721}`)

**Generierung**:
```bash
python3 scripts/fetch_growth_rates.py
```
Output: `data/species_growth.json`. Wird einmal generiert und committed.

`build.rs` kompiliert die JSON zur Compile-Zeit in eine `phf::Map<u16, GrowthRate>` (O(1)-Lookup, keine Heap-Allokation zur Laufzeit).

**Status**: `data/species_growth.json` enthält aktuell nur **7 Stub-Einträge** (die drei Starter und ein paar Test-Werte) — das Skript muss vor v0.1.0 ausgeführt werden.

Formeln in [`src/game/growth_rates.rs`](../src/game/growth_rates.rs) — gegen Bulbapedia-Tabellenwerte im Unit-Test verifiziert.

## Tools

- **Linux**: `scanmem` (`sudo apt install scanmem` oder `pacman -S scanmem`) — CLI-Memory-Scanner
- **Windows**: [Cheat Engine](https://www.cheatengine.org/)
- **Cross-Reference**: [PKHeX Source](https://github.com/kwsch/PKHeX) ist Goldstandard für Save-Structures
