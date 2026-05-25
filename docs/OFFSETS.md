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

**Aktueller Wert**: `PARTY_BASE_3DS = 0x08CF727C` (aus Citra-Tracker-v2 `citra-updater.py:799`).

> Brief-Hypothese war `0x08C861C8` — abgelöst durch den im Citra-Tracker tatsächlich
> verwendeten Wert, der erwiesenermaßen funktioniert. Trotzdem noch unverifiziert
> für unseren Build — Citra-Versions-Unterschiede oder ORAS v1.0 vs v1.4 könnten
> hier reinspielen.

**Quellen**:
- [`kcblack42/Citra-Tracker-v2`](https://github.com/kcblack42/Citra-Tracker-v2) — Python, enthält Party-Offsets pro Spiel-Branch
- Project Pokémon Wiki: ORAS Save-Structure
- [PKHeX `PB6.cs`](https://github.com/kwsch/PKHeX/blob/master/PKHeX.Core/PKM/PB6.cs) — Gen-6 Battle-Format

**Verifikation (TODO)**:
- [ ] Die ersten 4 Bytes ab `PARTY_BASE_3DS` sind die **PV** (Personality Value) des ersten Pokémon — random aber konstant für einen einzelnen Mon. Falls 0x00000000 → Slot leer oder Offset falsch.
- [ ] Slot-Stride ist 484 Bytes (`SLOT_OFFSET`).
- [ ] Speichern in Citra, ein Pokémon umarrangieren, prüfen dass Reihenfolge im Memory passt.

## Gen-6 Encryption — kritischer Blocker

**Problem**: Die Party-Daten in ORAS sind **verschlüsselt** im RAM gespeichert (gleicher Algorithmus wie im Save-File).
Der Citra-Tracker liest sie und ruft `decrypt_data()` auf bevor er Felder zugreift.

**Algorithmus** (aus `citra-updater.py:124-181`):
1. `pv = u32_le(slot[0..4])` — Personality Value
2. `sv = ((pv >> 13) & 0x1F) % 24` — Shuffle Value (0..23)
3. `slot[8..232]` = 4 Blöcke à 56 Bytes, XOR-verschlüsselt mit einem LCG-Stream:
   - LCG: `seed = (seed * 0x41C64E6D + 0x00006073) mod 2^32`
   - Pro 2 Bytes: XOR mit Bytes (seed>>16) und (seed>>24)
   - LCG-Seed = `pv`
4. Nach Decrypt: Block-Reihenfolge per `block_position[block][sv]`-Lookup unshuffled
5. `slot[232..]` (Battle-Stats) ist mit derselben fortgesetzten LCG-Sequence XOR-verschlüsselt

**Konsequenz**: Unser aktueller Code (`PartyPokemon::read`) liest und schreibt rohen XOR-Müll. **Wird beim ersten Live-Test offensichtlich falsche Species-IDs zurückgeben**. Vor Funktion-OK:

- [ ] LCG-Implementierung in Rust (`src/game/decrypt.rs`)
- [ ] Block-Shuffle-Lookup
- [ ] `PartyPokemon::read` decrypted die 254 relevanten Bytes vor dem Feld-Zugriff
- [ ] `PartyPokemon::write_exp` re-encrypted vor dem WriteProcessMemory (gleicher Algo, da symmetrisch)
- [ ] Tests mit konstanten PV+Plaintext gegen Citra-Tracker-Output verifizieren

Bis dahin: Test-Run wird nur den Citra-Prozess finden und FCRAM-Base loggen — kein EXP-Cap.

## R3: Badge-Byte-Offset

**Aktueller Wert**: `BADGE_BYTE_OFFSET_3DS = 0x08C6DDD4` (aus Citra-Tracker-v2 `citra-updater.py:809`).

**Im Save-File** (Quelle: [PKHeX](https://github.com/kwsch/PKHeX) — `PKHeX.Core/Saves/SAV6AO.cs`):
- ORAS hat 8 Hoenn-Liga-Orden als Bitflags in einem Byte (oder 16-bit Word) im Trainer-Profil
- Suche im Source nach `Badges` / `BadgeFlags`

**Verifikation (TODO)**:
- [ ] Save-State mit 0 Orden laden → Byte bei 0x08C6DDD4 sollte `0x00` sein
- [ ] Save-State mit 3 Orden laden → `0x07` (Bits 0,1,2 gesetzt) ODER ein Word `0x0007` mit umgekehrtem Bit-Layout
- [ ] Save-State mit 8 Orden laden → `0xFF`
- [ ] Falls Adresse falsch: scanmem / Cheat Engine via Triangulation (siehe Workflow unten)

**Tools/Workflow**:
- [ ] Falls Citra-Tracker-Wert nicht passt: Save-State #1 (0 Orden) → 1-Byte-Wert == `0` suchen
- [ ] Filter via Save-State #2 (3 Orden) → Wert `0x07`
- [ ] Adresse triangulieren → in 3DS-virtuelle Adresse umrechnen (`host_addr - fcram_base + 0x08000000`)

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
