# Brief für Claude-Code-Agent auf Windows

> Ziel-Reader: ein Claude-Code-Instance, die auf dem Windows-Hauptrechner des Users läuft und Citra direkt ansprechen kann. Dieser Brief ist self-contained — keine Vorkenntnis aus der Pi-Session nötig.

## Voraussetzungen (User installiert vorher)

1. **Git for Windows** → https://git-scm.com/download/win
2. **Rust (rustup)** → https://rustup.rs/ — Default-Installation (MSVC-Toolchain), benötigt VS Build Tools (Installer schlägt sie automatisch vor)
3. **GitHub CLI (`gh`)** → https://cli.github.com/ — danach `gh auth login`
4. **Citra** mit ORAS v1.4 — vermutlich schon installiert
5. **Pokémon Alpha Saphir Save-States** an drei Story-Punkten:
   - `0_badges.cst` — direkt nach Verlassen von Wurzelheim, vor erstem Orden
   - `3_badges.cst` — nach 3 Hoenn-Orden
   - `8_badges.cst` — nach allen 8 Orden, vor Liga
6. **Cheat Engine** (optional, für visuelle Verifikation) → https://www.cheatengine.org/

Repo-Setup:
```powershell
cd C:\Users\<dein-user>\Projects   # oder wo immer du willst
gh repo clone IamGroot212/soullink-levelcap
cd soullink-levelcap
cargo build --release
```

---

## Kontext für den Agent

Wir bauen `soullink-levelcap`, einen Rust-Daemon, der für eine 4-Spieler-Soullink-Challenge in **Pokémon Alpha Saphir** auf Citra die Party-Pokémon-EXP auf einen orden-basierten Level-Cap einfriert. Konzept: Cap steigt automatisch wenn der Spieler einen neuen Hoenn-Orden gewinnt — kein manueller Update-Workflow nötig.

Das Repo ist bereits weit gediehen:
- `caps.txt`-Parser fertig + getestet
- `ProcessMemory`-Trait + Windows-Backend (ReadProcessMemory/WriteProcessMemory) implementiert
- Citra-PID-Detection + FCRAM-Base-Heuristik (größter anonymer RW-Block ≥ 128 MiB)
- Daemon-Loop-Skelett, GitHub Actions CI grün auf Linux + Windows
- 23 Unit/Integration-Tests grün
- PokéAPI-Wachstumsraten für alle 721 Gen-1..6-Pokémon im Build embedded

**Was fehlt** (deine Mission): drei Memory-Layout-Bausteine, die nur mit laufendem Citra auf x86_64 verifiziert werden können. Die Pi-Hardware des Hauptentwicklers (ARM64) kann Citra nicht laufen lassen.

---

## Deine Aufgabe

### Phase A — Sanity-Check der Heuristiken (15 Min)

1. Citra mit ORAS starten, Save-State `8_badges.cst` laden (8 Orden, volle Party).
2. **Im Repo**:
   ```powershell
   cargo test --test memory_layout_test -- --ignored --nocapture
   ```
3. Erwartetes Verhalten **wenn FCRAM-Detection + Adressen OK sind**:
   - `[test] Orden: 8` (oder zumindest ein Wert 1..=8)
   - Pro Slot eine plausible Species-ID (1..=721) und Level (1..=100)
4. Wahrscheinlich-Ergebnis **beim ersten Lauf**: Species/Level sind Müll, weil die Party verschlüsselt ist (siehe Phase C). FCRAM-Detection und Badge-Read sollten aber gehen.

**Output für den Repo-Owner**: Schreib in `docs/OFFSETS.md` unter "Phase A Ergebnis" auf:
- Citra-Version (`citra --version` oder GUI → Hilfe)
- FCRAM-Base-Adresse (aus dem Test-Output)
- Badge-Read-Wert (sollte 8 sein) — wenn nicht 8: Phase B starten

### Phase B — Badge-Offset triangulieren (falls Phase A != 8 zurückgibt)

`citra-updater.py:809` sagt `badgeaddress = 0x8C6DDD4` für ORAS. Das ist unser aktueller Wert. Wenn der falsch ist:

1. Cheat Engine starten, Citra-Prozess attachen.
2. Save-State `0_badges.cst` laden → "First Scan", Value Type `Byte`, Value `0`. Riesige Treffermenge.
3. Save-State `1_badges.cst` (falls vorhanden) oder `3_badges.cst` laden → "Next Scan", Value `1` bzw. `7`.
4. Idealfall: 1–10 Adressen übrig. Save-State `8_badges.cst` → Next Scan auf `255`. Sollte 1 Treffer sein.
5. Adresse notieren (Host-VA), umrechnen in 3DS-Adresse:
   ```
   addr_3ds = host_va - fcram_base + 0x08000000
   ```
