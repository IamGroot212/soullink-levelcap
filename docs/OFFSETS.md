# Memory-Offsets — Research-Notes

> **Status: WIP**. Die hier dokumentierten Offsets sind teilweise Platzhalter und müssen vor v0.1.0
> mit einem laufenden Citra + ORAS-Save-State verifiziert werden.

## Phase A Ergebnis (2026-05-25)

**Setup**:
- Citra-Version: `citra-windows-msvc-20240303-0ff3440` (Build 0ff3440, 2024-03-03)
- ROM: `AS_random2.cxi` (randomisierte ORAS-CXI, EUR-Region, Title-ID `000400000011C500`)
- Save vorbereitet via PKHeX: Badges auf `0xFF` (alle 8), Party 4×Mons

**Befund FCRAM-Detection**:
- Citra-Prozess hat **eine** RW-Region ≥ 64 MiB: **256 MiB** großer Buffer
  (= New3DS Extended FCRAM, statt 128 MiB Old3DS)
- Heuristik `find_fcram_base` picked diese korrekt

**Befund: citra-updater.py-Offsets stimmen NICHT für unseren Build**:
- Erwartet (citra-updater.py): Badges `0x08C6DDD4`, Party `0x08CF727C` (Diff `0x894A8`)
- Tatsächlich (per Triangulation): Badges `0x0F48EE14`, Party `0x0F49E50C` (Diff `0xF6F8`)
- Die ~100 MiB Verschiebung kommt vermutlich aus Citra-Version-Differenzen oder dem
  randomisierten CXI-Layout. **Beide Werte aktualisiert** in `src/game/badges.rs` + `src/game/party.rs`.

**Triangulations-Methodik** (siehe `tests/fcram_diagnostic.rs`):
1. Save-File `main` mit PowerShell ausgelesen
2. Misc-Block @ file-offset `0x4200`: Money(u32 LE=11012)+Badges(u8=0xFF)+padding
   → Signatur `04 2B 00 00 FF 00 00 00` (8 bytes, eindeutig)
3. Scan in 256 MiB FCRAM: **1 Treffer** → Badge-Adresse `0x0F48EE14`
4. Party-PVs aus `main` @ file-offset `0x14200` ausgelesen (4 von 6 Slots belegt)
5. Scan PV0+PV1+PV2: cross-korrelation an `+0/+484/+968` → **eindeutig** Party-Base `0x0F49E50C`

**Test-Output `memory_layout_test`**:
```
[test] Orden: 8   <-- ✓ Badge-Read funktioniert
thread panicked: Species-ID 57638 außerhalb Gen1-6   <-- erwartet, Decrypt fehlt (Phase C)
```

**Offen / Risiken**:
- **Stability across Citra-Restart unverifiziert** — der gefundene Offset könnte sich
  ändern wenn Citra anders allocate (Save-State-Load vs. Continue, oder zwischen Sessions)
- Pi-Side sollte ggf. eine **Runtime-Triangulation** via Misc-Signatur einbauen,
  statt hardcoded Offsets zu verwenden (siehe `tests/fcram_diagnostic.rs`)

## Phase C Ergebnis (2026-05-25)

**Implementation**:
- `src/game/decrypt.rs`: `decrypt_slot`/`encrypt_slot` für 254-Byte-Konkat
  (slot[0..232] + slot[344..366]) per LCG-XOR + 4-Block-Shuffle.
- Algorithmus 1:1 nach `citra-updater.py:100-181` (kcblack42/Citra-Tracker-v2).
- Roundtrip-Tests grün, 5 Unit-Tests.

**Was funktioniert**: Block-A-Decrypt (Species + EXP) für Slot 0.
Slot 0 in unserem Test: `species=631 lvl=64 exp=262144` — EXP = 64³ = Medium-Fast
Lvl 64, in sich konsistent. OT-Name "ANGELA" (player name) korrekt aus Block D.

**Was NICHT funktioniert**: Stats-Decrypt (slot[344..366]) liefert für `plain[0xEC]`
(Level) keinen plausiblen Wert — getestet mit drei LCG-Varianten:
1. **Reset zwischen Blocks und Stats** (Citra-Tracker default): liefert 171/165/172/153
2. **LCG continuous über alle 254 Bytes**: liefert 224/56/93/227
3. **Continuous + Skip-Gap (56 Advances)**: liefert 235/194/195/50

Keine Variante gibt für alle Slots gültige Level. Production-Workaround:
**Level wird aus EXP + Growth-Rate berechnet** (`growth_rates::level_from_exp`)
statt aus `plain[0xEC]` gelesen.

**Slot-Filter in `PartyPokemon::read`**:
- `PV == 0` → leer (skip)
- `Sanity != 0` (bytes 4-5) → korrumpiert / non-PB6-Format (skip)
- `species == 0 || species > 721` → außerhalb Gen-1-6 (skip)
- `exp > 1_640_000` → über Lvl 100 in jedem Growth-Rate (skip)