6. **Wenn neuer Wert ≠ 0x08C6DDD4**: update `src/game/badges.rs::BADGE_BYTE_OFFSET_3DS` und dokumentiere im OFFSETS.md.

### Phase C — Gen-6 Encryption-Layer implementieren (Kernarbeit)

Das ist der eigentliche Brocken. Aktuell liest unser Code rohen XOR-Müll aus der Party.

**Referenz**: `citra-updater.py` Zeile 124-181 aus https://github.com/kcblack42/Citra-Tracker-v2 — Python-Implementation, exakt nachzubauen.

#### Algorithmus

Ein Party-Slot ist 484 Bytes. Layout:
```
[0..8]     Header inkl. PV (Personality Value, u32 LE bei 0..4)
[8..232]   Encrypted PKM-Daten: 4 Blöcke à 56 Bytes (BLOCK_SIZE), reihenfolge geshufflet
[232..344] Reserved (Battle-Status etc., wird vom Tracker übersprungen)
[344..366] Encrypted Stats (Level + HP + 5 Stats = 22 Bytes)
[366..484] Reserved
```

**Decrypt-Algorithmus**:
1. `pv = u32_le(slot[0..4])`
2. `sv = ((pv >> 13) & 0x1F) % 24` — Shuffle Value (0..23)
3. LCG-Stream `seed` initialisiert mit `pv`, dann pro Iteration:
   ```
   seed = (seed * 0x41C64E6D + 0x00006073) mod 2^32
   ```
4. Über `slot[8..232]` in 2-Byte-Schritten: für jedes Bytepaar `(b0, b1)`:
   - `seed = next(seed)`
   - `b0 ^= (seed >> 16) & 0xFF`
   - `b1 ^= (seed >> 24) & 0xFF`
5. Block-Unshuffle: das `[8..232]`-Array besteht aus 4×56-Byte-Blöcken in Reihenfolge A',B',C',D'. Die "logischen" Blöcke A/B/C/D ergeben sich aus dem 4×24-Lookup-Table aus `citra-updater.py` Zeile 152-156 indiziert mit `sv`.
6. Für die Stats `slot[344..366]` (22 Bytes): **LCG fortsetzen** (nicht neu seeden) — also zuerst die `[232..344]`-Lücke durchadvancen (112 Bytes = 56 LCG-Steps), dann die 22 Stats-Bytes XOR-decrypten.
   - **Achtung**: Im Tracker-Code wird ein konkateniertes Array `data = party_data + stats_data` mit Länge 254 dekryptiert — d.h. die LCG wird kontinuierlich von Byte 0 bis 254 angewandt, ohne 112-Byte-Lücke. **Verifiziere welche Variante in der ORAS-Realität gilt** (Tracker-Code lässt vermuten: konkateniert; sprich, einfach `slot[0..232] + slot[344..366]` lesen und als 254-Byte-Block dekryptieren).
7. Nach Decrypt: 
   - Species: `u16_le(plain[0x08..0x0A])` (im Block A — nach Unshuffle)
   - EXP: `u32_le(plain[0x10..0x14])` (im Block A, +8)
   - Level (current): `plain[0xEC]` (= stats[4], also `stats_data[4]` decrypted)

**Encrypt-Algorithmus**: derselbe Algorithmus rückwärts. Da XOR symmetrisch ist und Shuffle reversibel: 
1. Block-Shuffle (umgekehrte Permutation aus dem Lookup)
2. Bytes in Slot zurückschreiben
3. Mit gleichem LCG-Stream XOR-en (idempotent — XOR derselben Sequence stellt das Original wieder her)

#### Implementation

Lege an: `src/game/decrypt.rs`. Public API:
```rust
pub fn decrypt_slot(slot: &[u8; 254]) -> [u8; 254];  // ent-XOR'd + un-shuffled
pub fn encrypt_slot(plain: &[u8; 254]) -> [u8; 254]; // re-shuffled + XOR'd
```

Wichtig: 
- Die Funktionen nehmen das **konkatenierte 254-Byte-Slice** (slot[0..232] + slot[344..366]) — nicht den vollen 484-Byte-Slot. Reads/Writes von / nach Citra-Memory machen den Aufruf in zwei Stücken.
- Header (slot[0..8]) bleibt **unverändert**, auch nach Decrypt.

#### Tests (in `tests/decrypt_test.rs`)

1. **Round-trip**: `encrypt(decrypt(x)) == x` für 10 zufällige Slots (Property-based oder hardcoded).
2. **Bekanntes Pokémon**: Greife mit Cheat Engine im laufenden Citra einen Party-Slot raw aus (496 Bytes ab `0x08CF727C`), speichere Hex-Dump im Test-Fixture. Decrypt sollte konsistent ergeben:
   - Erste Bytes (Species) plausibel (1..=721)
   - Level zwischen 1 und 100
   - EXP konsistent mit Level + Growth-Rate (kreuzcheck mit `min_exp_for_level`)

#### `PartyPokemon::read` umbauen

Aktuell (`src/game/party.rs`):
```rust
let species = mem.read_u16_le(base + OFF_SPECIES_ENCRYPTED)?;  // Müll
```

Neu (skizziert):
```rust
let blob_a = mem.read_bytes(base, 232)?;
let blob_b = mem.read_bytes(base + 344, 22)?;
let mut concat = [0u8; 254];
concat[..232].copy_from_slice(&blob_a);
concat[232..].copy_from_slice(&blob_b);
let plain = decrypt_slot(&concat);

let species = u16::from_le_bytes([plain[0x08], plain[0x09]]);
let exp     = u32::from_le_bytes([plain[0x10], plain[0x11], plain[0x12], plain[0x13]]);
let level   = plain[0xEC];
```

#### `PartyPokemon::write_exp` umbauen

EXP-only Write: 
1. `read` macht decrypt → wir haben `plain`
2. Plain modifizieren: `plain[0x10..0x14] = new_exp.to_le_bytes()`
3. `encrypt_slot(&plain)` → `concat_enc`
4. Schreibe `concat_enc[..232]` zurück nach `base`, `concat_enc[232..]` nach `base + 344`

**Achtung-Stolperfalle**: PV (Bytes 0..4) wird nicht XOR'd, aber Decrypt nutzt PV für die Stream-Generation. Solange wir die ersten 4 Bytes nie ändern, ist Re-Encryption deterministisch.

### Phase D — End-to-End-Test

Nachdem Phase B + C durch sind:

1. `cargo build --release`
2. Citra läuft mit ORAS, Save-State mit z.B. 2 Orden und einer Lvl-26-Party laden
3. `caps.example.txt` zu `caps.txt` umbenennen
4. `.\target\release\soullink-levelcap.exe`
5. Erwartete Log-Ausgabe:
   ```
   [soullink-levelcap v0.1.0] gestartet
   [INFO] Caps geladen aus: caps.txt
   ...
   [soullink-levelcap] Citra gefunden (PID: NNNN, FCRAM @ 0xXXXX)
   [INFO] Orden-Anzahl: 2 → Cap: Level 24
   [CAP] Slot 0 (...) auf Lvl 26 eingefroren (EXP NNN → MMM)
   ```
6. In Citra weiterspielen: Pokémon kämpft, gewinnt EXP, **darf aber nicht über Lvl 26 steigen** (Daemon friert ein).
7. In Citra einen 3. Orden gewinnen → Daemon-Log soll `Orden-Anzahl: 3 → Cap: Level 29` zeigen, neue Level-Ups bis 29 funktionieren wieder.

### Phase E — Commit + Push

Wenn alle Phases grün:
```powershell
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --all-targets

git add .
git commit -m "feat: gen-6 encryption layer + verified ORAS offsets"
git push
```

Wenn etwas hängt: einen WIP-Branch pushen und PR öffnen mit einer kurzen Notiz, wo's klemmt. Der Pi-Entwickler übernimmt von dort.

---

## Was du NICHT tun sollst

- **Keine Refactorings** außerhalb der Phase-C-Files. Der Daemon-Loop, Memory-Backend und CLI sind fertig — nicht anfassen.
- **Keine neuen Features**. Auch wenn dir z.B. Auto-Update oder GUI sinnvoll erscheinen — nicht jetzt, Brief sagt explizit "einmal Setup, never touch".
- **Keine kompletten Rewrites** des Encryption-Layers in einem fancy Stil. Halt dich an die Python-Vorlage, das spart Verifikationsaufwand.
- **caps.txt nicht committen** — die Datei ist `.gitignore`d. Nur `caps.example.txt`.
- **Citra-Save-States nicht ins Repo committen** — die sind potenziell rechtlich heikel und nur lokal nützlich.

## Bei Unklarheit / Blockern

Schreib eine Notiz in `docs/OFFSETS.md` unter `## Open Questions for Pi-Side` mit konkreten Fragen. Push der Notiz reicht, der Pi-Entwickler sieht es beim nächsten Pull.

Viel Erfolg.