In unserer User-Party haben Slots 1-3 alle `sanity != 0` (Hex-Werte 0xDEE0, 0x863A,
0xB6A8) bereits im Backup vor PKHeX-Edit. Vermutung: Soullink-Importe via
randomisiertem ROM speichern Pokemon in non-Standard-Format. Daemon ignoriert
diese automatisch.

## Stride-Bugfix (Phase C+, 2026-05-26)

**Bug entdeckt**: Anfangs nutzten wir Stride 484 (citra-updater-Wert) ab
`PARTY_BASE_3DS = 0x0F49E50C`. Nur Slot 0 hatte plausible Daten; Slots 1-3 wurden
als "korrumpiert" (sanity != 0, garbage species) verworfen. Verifikation gegen
PKHeX-Export einer Party-Pokemon als `.pk6` zeigte: PKHeX sieht **alle 3 Mons als
gültig** an. Mein Decrypt produzierte Müll für Slots 1+.

**Root Cause**: In RAM existieren **zwei Party-Kopien**:

- **0x0F49E50C, stride 260** (PB6 `SIZE_6PARTY` = Save-Block-Kopie in RAM)
- **0x0F5182BC, stride 484** (Citra-Tracker Battle-Party-Layout mit
  112-Byte-Gap für Battle-Status)

Wir hatten Base von Kopie 1 mit Stride von Kopie 2 gemischt. Slot 0 startet bei
+0 in beiden Layouts, deshalb funktionierte slot 0 zufällig.

**Triangulation** (3 EC-Werte aus .sav gegen RAM cross-korreliert):
- EC0=0xFC645FE6 (Heatmor)
- EC1=0x73A26FC1 (Skunkapuh)
- EC2=0x3F76F4BD (Lvl 72 Mon, Slow growth)

Beide RAM-Triangulationen geben dieselben 3 Pokemon — bestätigt dass beide
Layouts existieren. Wir nutzen die 260-Stride-Variante (= identisch zu
PKHeX-Sav-Format, kein 112-Byte-Gap, Stats unverschoben bei +232).

**Fix in src/game/party.rs**: `POKEMON_SIZE = 260`, `STATS_START = 232`.

**Verifikation**: `cargo test --test memory_layout_test -- --ignored --nocapture`
zeigt jetzt alle 3 Party-Mons:
```
[test] Slot 0: species=631 lvl=64 exp=262144   (Heatmor)
[test] Slot 1: species=434 lvl=64 exp=262144   (Skunkapuh)
[test] Slot 2: species=604 lvl=72 exp=466560   (Lvl-72, Slow growth)
```

**Offen**: Welche der beiden RAM-Kopien wird vom Spiel als "authoritative" für
EXP-Updates genutzt? Vermutung: beide werden in Sync gehalten; daemon schreibt
in 260-Stride und das gilt nach der nächsten Tick auch im 484-Stride. Live im
Battle zu testen.

## Phase D Ergebnis (2026-05-25)

**Daemon-Start funktioniert end-to-end**:
```
[soullink-levelcap v0.1.0] gestartet
[INFO] Caps geladen aus: caps.txt
[INFO]   0 Orden → Lvl 15  ...  8 Orden → Lvl 48
[soullink-levelcap] Suche Citra-Prozess...
[soullink-levelcap] Citra gefunden (PID: 46448, FCRAM @ 0x2150d424000)
[INFO] Orden-Anzahl: 8 → Cap: Level 48
```

**Stabilitäts-Check**: FCRAM-Host-Adresse wechselte zwischen Citra-Sessions
(0x1E680005000 → 0x2150d424000) — unsere `find_fcram_base` Heuristik fand den
neuen Host-Pointer korrekt; die 3DS-virtuellen Offsets in `BADGE_BYTE_OFFSET_3DS`
und `PARTY_BASE_3DS` blieben gültig. Layout-Position innerhalb FCRAM ist also
stable across Citra-Restart.

**Live Write/Read-Roundtrip** (`tests/write_exp_live.rs`):
```
Ausgangs:  Slot 0 species=631 lvl=64 exp=262144
Schreibe:  EXP=274489
Read-back: species=631 lvl=64 exp=274489    ✓ persistent in RAM
Restore:   EXP=262144                       ✓
```
`PartyPokemon::write_exp` (decrypt → modify EXP → encrypt → write blocks+stats)
funktioniert in live Citra-RAM. Species bleibt unverändert, EXP wird präzise
gesetzt, beim nächsten Read kommt der neue Wert raus.

**Was NICHT live getestet wurde**:
- Cap-Freeze-Logik beim aktiven Spielen (Pokemon kämpft, gewinnt EXP, Daemon
  schreibt zurück). User-Party Slot 0 = Lvl 64 ist über jedem definierten Cap
  (max Cap 48 bei 8 Orden), `Overshoot-Policy` friert beim aktuellen Level ein
  statt down-zu-leveln. Demonstrable via Battle in Citra, aber nicht automatisierbar.
- Cap-Update beim Gewinn neuer Orden — User hat schon alle 8.

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
